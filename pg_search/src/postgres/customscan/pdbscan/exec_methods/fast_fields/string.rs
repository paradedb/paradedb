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

use std::rc::Rc;

use crate::index::fast_fields_helper::{FFIndex, WhichFastField};
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::{
    non_string_ff_to_datum, ords_to_sorted_terms, FastFieldExecState,
};
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::fast_fields::StrColumn;
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::query::SearchQueryInput;

use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::pg_sys::CustomScanState;
use pgrx::IntoDatum;
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::query::Query;
use tantivy::schema::Schema;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocAddress, Executor, SegmentOrdinal};

pub struct StringFastFieldExecState {
    inner: FastFieldExecState,
    search_results: StringAggResults,
    field: String,
    field_idx: FFIndex,
}

impl StringFastFieldExecState {
    pub fn new(field: String, which_fast_fields: Vec<WhichFastField>) -> Self {
        let field_idx = which_fast_fields
            .iter()
            .position(|wff| matches!(wff, WhichFastField::Named(name, _) if name == &field))
            .unwrap_or_else(|| {
                panic!("No string fast field named {field} in {which_fast_fields:?}")
            });
        Self {
            inner: FastFieldExecState::new(which_fast_fields),
            search_results: StringAggResults::new(vec![]),
            field,
            field_idx,
        }
    }
}

impl ExecMethod for StringFastFieldExecState {
    fn init(&mut self, state: &mut PdbScanState, cstate: *mut CustomScanState) {
        self.inner.init(state, cstate);
    }

    fn query(&mut self, state: &mut PdbScanState) -> bool {
        if let Some(parallel_state) = state.parallel_state {
            if let Some(segment_id) = unsafe { checkout_segment(parallel_state) } {
                let searcher = StringAggSearcher(state.search_reader.as_ref().unwrap());
                self.search_results = searcher.string_agg_by_segment(
                    state.need_scores(),
                    state.enhanced_search_query_input(),
                    &self.field,
                    segment_id,
                );
                return true;
            }

            // no more segments to query
            self.inner.search_results = SearchResults::None;
            false
        } else if self.inner.did_query {
            // not parallel, so we're done
            false
        } else {
            // not parallel, first time query
            let searcher = StringAggSearcher(state.search_reader.as_ref().unwrap());
            self.search_results = searcher.string_agg(
                state.need_scores(),
                state.enhanced_search_query_input(),
                &self.field,
            );
            self.inner.did_query = true;
            true
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        unsafe {
            match self.search_results.next() {
                None => ExecState::Eof,
                Some((scored, doc_address, term)) => {
                    let slot = self.inner.slot;
                    let natts = (*(*slot).tts_tupleDescriptor).natts as usize;

                    crate::postgres::utils::u64_to_item_pointer(scored.ctid, &mut (*slot).tts_tid);
                    (*slot).tts_tableOid = (*self.inner.heaprel).rd_id;

                    let blockno = item_pointer_get_block_number(&(*slot).tts_tid);
                    let is_visible = if blockno == self.inner.blockvis.0 {
                        // we know the visibility of this block because we just checked it last time
                        self.inner.blockvis.1
                    } else {
                        // new block so check its visibility
                        self.inner.blockvis.0 = blockno;
                        self.inner.blockvis.1 = is_block_all_visible(
                            self.inner.heaprel,
                            &mut self.inner.vmbuff,
                            blockno,
                        );
                        self.inner.blockvis.1
                    };

                    if is_visible {
                        self.inner.blockvis = (blockno, true);

                        (*slot).tts_flags &= !pg_sys::TTS_FLAG_EMPTY as u16;
                        (*slot).tts_flags |= pg_sys::TTS_FLAG_SHOULDFREE as u16;
                        (*slot).tts_nvalid = natts as _;

                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();
                        let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
                        let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

                        for (i, att) in tupdesc.iter().enumerate() {
                            if i == self.field_idx {
                                isnull[i] = term.is_null();
                                datums[i] = term;
                                continue;
                            }

                            match non_string_ff_to_datum(
                                (&self.inner.which_fast_fields[i], i),
                                att.atttypid,
                                scored.bm25,
                                doc_address,
                                &mut self.inner.ffhelper,
                                slot,
                            ) {
                                None => {
                                    datums[i] = pg_sys::Datum::null();
                                    isnull[i] = true;
                                }
                                Some(datum) => {
                                    datums[i] = datum;
                                    isnull[i] = false;
                                }
                            }
                        }

                        ExecState::Virtual { slot }
                    } else {
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

    fn reset(&mut self, _state: &mut PdbScanState) {
        // Reset tracking state but don't clear search_results - that's handled by PdbScanState.reset()
        self.inner.reset(_state);
    }
}

/// The result of searching one segment: a column, and vec of matches with TermOrdinals for that
/// column.
type SegmentResult = (StrColumn, Vec<(TermOrdinal, SearchIndexScore, DocAddress)>);

struct StringAggResults {
    /// Per-segment results which have yet to be emitted.
    per_segment: std::vec::IntoIter<SegmentResult>,
    /// An iterator for the current segment.
    current_segment: Box<dyn Iterator<Item = (SearchIndexScore, DocAddress, Option<Rc<str>>)>>,
}

impl StringAggResults {
    #[allow(clippy::type_complexity)]
    fn new(per_segment: Vec<SegmentResult>) -> Self {
        Self {
            per_segment: per_segment.into_iter(),
            current_segment: Box::new(std::iter::empty()),
        }
    }
}

impl Iterator for StringAggResults {
    type Item = (SearchIndexScore, DocAddress, pg_sys::Datum);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // See if there are more results from the current segment.
            if let Some((score, doc_address, term)) = self.current_segment.next() {
                let datum = term
                    .map(|term| {
                        term.into_datum()
                            .expect("String fast field must be a datum.")
                    })
                    .unwrap_or_else(pg_sys::Datum::null);
                return Some((score, doc_address, datum));
            }

            // Get results from the next segment, if any.
            let (str_ff, results) = self.per_segment.next()?;
            self.current_segment = Box::new(
                ords_to_sorted_terms(str_ff, results, |(term_ordinal, _, _)| *term_ordinal)
                    .map(|((_, scored, doc_address), term)| (scored, doc_address, term)),
            );
        }
    }
}

struct StringAggSearcher<'a>(&'a SearchIndexReader);

impl StringAggSearcher<'_> {
    pub fn string_agg(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        field: &str,
    ) -> StringAggResults {
        let collector = term_ord_collector::TermOrdCollector {
            need_scores,
            field: field.into(),
        };

        let query = self.0.query(query);
        let schema = Schema::from(self.0.schema().clone());
        let results = self
            .0
            .searcher()
            .search_with_executor(
                &query,
                &collector,
                &Executor::SingleThread,
                if need_scores {
                    tantivy::query::EnableScoring::Enabled {
                        searcher: self.0.searcher(),
                        statistics_provider: self.0.searcher(),
                    }
                } else {
                    tantivy::query::EnableScoring::Disabled {
                        schema: &schema,
                        searcher_opt: Some(self.0.searcher()),
                    }
                },
            )
            .expect("failed to search");

        StringAggResults::new(results)
    }

    pub fn string_agg_by_segment(
        &self,
        need_scores: bool,
        query: &SearchQueryInput,
        field: &str,
        segment_id: SegmentId,
    ) -> StringAggResults {
        let (segment_ord, segment_reader) = self
            .0
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
        let collector = term_ord_collector::TermOrdCollector {
            need_scores,
            field: field.into(),
        };
        let schema = Schema::from(self.0.schema().clone());
        let weight = self
            .0
            .query(query)
            .weight(if need_scores {
                tantivy::query::EnableScoring::Enabled {
                    searcher: self.0.searcher(),
                    statistics_provider: self.0.searcher(),
                }
            } else {
                tantivy::query::EnableScoring::Disabled {
                    schema: &schema,
                    searcher_opt: Some(self.0.searcher()),
                }
            })
            .expect("weight should be constructable");

        let results = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("single segment collection should succeed");

        StringAggResults::new(vec![results])
    }
}

mod term_ord_collector {
    use crate::index::fast_fields_helper::FFType;
    use crate::index::reader::index::SearchIndexScore;
    use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::NULL_TERM_ORDINAL;

    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::columnar::StrColumn;
    use tantivy::termdict::TermOrdinal;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    pub struct TermOrdCollector {
        pub need_scores: bool,
        pub field: String,
    }

    impl Collector for TermOrdCollector {
        type Fruit = Vec<super::SegmentResult>;
        type Child = TermOrdSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            let ff = segment_reader.fast_fields();
            Ok(TermOrdSegmentCollector {
                segment_ord: segment_local_id,
                results: Default::default(),
                ff: ff.str(&self.field)?.expect("ff should be a str field"),
                ctid_ff: FFType::new_ctid(ff),
            })
        }

        fn requires_scoring(&self) -> bool {
            self.need_scores
        }

        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            Ok(segment_fruits)
        }
    }

    pub struct TermOrdSegmentCollector {
        pub ff: StrColumn,
        pub results: Vec<(TermOrdinal, SearchIndexScore, DocAddress)>,
        pub segment_ord: SegmentOrdinal,
        ctid_ff: FFType,
    }

    impl SegmentCollector for TermOrdSegmentCollector {
        type Fruit = super::SegmentResult;

        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            let ctid = self.ctid_ff.as_u64(doc).expect("ctid should be present");
            let scored = SearchIndexScore::new(ctid, score);
            self.results.push((
                self.ff.term_ords(doc).next().unwrap_or(NULL_TERM_ORDINAL),
                scored,
                doc_address,
            ));
        }

        fn harvest(self) -> Self::Fruit {
            (self.ff, self.results)
        }
    }
}
