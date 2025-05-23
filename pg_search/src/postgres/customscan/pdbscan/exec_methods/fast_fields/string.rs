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

use crate::api::HashMap;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
use crate::postgres::customscan::pdbscan::exec_methods::fast_fields::FastFieldExecState;
use crate::postgres::customscan::pdbscan::exec_methods::{ExecMethod, ExecState};
use crate::postgres::customscan::pdbscan::is_block_all_visible;
use crate::postgres::customscan::pdbscan::parallel::checkout_segment;
use crate::postgres::customscan::pdbscan::scan_state::PdbScanState;
use crate::query::SearchQueryInput;
use parking_lot::Mutex;
use pgrx::itemptr::item_pointer_get_block_number;
use pgrx::pg_sys;
use pgrx::pg_sys::CustomScanState;
use rayon::prelude::*;
use std::collections::BTreeMap;
use tantivy::collector::Collector;
use tantivy::index::SegmentId;
use tantivy::query::Query;
use tantivy::{DocAddress, Executor, SegmentOrdinal};

pub struct StringFastFieldExecState {
    inner: FastFieldExecState,
    search_results: StringAggResults,
    field: String,
}

impl StringFastFieldExecState {
    pub fn new(field: String, which_fast_fields: Vec<WhichFastField>) -> Self {
        Self {
            inner: FastFieldExecState::new(which_fast_fields),
            search_results: StringAggResults::None,
            field,
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
                    state.search_query_input(),
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
            self.search_results =
                searcher.string_agg(state.need_scores(), state.search_query_input(), &self.field);
            self.inner.did_query = true;
            true
        }
    }

    fn internal_next(&mut self, state: &mut PdbScanState) -> ExecState {
        if matches!(self.search_results, StringAggResults::None) {
            return ExecState::Eof;
        }

        unsafe {
            // SAFETY:  .next() can't be called with self.search_results being set to Some(...)
            match self.search_results.next() {
                None => ExecState::Eof,
                Some((scored, doc_address, mut term)) => {
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

                        // Use the shared extract_data_from_fast_fields function
                        let tupdesc = self.inner.tupdesc.as_ref().unwrap();
                        crate::postgres::customscan::pdbscan::exec_methods::fast_fields::extract_data_from_fast_fields(
                            natts,
                            tupdesc,
                            &self.inner.which_fast_fields,
                            &mut self.inner.ffhelper,
                            slot,
                            scored,
                            doc_address,
                            &mut term,
                        );

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

type SearchResultsIter = std::vec::IntoIter<(SearchIndexScore, DocAddress)>;
type BatchedResultsIter = std::vec::IntoIter<(Option<String>, SearchResultsIter)>;
type MergedResultsMap = BTreeMap<Option<String>, Vec<(SearchIndexScore, DocAddress)>>;
#[derive(Default)]
enum StringAggResults {
    #[default]
    None,
    Batched {
        current: (Option<String>, SearchResultsIter),
        set: BatchedResultsIter,
    },
    SingleSegment(crossbeam::channel::IntoIter<(SearchIndexScore, DocAddress, Option<String>)>),
}

impl Iterator for StringAggResults {
    type Item = (SearchIndexScore, DocAddress, Option<String>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            StringAggResults::None => None,
            StringAggResults::Batched { current, set } => loop {
                if let Some(next) = current.1.next() {
                    return Some((next.0, next.1, current.0.clone()));
                } else if let Some(next_set) = set.next() {
                    *current = next_set;
                } else {
                    return None;
                }
            },
            StringAggResults::SingleSegment(iter) => iter.next(),
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
                        schema: &self.0.schema().schema,
                        searcher_opt: Some(self.0.searcher()),
                    }
                },
            )
            .expect("failed to search");

        let field = field.to_string();
        let searcher = self.0.searcher().clone();

        let merged: Mutex<MergedResultsMap> = Mutex::new(BTreeMap::new());

        results
            .into_par_iter()
            .for_each(|(str_ff, segment_results)| {
                let keys = segment_results.keys().cloned().collect::<Vec<_>>();
                let segment_values: Vec<_> = segment_results.into_iter().collect();
                let mut values_iter = segment_values.into_iter();
                let mut resolved = HashMap::default();
                str_ff
                    .dictionary()
                    .sorted_ords_to_term_cb(keys.into_iter(), |bytes| {
                        let term = std::str::from_utf8(bytes)
                            .expect("term should be valid utf8")
                            .to_string();

                        resolved.insert(
                            term,
                            values_iter.next().unwrap_or_else(|| panic!("internal error: don't have the same number of TermOrd keys and values")).1,
                        );

                        Ok(())
                    })
                    .expect("term ord resolution should succeed");

                let mut null_results = Vec::new();
                if str_ff.num_terms() == 0 {
                    if let Some(next_value) = values_iter.next() {
                        null_results.push(next_value);
                        // Collect any additional remaining values
                        null_results.extend(values_iter);
                    }
                }
                let mut guard = merged.lock();
                for (term, mut results) in resolved {
                    guard.entry(Some(term)).or_default().append(&mut results);
                }

                for (_, mut results) in null_results {
                    guard.entry(None).or_default().append(&mut results);
                }
            });

        let set = merged
            .into_inner()
            .into_iter()
            .map(|(term, docs)| (term, docs.into_iter()))
            .collect::<Vec<_>>()
            .into_iter();
        StringAggResults::Batched {
            current: (None, vec![].into_iter()),
            set,
        }
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
                    schema: &self.0.schema().schema,
                    searcher_opt: Some(self.0.searcher()),
                }
            })
            .expect("weight should be constructable");

        let (str_ff, results) = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("single segment collection should succeed");

        let field = field.to_string();
        let searcher = self.0.searcher().clone();
        let dictionary = str_ff.dictionary();
        let term_ords = results.keys().cloned().collect::<Vec<_>>();
        let mut results = results.into_iter();
        let (sender, receiver) = crossbeam::channel::unbounded();
        dictionary
            .sorted_ords_to_term_cb(term_ords.into_iter(), |bytes| {
                let term = std::str::from_utf8(bytes)
                    .expect("term should be valid utf8")
                    .to_string();

                let (_, values) = results.next().unwrap_or_else(|| {
                    panic!("internal error: don't have the same number of TermOrd keys and values")
                });
                for (scored, doc_address) in values {
                    sender
                        .send((scored, doc_address, Some(term.clone())))
                        .map_err(|_| {
                            std::io::Error::new(std::io::ErrorKind::Other, "failed to send term")
                        })?;
                }

                Ok(())
            })
            .expect("term ord lookup should succeed");

        if dictionary.num_terms() == 0 && results.len() > 0 {
            let (_, values) = results.next().unwrap_or_else(|| {
                panic!("internal error: don't have the same number of TermOrd keys and values")
            });
            for (scored, doc_address) in values {
                let _ = sender.send((scored, doc_address, None)).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::Other, "failed to send term")
                });
            }
        }

        StringAggResults::SingleSegment(receiver.into_iter())
    }
}

mod term_ord_collector {
    use crate::index::reader::index::SearchIndexScore;
    use std::collections::BTreeMap;
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::columnar::StrColumn;

    use crate::index::fast_fields_helper::FFType;
    use tantivy::termdict::TermOrdinal;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    pub struct TermOrdCollector {
        pub need_scores: bool,
        pub field: String,
    }

    impl Collector for TermOrdCollector {
        type Fruit = Vec<(
            StrColumn,
            BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>,
        )>;
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
        pub results: BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>,
        pub segment_ord: SegmentOrdinal,
        ctid_ff: FFType,
    }

    impl SegmentCollector for TermOrdSegmentCollector {
        type Fruit = (
            StrColumn,
            BTreeMap<TermOrdinal, Vec<(SearchIndexScore, DocAddress)>>,
        );

        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            let ctid = self.ctid_ff.as_u64(doc).expect("ctid should be present");
            let scored = SearchIndexScore::new(ctid, score);
            if let Some(term_ord) = self.ff.term_ords(doc).next() {
                self.results
                    .entry(term_ord)
                    .or_default()
                    .push((scored, doc_address));
            } else {
                self.results
                    .entry(0)
                    .or_default()
                    .push((scored, doc_address));
            }
        }

        fn harvest(self) -> Self::Fruit {
            (self.ff, self.results)
        }
    }
}
