// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::index::fast_fields_helper::{
    build_arrow_schema, ords_to_bytes_array, ords_to_string_array, FFHelper, FFType, WhichFastField,
};
use crate::index::reader::index::{MultiSegmentSearchResults, SearchIndexScore};
use crate::postgres::heap::VisibilityChecker;
use arrow_array::builder::BooleanBuilder;
use arrow_array::{Array, ArrayRef, BooleanArray, Float32Array, RecordBatch, UInt64Array};
use arrow_schema::SchemaRef;
use datafusion::arrow::compute;
use std::sync::Arc;
use tantivy::{DocAddress, DocId, Score, SegmentOrdinal};

/// The maximum number of rows to batch materialize in memory while iterating over a result set.
///
/// Setting this value larger reduces the cost of our joins to the term dictionary by allowing more
/// terms to be looked up at a time, but increases our memory usage by forcing more column values to
/// be held in memory at a time.
const MAX_BATCH_SIZE: usize = 128_000;

/// The maximum number of rows to batch when all string/byte columns are
/// deferred during late materialization. Aligned with DataFusion's default
/// batch size, since we are not fetching string dictionaries during the scan phase.
const DEFERRED_BATCH_SIZE: usize = 8_192;

/// Compact `ids` and `scores` in-place based on a boolean mask.
fn compact_with_mask(
    ids: &mut Vec<DocId>,
    scores: &mut Vec<Score>,
    memoized_columns: &mut Vec<Option<ArrayRef>>,
    mask: &BooleanArray,
) {
    if mask.false_count() == 0 && mask.null_count() == 0 {
        return;
    }

    // Compact ids and scores.
    let mut write_idx = 0;
    for (read_idx, valid) in mask.iter().enumerate() {
        if valid == Some(true) {
            if read_idx != write_idx {
                ids[write_idx] = ids[read_idx];
                scores[write_idx] = scores[read_idx];
            }
            write_idx += 1;
        }
    }
    ids.truncate(write_idx);
    scores.truncate(write_idx);

    // Compact memoized columns
    for opt_col in memoized_columns {
        if let Some(col) = opt_col {
            *opt_col = Some(compute::filter(col, mask).expect("Filter failed"));
        }
    }
}

/// Ensure `memoized_columns[ff_index]` is populated, fetching from the fast field helper if needed.
///
/// Skips synthetic fields (Ctid, DeferredCtid, Score, TableOid, Junk) which don't have
/// real fast-field backends. Pre-filters should never reference these indices, but this
/// guard prevents silent corruption if they do.
fn ensure_column_fetched(
    memoized_columns: &mut [Option<ArrayRef>],
    which_fast_fields: &[WhichFastField],
    ffhelper: &FFHelper,
    segment_ord: SegmentOrdinal,
    ff_index: usize,
    ids: &[DocId],
) {
    if memoized_columns[ff_index].is_some() {
        return;
    }
    match &which_fast_fields[ff_index] {
        WhichFastField::Named(_, _) | WhichFastField::Deferred(_, _) => {
            memoized_columns[ff_index] = Some(
                ffhelper
                    .column(segment_ord, ff_index)
                    .fetch_values_or_ords_to_arrow(ids),
            );
        }
        WhichFastField::Ctid
        | WhichFastField::Score
        | WhichFastField::TableOid
        | WhichFastField::Junk(_) => {}
        WhichFastField::DeferredCtid(alias) => {
            panic!(
                "pre-filter referenced DeferredCtid column '{alias}' at index {ff_index} \
                 — this indicates a planning bug"
            );
        }
    }
}

/// A batch of visible tuples and their fast field values.
#[derive(Default)]
pub struct Batch {
    /// An iterator of ids which have been consumed from the underlying `SearchResults`
    /// iterator as a batch.
    pub ids: Vec<(SearchIndexScore, DocAddress)>,

    /// The current batch of fast field values, indexed by FFIndex, then by row.
    /// This uses Arrow arrays for efficient columnar storage.
    pub fields: Vec<Option<ArrayRef>>,
}

impl Batch {
    /// Convert the batch to an Arrow `RecordBatch`.
    #[allow(dead_code)]
    pub fn to_record_batch(&self, schema: &SchemaRef) -> RecordBatch {
        let columns: Vec<ArrayRef> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                field.clone().unwrap_or_else(|| {
                    let data_type = schema.field(i).data_type();
                    arrow_array::new_null_array(data_type, self.ids.len())
                })
            })
            .collect();

        RecordBatch::try_new(schema.clone(), columns).expect("Failed to create RecordBatch")
    }
}

/// A scanner that iterates over search results in batches, fetching fast fields.
///
/// This scanner consumes [`WhichFastField`] column selectors, which represent "widened" Postgres types
/// (e.g. storage types), and produces Arrow arrays corresponding to those widened types.
pub struct Scanner {
    search_results: MultiSegmentSearchResults,
    batch_size: usize,
    which_fast_fields: Vec<WhichFastField>,
    table_oid: u32,
    maybe_ctids: Vec<Option<u64>>,
    visibility_results: Vec<Option<u64>>,
    prefetched: Option<Batch>,
    /// When true, visibility checking is deferred to VisibilityFilterExec.
    /// Packed DocAddresses are emitted instead of real ctids.
    defer_visibility: bool,
    /// Rows entering the pre-materialization filter stage (after visibility).
    pub pre_filter_rows_scanned: usize,
    /// Rows removed by pre-materialization filters.
    pub pre_filter_rows_pruned: usize,
}

impl Scanner {
    /// Create a new scanner for the given search results.
    ///
    /// `batch_size_hint` is an optional hint for the batch size. It will be clamped to
    /// `MAX_BATCH_SIZE`.
    ///
    /// Note: `batch_size_hint` should only be provided when we have a very good idea of how
    /// many total rows will be requested (e.g. `LIMIT` queries where `ColumnarExecState`
    /// is the top-level node). In all other cases (e.g. `JoinScan`, `TableProvider`), it
    /// should be `None` to allow the default batch size to be used, which is optimized for
    /// columnar string lookups.
    pub fn new(
        search_results: MultiSegmentSearchResults,
        batch_size_hint: Option<usize>,
        which_fast_fields: Vec<WhichFastField>,
        table_oid: u32,
    ) -> Self {
        let all_strings_deferred = !which_fast_fields.iter().any(|wff| {
            matches!(
                wff,
                WhichFastField::Named(_, field_type) if matches!(
                    field_type.arrow_data_type(),
                    arrow_schema::DataType::Utf8View
                        | arrow_schema::DataType::BinaryView
                        | arrow_schema::DataType::LargeUtf8
                        | arrow_schema::DataType::LargeBinary
                )
            )
        });

        let default_batch_size = if all_strings_deferred {
            DEFERRED_BATCH_SIZE
        } else {
            MAX_BATCH_SIZE
        };

        let batch_size = batch_size_hint
            .unwrap_or(default_batch_size)
            .min(default_batch_size);

        let defer_visibility = which_fast_fields
            .iter()
            .any(|wff| matches!(wff, WhichFastField::DeferredCtid(_)));

        Self {
            search_results,
            batch_size,
            which_fast_fields,
            table_oid,
            maybe_ctids: Vec::new(),
            visibility_results: Vec::new(),
            prefetched: None,
            defer_visibility,
            pre_filter_rows_scanned: 0,
            pre_filter_rows_pruned: 0,
        }
    }

    /// Returns the Arrow schema for this scanner.
    #[allow(dead_code)]
    pub fn schema(&self) -> SchemaRef {
        build_arrow_schema(&self.which_fast_fields)
    }

    /// Override the batch size. Clamped to `MAX_BATCH_SIZE`.
    pub(crate) fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size.min(MAX_BATCH_SIZE);
    }

    /// Returns the estimated number of rows that will be produced by this scanner.
    pub fn estimated_rows(&self) -> u64 {
        self.search_results.estimated_doc_count()
    }

    fn try_get_batch_ids(&mut self) -> Option<(SegmentOrdinal, Vec<Score>, Vec<DocId>)> {
        // Collect a batch of ids for a single segment.
        loop {
            let scorer_iter = self.search_results.current_segment()?;
            let segment_ord = scorer_iter.segment_ord();

            // Collect a batch of ids/scores for this segment.
            let mut scores = Vec::with_capacity(self.batch_size);
            let mut ids = Vec::with_capacity(self.batch_size);
            while ids.len() < self.batch_size {
                let Some((score, id)) = scorer_iter.next() else {
                    // No more results for the current segment: remove it.
                    self.search_results.current_segment_pop();
                    break;
                };

                // TODO: Further decompose `ScorerIter` to avoid (re)constructing a `DocAddress`.
                debug_assert_eq!(id.segment_ord, segment_ord);
                scores.push(score);
                ids.push(id.doc_id);
            }

            if ids.is_empty() {
                // This segment was completely empty: move to the next.
                continue;
            }

            return Some((segment_ord, scores, ids));
        }
    }

    /// Fetch the next batch of results, applying visibility checks and
    /// pre-materialization filters.
    ///
    /// `pre_filters` are applied after visibility checks but *before* column
    /// materialization, allowing string-column filters (including dynamically
    /// generated lexicographical thresholds) to operate natively on cheap
    /// term ordinals rather than requiring expensive dictionary lookups.
    pub fn next(
        &mut self,
        ffhelper: &FFHelper,
        visibility: &mut VisibilityChecker,
        pre_filters: Option<&crate::scan::pre_filter::PreFilters<'_>>,
    ) -> Option<Batch> {
        if let Some(batch) = self.prefetched.take() {
            return Some(batch);
        }
        pgrx::check_for_interrupts!();
        let (segment_ord, mut scores, mut ids) = self.try_get_batch_ids()?;

        // Memoize fetched columns to avoid redundant fetches.
        // - Numeric columns: stores the values directly.
        // - Text/Bytes columns: stores the term ordinals (UInt64Array).
        // This allows pre-filters to operate on the ordinals cheaply, and we only materialize
        // the string/bytes values at the end when constructing the Batch.
        // We must compact these arrays whenever we filter rows (pre-filtering or visibility)
        // to keep them aligned with `ids`.
        let mut memoized_columns: Vec<Option<ArrayRef>> = vec![None; self.which_fast_fields.len()];

        // Apply pre-materialization filters before visibility checks (which require the ctid), and
        // before dictionary lookups.
        if let Some(pre_filters) = pre_filters {
            let before = ids.len();
            for pre_filter in pre_filters.filters {
                if ids.is_empty() {
                    break;
                }
                for &ff_index in &pre_filter.required_columns {
                    ensure_column_fetched(
                        &mut memoized_columns,
                        &self.which_fast_fields,
                        ffhelper,
                        segment_ord,
                        ff_index,
                        &ids,
                    );
                }
                let mask = pre_filter
                    .apply_arrow(
                        ffhelper,
                        segment_ord,
                        &memoized_columns,
                        pre_filters.schema,
                        ids.len(),
                    )
                    .unwrap_or_else(|e| panic!("Pre-filter failed: {e}"));
                compact_with_mask(&mut ids, &mut scores, &mut memoized_columns, &mask);
            }
            self.pre_filter_rows_scanned += before;
            self.pre_filter_rows_pruned += before - ids.len();
        }

        // Batch lookup the ctids and visibility check them.
        // When defer_visibility is true, we skip visibility checking entirely —
        // VisibilityFilterExec will handle it in batch after the join.
        let ctids: Vec<u64> = if self.defer_visibility {
            // TODO: Remove ctid from SearchIndexScore entirely and use batch
            // lookups (similar to materialize_deferred_ctid) in consumers.
            // These placeholder zeros only appear in Batch.ids for LIMIT
            // tracking via SearchIndexScore, where the ctid value is irrelevant.
            vec![0u64; ids.len()]
        } else {
            self.maybe_ctids.resize(ids.len(), None);
            ffhelper
                .ctid(segment_ord)
                .as_u64s(&ids, &mut self.maybe_ctids);

            // Filter out invisible rows.
            self.visibility_results.resize(ids.len(), None);
            visibility.check_batch(&self.maybe_ctids, &mut self.visibility_results);

            let mut ctids = Vec::with_capacity(ids.len());
            let mut visibility_mask_builder = BooleanBuilder::with_capacity(ids.len());
            for maybe_visible_ctid in self.visibility_results.drain(..) {
                if let Some(visible_ctid) = maybe_visible_ctid {
                    visibility_mask_builder.append_value(true);
                    ctids.push(visible_ctid);
                } else {
                    visibility_mask_builder.append_value(false);
                }
            }
            // Then filter the remaining columns using the mask.
            compact_with_mask(
                &mut ids,
                &mut scores,
                &mut memoized_columns,
                &visibility_mask_builder.finish(),
            );
            ctids
        };

        // Pre-fetch any Named columns that weren't already fetched by pre-filters.
        for (ff_index, which_ff) in self.which_fast_fields.iter().enumerate() {
            if matches!(which_ff, WhichFastField::Named(_, _)) {
                ensure_column_fetched(
                    &mut memoized_columns,
                    &self.which_fast_fields,
                    ffhelper,
                    segment_ord,
                    ff_index,
                    &ids,
                );
            }
        }

        // Execute batch lookups of the fast-field values, fetch term content from the dictionaries,
        // and construct the batch.
        let fields = self
            .which_fast_fields
            .iter()
            .enumerate()
            .map(|(ff_index, which_ff)| match which_ff {
                WhichFastField::Ctid => {
                    Some(Arc::new(UInt64Array::from(ctids.clone())) as ArrayRef)
                }
                WhichFastField::Score => {
                    Some(Arc::new(Float32Array::from(scores.clone())) as ArrayRef)
                }
                WhichFastField::TableOid => {
                    let mut builder = arrow_array::builder::UInt32Builder::with_capacity(ids.len());
                    for _ in 0..ids.len() {
                        builder.append_value(self.table_oid);
                    }
                    Some(Arc::new(builder.finish()) as ArrayRef)
                }
                WhichFastField::Junk(_) => None,
                WhichFastField::Named(_, _) => {
                    let col_array = memoized_columns[ff_index].clone().unwrap();

                    match ffhelper.column(segment_ord, ff_index) {
                        FFType::Text(str_column) => {
                            let ords_array = col_array
                                .as_any()
                                .downcast_ref::<UInt64Array>()
                                .expect("Expected UInt64Array for Text ordinals");
                            Some(
                                ords_to_string_array(str_column.clone(), ords_array)
                                    .expect("Failed to lookup ordinals"),
                            )
                        }
                        FFType::Bytes(bytes_column) => {
                            let ords_array = col_array
                                .as_any()
                                .downcast_ref::<UInt64Array>()
                                .expect("Expected UInt64Array for Bytes ordinals");
                            Some(
                                ords_to_bytes_array(bytes_column.clone(), ords_array)
                                    .expect("Failed to lookup ordinals"),
                            )
                        }
                        _ => Some(col_array),
                    }
                }
                // Determine which union state to emit for the deferred column:
                // 1. Some(UInt64) -> The pre-filter already fetched ordinals. Emit State 1 (Term Ordinals).
                // 2. Some(other)  -> The pre-filter fully materialized the column. Emit State 2 (Materialized).
                // 3. None         -> The pre-filter didn't touch this column. Emit State 0 (DocAddress).
                WhichFastField::DeferredCtid(_) => Some(Arc::new(
                    crate::scan::deferred_encode::pack_doc_addresses(segment_ord, &ids),
                ) as ArrayRef),
                WhichFastField::Deferred(_, field_type) => {
                    let is_bytes = matches!(
                        field_type.arrow_data_type(),
                        arrow_schema::DataType::BinaryView | arrow_schema::DataType::LargeBinary
                    );
                    use arrow_schema::DataType;

                    match &memoized_columns[ff_index] {
                        Some(col_array) if col_array.data_type() == &DataType::UInt64 => {
                            Some(crate::scan::deferred_encode::build_state_term_ordinals(
                                segment_ord,
                                col_array.clone(),
                                is_bytes,
                            ))
                        }
                        Some(col_array) => {
                            Some(crate::scan::deferred_encode::build_state_hydrated(
                                col_array.clone(),
                                is_bytes,
                            ))
                        }
                        None => Some(crate::scan::deferred_encode::build_state_doc_address(
                            segment_ord,
                            &ids,
                            is_bytes,
                        )),
                    }
                }
            })
            .collect();

        Some(Batch {
            ids: ids
                .into_iter()
                .zip(scores)
                .zip(ctids)
                .map(|((id, score), ctid)| {
                    (
                        SearchIndexScore::new(ctid, score),
                        DocAddress::new(segment_ord, id),
                    )
                })
                .collect(),
            fields,
        })
    }

    /// Prefetch a single batch and store it for the next `next()` call.
    ///
    /// This is used to force some work between parallel segment checkouts while
    /// preserving correctness (the prefetched batch will still be returned).
    ///
    /// **WARNING:** This method is specialized for multi-partition parallel workflows
    /// (where all partitions must be opened concurrently and checked out via throttled loop).
    /// It should **not** be used for single-partition lazy execution, as chaining segments
    /// end-on-end dynamically does not require prefetching to yield time.
    pub fn prefetch_next(
        &mut self,
        ffhelper: &FFHelper,
        visibility: &mut VisibilityChecker,
        pre_filters: Option<&crate::scan::pre_filter::PreFilters<'_>>,
    ) {
        if self.prefetched.is_none() {
            if let Some(batch) = self.next(ffhelper, visibility, pre_filters) {
                self.prefetched = Some(batch);
            }
        }
    }
}
