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

use std::convert::identity;
use std::sync::Arc;

use arrow_array::builder::{
    BinaryViewBuilder, BooleanBuilder, Float64Builder, Int64Builder, StringViewBuilder,
    TimestampNanosecondBuilder, UInt64Builder,
};
use arrow_array::{ArrayRef, Float32Array, RecordBatch, UInt64Array};
use arrow_buffer::Buffer;
use arrow_schema::SchemaRef;
use tantivy::columnar::{BytesColumn, StrColumn};
use tantivy::termdict::TermOrdinal;
use tantivy::{DocAddress, SegmentOrdinal};

use crate::index::fast_fields_helper::{build_arrow_schema, FFHelper, FFType, WhichFastField};
use crate::index::reader::index::{MultiSegmentSearchResults, SearchIndexScore};
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::types_arrow::date_time_to_ts_nanos;

use super::pre_filter::{apply_pre_filter, PreFilter};

/// The maximum number of rows to batch materialize in memory while iterating over a result set.
///
/// Setting this value larger reduces the cost of our joins to the term dictionary by allowing more
/// terms to be looked up at a time, but increases our memory usage by forcing more column values to
/// be held in memory at a time.
const MAX_BATCH_SIZE: usize = 128_000;

const NULL_TERM_ORDINAL: TermOrdinal = u64::MAX;

/// A macro to fetch values for the given ids into an Arrow array.
macro_rules! fetch_ff_column {
    ($col:expr, $ids:ident, $($ff_type:ident => $conversion:ident => $builder:ident),* $(,)?) => {
        match $col {
            $(
                FFType::$ff_type(col) => {
                    let mut column_results = Vec::with_capacity($ids.len());
                    column_results.resize($ids.len(), None);
                    col.first_vals(&$ids, &mut column_results);
                    let mut builder = $builder::with_capacity($ids.len());
                    for maybe_val in column_results {
                        if let Some(val) = maybe_val {
                            builder.append_value($conversion(val));
                        } else {
                            builder.append_null();
                        }
                    }
                    Arc::new(builder.finish()) as ArrayRef
                }
            )*
            x => panic!("Unhandled column type {x:?}"),
        }
    };
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
    /// many total rows will be requested (e.g. `LIMIT` queries where `MixedFastFieldExecState`
    /// is the top-level node). In all other cases (e.g. `JoinScan`, `TableProvider`), it
    /// should be `None` to allow the default batch size to be used, which is optimized for
    /// mixed fast field string lookups.
    pub fn new(
        search_results: MultiSegmentSearchResults,
        batch_size_hint: Option<usize>,
        which_fast_fields: Vec<WhichFastField>,
        table_oid: u32,
    ) -> Self {
        let batch_size = batch_size_hint
            .unwrap_or(MAX_BATCH_SIZE)
            .min(MAX_BATCH_SIZE);
        Self {
            search_results,
            batch_size,
            which_fast_fields,
            table_oid,
            maybe_ctids: Vec::new(),
            visibility_results: Vec::new(),
            prefetched: None,
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

    fn try_get_batch_ids(&mut self) -> Option<(SegmentOrdinal, Vec<f32>, Vec<u32>)> {
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
    /// materialization, allowing string-column filters to operate on cheap
    /// term ordinals rather than requiring expensive dictionary lookups.
    pub fn next(
        &mut self,
        ffhelper: &FFHelper,
        visibility: &mut VisibilityChecker,
        pre_filters: &[PreFilter],
    ) -> Option<Batch> {
        if let Some(batch) = self.prefetched.take() {
            return Some(batch);
        }
        pgrx::check_for_interrupts!();
        let (segment_ord, mut scores, mut ids) = self.try_get_batch_ids()?;

        // Batch lookup the ctids.
        self.maybe_ctids.resize(ids.len(), None);
        ffhelper
            .ctid(segment_ord)
            .as_u64s(&ids, &mut self.maybe_ctids);

        // Filter out invisible rows.
        self.visibility_results.resize(ids.len(), None);
        visibility.check_batch(&self.maybe_ctids, &mut self.visibility_results);

        let mut ctids = Vec::with_capacity(ids.len());
        let mut write_idx = 0;
        for (read_idx, maybe_visible_ctid) in self.visibility_results.iter().enumerate() {
            if let Some(visible_ctid) = maybe_visible_ctid {
                ctids.push(*visible_ctid);
                if read_idx != write_idx {
                    ids[write_idx] = ids[read_idx];
                    scores[write_idx] = scores[read_idx];
                }
                write_idx += 1;
            }
        }
        ids.truncate(write_idx);
        scores.truncate(write_idx);

        // Apply pre-materialization filters (before expensive dictionary lookups).
        if !pre_filters.is_empty() {
            let before = ids.len();
            for pre_filter in pre_filters {
                if ids.is_empty() {
                    break;
                }
                apply_pre_filter(
                    ffhelper,
                    segment_ord,
                    pre_filter,
                    &mut ids,
                    &mut ctids,
                    &mut scores,
                );
            }
            self.pre_filter_rows_scanned += before;
            self.pre_filter_rows_pruned += before - ids.len();
        }

        // Execute batch lookups of the fast-field values, and construct the batch.
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
                WhichFastField::Named(_, _) => match ffhelper.column(segment_ord, ff_index) {
                    FFType::Text(str_column) => {
                        // Get the term ordinals.
                        let mut term_ords = Vec::with_capacity(ids.len());
                        term_ords.resize(ids.len(), None);
                        str_column.ords().first_vals(&ids, &mut term_ords);
                        Some(ords_to_string_array(
                            str_column.clone(),
                            term_ords
                                .into_iter()
                                .map(|maybe_ord| maybe_ord.unwrap_or(NULL_TERM_ORDINAL)),
                        ))
                    }
                    FFType::Bytes(bytes_column) => {
                        // Get the term ordinals for bytes columns.
                        let mut term_ords = Vec::with_capacity(ids.len());
                        term_ords.resize(ids.len(), None);
                        bytes_column.ords().first_vals(&ids, &mut term_ords);
                        Some(ords_to_bytes_array(
                            bytes_column.clone(),
                            term_ords
                                .into_iter()
                                .map(|maybe_ord| maybe_ord.unwrap_or(NULL_TERM_ORDINAL)),
                        ))
                    }
                    FFType::Junk => None,
                    numeric_column => Some(fetch_ff_column!(numeric_column, ids,
                        I64  => identity => Int64Builder,
                        F64  => identity => Float64Builder,
                        U64  => identity => UInt64Builder,
                        Bool => identity => BooleanBuilder,
                        Date => date_time_to_ts_nanos => TimestampNanosecondBuilder,
                    )),
                },
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
    pub fn prefetch_next(
        &mut self,
        ffhelper: &FFHelper,
        visibility: &mut VisibilityChecker,
        pre_filters: &[PreFilter],
    ) {
        if self.prefetched.is_none() {
            if let Some(batch) = self.next(ffhelper, visibility, pre_filters) {
                self.prefetched = Some(batch);
            }
        }
    }
}

/// Given an unordered collection of TermOrdinals for the given StrColumn, return a
/// `StringViewArray` with one row per input term ordinal (in the input order).
///
/// A `StringViewArray` contains a series of buffers containing arbitrarily concatenated bytes data,
/// and then a series of (buffer, offset, len) entries representing views into those buffers. This
/// method creates a single buffer containing the concatenated data for the given term ordinals in
/// term sorted order, and then a view per input row in input order. A caller can ignore those
/// details and just consume the array as if it were an array of strings.
///
/// `NULL_TERM_ORDINAL` represents NULL, and will be emitted last in the sorted order.
fn ords_to_string_array(
    str_ff: StrColumn,
    term_ords: impl IntoIterator<Item = TermOrdinal>,
) -> ArrayRef {
    // Enumerate the term ordinals to preserve their positions, and then sort them by ordinal.
    let mut term_ords = term_ords.into_iter().enumerate().collect::<Vec<_>>();
    term_ords.sort_unstable_by_key(|(_, term_ord)| *term_ord);

    // Iterate over the sorted term ordinals: as we visit each term ordinal, we will append the
    // term to a StringViewBuilder's data buffer, and record a view to be appended later in sorted
    // order.
    let mut builder = StringViewBuilder::with_capacity(term_ords.len());
    let mut views: Vec<Option<(u32, u32)>> = Vec::with_capacity(term_ords.len());
    views.resize(term_ords.len(), None);

    let mut buffer = Vec::new();
    let mut bytes = Vec::new();
    let mut current_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(0);
    let mut current_sstable_delta_reader = str_ff
        .dictionary()
        .sstable_delta_reader_block(current_block_addr.clone())
        .expect("Failed to open term dictionary.");
    let mut current_ordinal = 0;
    let mut previous_term: Option<(TermOrdinal, (u32, u32))> = None;
    for (row_idx, ord) in term_ords {
        if ord == NULL_TERM_ORDINAL {
            // NULL_TERM_ORDINAL sorts highest, so all remaining ords will have `None` views, and
            // be appended to the builder as null.
            break;
        }

        // only advance forward if the new ord is different than the one we just processed
        //
        // this allows the input TermOrdinal iterator to contain and reuse duplicates, so long as
        // it's still sorted
        match &previous_term {
            Some((previous_ord, previous_view)) if *previous_ord == ord => {
                // This is the same term ordinal: reuse the previous view.
                views[row_idx] = Some(*previous_view);
                continue;
            }
            // Fall through.
            _ => {}
        }

        // This is a new term ordinal: decode it and append it to the builder.
        assert!(ord >= current_ordinal);
        // check if block changed for new term_ord
        let new_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(ord);
        if new_block_addr != current_block_addr {
            current_block_addr = new_block_addr;
            current_ordinal = current_block_addr.first_ordinal;
            current_sstable_delta_reader = str_ff
                .dictionary()
                .sstable_delta_reader_block(current_block_addr.clone())
                .unwrap_or_else(|e| panic!("Failed to fetch next dictionary block: {e}"));
            bytes.clear();
        }

        // Move to ord inside that block
        for _ in current_ordinal..=ord {
            match current_sstable_delta_reader.advance() {
                Ok(true) => {}
                Ok(false) => {
                    panic!("Term ordinal {ord} did not exist in the dictionary.");
                }
                Err(e) => {
                    panic!("Failed to decode dictionary block: {e}")
                }
            }
            bytes.truncate(current_sstable_delta_reader.common_prefix_len());
            bytes.extend_from_slice(current_sstable_delta_reader.suffix());
        }
        current_ordinal = ord + 1;

        // Set the view for this row_idx.
        let offset: u32 = buffer
            .len()
            .try_into()
            .expect("Too many terms requested in `ords_to_string_array`");
        let len: u32 = bytes
            .len()
            .try_into()
            .expect("Single term is too long in `ords_to_string_array`");
        buffer.extend_from_slice(&bytes);
        previous_term = Some((ord, (offset, len)));
        views[row_idx] = Some((offset, len));
    }

    // Append all the rows' views to the builder.
    let block_no = builder.append_block(Buffer::from(buffer));
    for view in views {
        // Each view is an offset and len in our single block, or None for a null.
        match view {
            Some((offset, len)) => unsafe {
                builder.append_view_unchecked(block_no, offset, len);
            },
            None => builder.append_null(),
        }
    }

    Arc::new(builder.finish())
}

/// Given an unordered collection of TermOrdinals for the given BytesColumn, return a
/// `BinaryViewArray` with one row per input term ordinal (in the input order).
///
/// This is identical to `ords_to_string_array` but uses `BinaryViewBuilder` for binary data.
///
/// `NULL_TERM_ORDINAL` represents NULL, and will be emitted last in the sorted order.
fn ords_to_bytes_array(
    bytes_ff: BytesColumn,
    term_ords: impl IntoIterator<Item = TermOrdinal>,
) -> ArrayRef {
    // Enumerate the term ordinals to preserve their positions, and then sort them by ordinal.
    let mut term_ords = term_ords.into_iter().enumerate().collect::<Vec<_>>();
    term_ords.sort_unstable_by_key(|(_, term_ord)| *term_ord);

    // Iterate over the sorted term ordinals: as we visit each term ordinal, we will append the
    // term to a BinaryViewBuilder's data buffer, and record a view to be appended later in sorted
    // order.
    let mut builder = BinaryViewBuilder::with_capacity(term_ords.len());
    let mut views: Vec<Option<(u32, u32)>> = Vec::with_capacity(term_ords.len());
    views.resize(term_ords.len(), None);

    let mut buffer = Vec::new();
    let mut bytes = Vec::new();
    let mut current_block_addr = bytes_ff.dictionary().sstable_index.get_block_with_ord(0);
    let mut current_sstable_delta_reader = bytes_ff
        .dictionary()
        .sstable_delta_reader_block(current_block_addr.clone())
        .expect("Failed to open term dictionary.");
    let mut current_ordinal = 0;
    let mut previous_term: Option<(TermOrdinal, (u32, u32))> = None;
    for (row_idx, ord) in term_ords {
        if ord == NULL_TERM_ORDINAL {
            // NULL_TERM_ORDINAL sorts highest, so all remaining ords will have `None` views, and
            // be appended to the builder as null.
            break;
        }

        // only advance forward if the new ord is different than the one we just processed
        //
        // this allows the input TermOrdinal iterator to contain and reuse duplicates, so long as
        // it's still sorted
        match &previous_term {
            Some((previous_ord, previous_view)) if *previous_ord == ord => {
                // This is the same term ordinal: reuse the previous view.
                views[row_idx] = Some(*previous_view);
                continue;
            }
            // Fall through.
            _ => {}
        }

        // This is a new term ordinal: decode it and append it to the builder.
        assert!(ord >= current_ordinal);
        // check if block changed for new term_ord
        let new_block_addr = bytes_ff.dictionary().sstable_index.get_block_with_ord(ord);
        if new_block_addr != current_block_addr {
            current_block_addr = new_block_addr;
            current_ordinal = current_block_addr.first_ordinal;
            current_sstable_delta_reader = bytes_ff
                .dictionary()
                .sstable_delta_reader_block(current_block_addr.clone())
                .unwrap_or_else(|e| panic!("Failed to fetch next dictionary block: {e}"));
            bytes.clear();
        }

        // Move to ord inside that block
        for _ in current_ordinal..=ord {
            match current_sstable_delta_reader.advance() {
                Ok(true) => {}
                Ok(false) => {
                    panic!("Term ordinal {ord} did not exist in the dictionary.");
                }
                Err(e) => {
                    panic!("Failed to decode dictionary block: {e}")
                }
            }
            bytes.truncate(current_sstable_delta_reader.common_prefix_len());
            bytes.extend_from_slice(current_sstable_delta_reader.suffix());
        }
        current_ordinal = ord + 1;

        // Set the view for this row_idx.
        let offset: u32 = buffer
            .len()
            .try_into()
            .expect("Too many terms requested in `ords_to_bytes_array`");
        let len: u32 = bytes
            .len()
            .try_into()
            .expect("Single term is too long in `ords_to_bytes_array`");
        buffer.extend_from_slice(&bytes);
        previous_term = Some((ord, (offset, len)));
        views[row_idx] = Some((offset, len));
    }

    // Append all the rows' views to the builder.
    let block_no = builder.append_block(Buffer::from(buffer));
    for view in views {
        // Each view is an offset and len in our single block, or None for a null.
        match view {
            Some((offset, len)) => unsafe {
                builder.append_view_unchecked(block_no, offset, len);
            },
            None => builder.append_null(),
        }
    }

    Arc::new(builder.finish())
}
