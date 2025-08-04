// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::index::fast_fields_helper::FFHelper;
use crate::index::fast_fields_helper::{FFType, WhichFastField};
use crate::index::reader::index::MultiSegmentSearchResults;
use crate::index::reader::index::SearchIndexScore;
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    non_string_ff_to_datum, ords_to_sorted_terms, FastFieldExecState, NULL_TERM_ORDINAL,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::postgres::types::TantivyValue;

use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::PgOid;
use tantivy::schema::document::OwnedValue;
use tantivy::DocAddress;
use tantivy::SegmentOrdinal;

/// The number of rows to batch materialize in memory while iterating over a result set.
///
/// Setting this value larger reduces the cost of our joins to the term dictionary by allowing more
/// terms to be looked up at a time, but increases our memory usage by forcing more column values to
/// be held in memory at a time.
const JOIN_BATCH_SIZE: usize = 128_000;

/// A macro to fetch values for the given ids into a Vec<OwnedValue>.
macro_rules! fetch_ff_column {
    ($col:expr, $ids:ident, $($ff_type:ident => $owned_value:ident),* $(,)?) => {
        match $col {
            $(
                FFType::$ff_type(col) => {
                    let mut column_results = Vec::with_capacity($ids.len());
                    column_results.resize($ids.len(), None);
                    col.first_vals(&$ids, &mut column_results);
                    column_results.into_iter().map(|maybe_val| {
                        TantivyValue(maybe_val.map(OwnedValue::$owned_value).unwrap_or(OwnedValue::Null))
                    }).collect::<Vec<_>>()
                }
            )*
            x => panic!("Unhandled column type {x:?}"),
        }
    };
}

/// Execution state for mixed fast field retrieval optimized for both string and numeric fields.
///
/// This execution state is designed to handle two scenarios that previous implementations
/// couldn't handle efficiently:
/// 1. Multiple string fast fields in a single query
/// 2. A mix of string and numeric fast fields in a single query
///
/// Rather than reimplementing all functionality, this struct uses composition to build on
/// the existing FastFieldExecState while adding optimized processing paths for mixed field types.
///
/// # Usage Context
/// This execution method is selected when a query uses multiple fast fields with at least one
/// string fast field. It processes both string and numeric fields directly from the index's
/// fast field data structures, avoiding the need to fetch full documents.
///
/// # Feature Flag
/// This execution method is controlled by the `paradedb.enable_mixed_fast_field_exec` GUC setting.
/// It is disabled by default and can be enabled with:
/// ```sql
/// SET paradedb.enable_mixed_fast_field_exec = true;
/// ```
pub struct MixedFastFieldExecState {
    /// Core functionality shared with other fast field execution methods
    inner: FastFieldExecState,

    /// The batch size to use for this execution.
    batch_size: usize,

    /// The segment(s) that we have queried.
    search_results: Option<MultiSegmentSearchResults>,

    /// The current batch of fast field values
    batch: Batch,

    /// Statistics tracking the number of visible rows
    num_visible: usize,
}

impl MixedFastFieldExecState {
    /// Creates a new MixedFastFieldExecState from a list of fast fields.
    ///
    /// This constructor analyzes the provided fast fields and categorizes them
    /// into string and numeric types for optimized processing.
    ///
    /// # Arguments
    ///
    /// * `which_fast_fields` - Vector of fast fields that will be processed
    ///
    /// # Returns
    ///
    /// A new MixedFastFieldExecState instance
    pub fn new(
        which_fast_fields: Vec<WhichFastField>,
        limit: Option<usize>,
        can_use_virtual: bool,
    ) -> Self {
        // If there is a limit, then we use a batch size which is a small multiple of the limit, in
        // case of dead tuples.
        let batch_size = limit
            .map(|limit| std::cmp::min(limit * 2, JOIN_BATCH_SIZE))
            .unwrap_or(JOIN_BATCH_SIZE);
        Self {
            inner: FastFieldExecState::new(which_fast_fields, can_use_virtual),
            batch_size,
            search_results: None,
            batch: Batch::default(),
            num_visible: 0,
        }
    }

    fn try_get_batch_ids(&mut self) -> Option<(SegmentOrdinal, Vec<f32>, Vec<u32>)> {
        let search_results = self.search_results.as_mut()?;

        // Collect a batch of ids for a single segment.
        loop {
            let scorer_iter = search_results.current_segment()?;
            let segment_ord = scorer_iter.segment_ord();

            // Collect a batch of ids/scores for this segment.
            let mut scores = Vec::with_capacity(self.batch_size);
            let mut ids = Vec::with_capacity(self.batch_size);
            while ids.len() < self.batch_size {
                let Some((score, id)) = scorer_iter.next() else {
                    // No more results for the current segment: remove it.
                    search_results.current_segment_pop();
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

    /// If our SearchResults iterator contains entries, take one batch, and construct a new
    /// `joined_results` value which will lazily join them.
    fn try_join_batch(&mut self) -> bool {
        let Some((segment_ord, scores, ids)) = self.try_get_batch_ids() else {
            return false;
        };

        // Batch lookup the ctids.
        let ctids: Vec<u64> = {
            let mut ctids = Vec::with_capacity(ids.len());
            ctids.resize(ids.len(), None);
            self.inner
                .ffhelper
                .ctid(segment_ord)
                .as_u64s(&ids, &mut ctids);
            ctids
                .into_iter()
                .map(|ctid| ctid.expect("All docs must have ctids"))
                .collect()
        };

        // Execute batch lookups of the fast-field values, and construct the batch.
        self.batch.fields = self
            .inner
            .which_fast_fields
            .iter()
            .enumerate()
            .map(
                |(ff_index, _)| match self.inner.ffhelper.column(segment_ord, ff_index) {
                    FFType::Text(str_column) => {
                        // Get the term ordinals.
                        let mut term_ords = Vec::with_capacity(ids.len());
                        term_ords.resize(ids.len(), None);
                        str_column.ords().first_vals(&ids, &mut term_ords);
                        // Then enumerate to preserve the id index, and look up in
                        // the term dictionary.
                        let sorted_terms = ords_to_sorted_terms(
                            str_column.clone(),
                            term_ords.into_iter().enumerate().collect::<Vec<_>>(),
                            |(_, maybe_ord)| maybe_ord.unwrap_or(NULL_TERM_ORDINAL),
                        );
                        // Re-arrange the resulting terms back to docid order.
                        let mut terms = Vec::with_capacity(ids.len());
                        terms.resize(ids.len(), TantivyValue(OwnedValue::Null));
                        for ((index, _), term) in sorted_terms {
                            if let Some(term) = term {
                                // TODO: Immediately unwrapping the Rc after creation: should remove it.
                                terms[index] = TantivyValue(OwnedValue::Str((*term).to_owned()));
                            }
                        }
                        Some(terms)
                    }
                    FFType::Junk => None,
                    numeric_column => Some(fetch_ff_column!(numeric_column, ids,
                        I64 => I64,
                        F64 => F64,
                        U64 => U64,
                        Bool => Bool,
                        Date => Date,
                    )),
                },
            )
            .collect();

        self.batch.offset = 0;
        self.batch.ids.clear();
        self.batch.ids.extend(
            ids.into_iter()
                .zip(scores)
                .zip(ctids)
                .map(|((id, score), ctid)| {
                    (
                        SearchIndexScore::new(ctid, score),
                        DocAddress::new(segment_ord, id),
                    )
                }),
        );

        true
    }
}

impl ExecMethod for MixedFastFieldExecState {
    /// Initializes the execution state with the necessary context.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state containing query information
    /// * `cstate` - PostgreSQL's custom scan state pointer
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut pg_sys::CustomScanState) {
        // Initialize the inner FastFieldExecState
        self.inner.init(state, cstate);

        // Reset mixed field specific state
        self.search_results = None;
        self.batch.reset();
        self.num_visible = 0;
    }

    /// Executes the search query and prepares result processing.
    ///
    /// This method handles both parallel and non-parallel execution paths.
    /// For parallel execution, it processes a single segment at a time.
    /// For non-parallel execution, it processes all segments at once.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state containing query information
    ///
    /// # Returns
    ///
    /// `true` if there are results to process, `false` otherwise
    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if self.try_join_batch() {
            // We collected another batch of ids from the SearchResult: construct a
            return true;
        }

        // Handle parallel query execution
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                self.search_results = Some(
                    state
                        .search_reader
                        .as_ref()
                        .unwrap()
                        .search_segments([segment_id].into_iter()),
                );
                return true;
            }

            // No more segments to query in parallel mode
            false
        } else if self.inner.did_query {
            // Not parallel and already queried
            false
        } else {
            // First time query in non-parallel mode
            self.search_results = Some(state.search_reader.as_ref().unwrap().search(None));
            self.inner.did_query = true;
            true
        }
    }

    /// Fetches the next result and prepares it for returning to PostgreSQL.
    ///
    /// This method converts optimized search results into PostgreSQL tuple format,
    /// handling value retrieval for both string and numeric fields.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state
    ///
    /// # Returns
    ///
    /// The next execution state containing the result or EOF
    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        unsafe {
            // Process the next result from our optimized path
            match self.batch.next() {
                None => {
                    // No more in the current batch: trampoline out to ExecMethod::next to
                    // construct the next batch, if any.
                    ExecState::Eof
                }
                Some((row_idx, scored, doc_address)) => {
                    let heaprel = self
                        .inner
                        .heaprel
                        .as_ref()
                        .expect("MixedFastFieldsExecState: heaprel should be initialized");
                    let slot = self.inner.slot;
                    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                    // Set ctid and table OID
                    crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                    (*slot).tts_tableOid = heaprel.oid();

                    // Check visibility of the current block
                    let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                    let is_visible = if blockno == self.inner.blockvis.0 {
                        // We already know the visibility of this block because we just checked it last time
                        self.inner.blockvis.1
                    } else {
                        // New block, check visibility
                        self.inner.blockvis.0 = blockno;
                        self.inner.blockvis.1 =
                            is_block_all_visible(heaprel, &mut self.inner.vmbuff, blockno);
                        self.inner.blockvis.1
                    };

                    if is_visible && self.inner.can_use_virtual {
                        self.inner.blockvis = (blockno, true);

                        // Setup slot for returning data
                        (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                        (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                        (*slot).tts_nvalid = natts as _;

                        let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                        let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                        // Initialize all values to NULL
                        for i in 0..natts {
                            datums[i] = pg_sys::Datum::null();
                            isnull[i] = true;
                        }

                        let which_fast_fields = &self.inner.which_fast_fields;
                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();
                        debug_assert!(natts == which_fast_fields.len());

                        self.batch.populate(
                            row_idx,
                            scored,
                            doc_address,
                            which_fast_fields,
                            &mut self.inner.ffhelper,
                            tupdesc,
                            &mut *slot,
                            datums,
                            isnull,
                        );

                        ExecState::Virtual { slot }
                    } else {
                        // Row needs visibility check
                        ExecState::RequiresVisibilityCheck {
                            ctid: scored.ctid,
                            score: scored.bm25,
                            doc_address,
                        }
                    }
                }
            }
        }
    }

    /// Resets the execution state to its initial state.
    ///
    /// # Arguments
    ///
    /// * `state` - The current scan state
    fn reset(&mut self, state: &mut PdbScanState) {
        // Reset inner FastFieldExecState
        self.inner.reset(state);

        // Reset mixed results state
        self.search_results = None;
        self.batch.reset();

        // Reset statistics
        self.num_visible = 0;
    }

    /// Increments the count of visible rows.
    ///
    /// Called when a row passes visibility checks.
    fn increment_visible(&mut self) {
        self.num_visible += 1;
    }
}

/// A batch of tuples.
///
/// In order to be able to copy directly from the fetched columns into a tuple slot and to reuse
/// buffers, this structure acts like an inverted Iterator:
/// * Call `next()` to get the next ctid.
/// * If the ctid is interesting, call `populate()` for the row_idx.
#[derive(Default)]
struct Batch {
    /// The current offset in the ids.
    offset: usize,

    /// An iterator of ids which have been consumed from the underlying `SearchResults`
    /// iterator as a batch.
    ids: Vec<(SearchIndexScore, DocAddress)>,

    /// The current batch of fast field values, indexed by FFIndex, then by row.
    /// TODO: Use Arrow here?
    fields: Vec<Option<Vec<TantivyValue>>>,
}

impl Batch {
    fn next(&mut self) -> Option<(usize, SearchIndexScore, DocAddress)> {
        let res = self
            .ids
            .get(self.offset)
            .map(|(s, d)| (self.offset, *s, *d));
        self.offset += 1;
        res
    }

    #[allow(clippy::too_many_arguments)]
    fn populate(
        &mut self,
        row_idx: usize,
        scored: SearchIndexScore,
        doc_address: DocAddress,
        which_fast_fields: &[WhichFastField],
        ff_helper: &mut FFHelper,
        tupdesc: &pgrx::PgTupleDesc,
        slot: &mut pg_sys::TupleTableSlot,
        datums: &mut [pg_sys::Datum],
        isnull: &mut [bool],
    ) {
        for (i, (att, which_fast_field)) in tupdesc.iter().zip(which_fast_fields).enumerate() {
            match &mut self.fields[i] {
                Some(column) => {
                    // We extracted this field: convert it into a datum.
                    let datum_res = unsafe {
                        std::mem::take(&mut column[row_idx])
                            .try_into_datum(PgOid::from(att.atttypid))
                    };
                    match datum_res {
                        Ok(Some(datum)) => {
                            datums[i] = datum;
                            isnull[i] = false;
                            continue;
                        }
                        Ok(None) => {
                            // Null datum.
                            continue;
                        }
                        Err(e) => {
                            panic!(
                                "Failed to convert to attribute type for \
                                {:?} and {which_fast_field:?}: {e}",
                                att.atttypid
                            );
                        }
                    }
                }
                None => {
                    // Fall back to non_string_ff_to_datum for things like the score, ctid,
                    // etc.
                    let datum_opt = unsafe {
                        non_string_ff_to_datum(
                            (&which_fast_fields[i], i),
                            att.atttypid,
                            scored.bm25,
                            doc_address,
                            ff_helper,
                            slot,
                        )
                    };
                    if let Some(datum) = datum_opt {
                        datums[i] = datum;
                        isnull[i] = false;
                    }
                }
            }
        }
    }

    fn reset(&mut self) {
        self.offset = 0;
        self.ids.clear();
        self.fields.clear();
    }
}
