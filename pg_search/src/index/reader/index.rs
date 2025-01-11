// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::index::fast_fields_helper::FFType;
use crate::index::merge_policy::AllowedMergePolicy;
use crate::index::{setup_tokenizers, BlockDirectoryType};
use crate::postgres::storage::block::CLEANUP_LOCK;
use crate::postgres::storage::buffer::{BufferManager, PinnedBuffer};
use crate::query::SearchQueryInput;
use crate::schema::SearchField;
use crate::schema::{SearchFieldName, SearchIndexSchema};
use anyhow::Result;
use pgrx::{pg_sys, PgRelation};
use rustc_hash::FxHashMap;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::iter::Flatten;
use std::path::PathBuf;
use std::sync::Arc;
use tantivy::collector::{Collector, TopDocs};
use tantivy::index::Index;
use tantivy::query::{EnableScoring, QueryClone, QueryParser, Weight};
use tantivy::schema::FieldType;
use tantivy::termdict::TermOrdinal;
use tantivy::{
    query::Query, DocAddress, DocId, IndexReader, Order, ReloadPolicy, Score, Searcher,
    SegmentOrdinal, SegmentReader, TantivyDocument, TantivyError,
};
use tantivy::{snippet::SnippetGenerator, Executor};

/// Represents a matching document from a tantivy search.  Typically it is returned as an Iterator
/// Item alongside the originating tantivy [`DocAddress`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchIndexScore {
    pub ctid: u64,
    pub bm25: f32,
}

impl SearchIndexScore {
    #[inline]
    pub fn new(ctid: u64, score: Score) -> Self {
        Self { ctid, bm25: score }
    }
}

impl PartialOrd for SearchIndexScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.bm25.partial_cmp(&other.bm25)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
    None,
}

impl From<SortDirection> for tantivy::Order {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Asc => Order::Asc,
            SortDirection::Desc => Order::Desc,
            SortDirection::None => Order::Asc,
        }
    }
}

/// An iterator of the different styles of search results we can return
#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum SearchResults {
    #[default]
    None,
    TopNByScore(
        usize,
        Searcher,
        FxHashMap<SegmentOrdinal, FFType>,
        std::vec::IntoIter<(Score, DocAddress)>,
    ),
    TopNByTweakedScore(
        usize,
        Searcher,
        FxHashMap<SegmentOrdinal, FFType>,
        std::vec::IntoIter<(TweakedScore, DocAddress)>,
    ),
    TopNByField(
        usize,
        Searcher,
        FxHashMap<SegmentOrdinal, FFType>,
        std::vec::IntoIter<(TermOrdinal, DocAddress)>,
    ),
    SingleSegment(
        usize,
        Searcher,
        FxHashMap<SegmentOrdinal, FFType>,
        vec_collector::FruitStyle,
    ),
    AllSegments(
        usize,
        Searcher,
        FxHashMap<SegmentOrdinal, FFType>,
        Flatten<std::vec::IntoIter<vec_collector::FruitStyle>>,
    ),
}

#[derive(PartialEq, Clone)]
pub struct TweakedScore {
    dir: SortDirection,
    score: Score,
}

impl PartialOrd for TweakedScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let cmp = self.score.partial_cmp(&other.score);
        match self.dir {
            SortDirection::Desc => cmp,
            SortDirection::Asc => cmp.map(|o| o.reverse()),
            SortDirection::None => Some(Ordering::Equal),
        }
    }
}

impl Iterator for SearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SearchResults::None => None,
            SearchResults::TopNByScore(_, searcher, ff_lookup, iter) => {
                let (score, doc_address) = iter.next()?;

                let ctid_ff = ff_lookup.entry(doc_address.segment_ord).or_insert_with(|| {
                    FFType::new(
                        searcher
                            .segment_reader(doc_address.segment_ord)
                            .fast_fields(),
                        "ctid",
                    )
                });
                let scored = SearchIndexScore {
                    ctid: ctid_ff
                        .as_u64(doc_address.doc_id)
                        .expect("ctid should be present"),
                    bm25: score,
                };
                Some((scored, doc_address))
            }
            SearchResults::TopNByTweakedScore(_, searcher, ff_lookup, iter) => {
                let (scored, doc_address) = iter.next()?;

                let ctid_ff = ff_lookup.entry(doc_address.segment_ord).or_insert_with(|| {
                    FFType::new(
                        searcher
                            .segment_reader(doc_address.segment_ord)
                            .fast_fields(),
                        "ctid",
                    )
                });
                let scored = SearchIndexScore {
                    ctid: ctid_ff
                        .as_u64(doc_address.doc_id)
                        .expect("ctid should be present"),
                    bm25: scored.score,
                };
                Some((scored, doc_address))
            }
            SearchResults::TopNByField(_, searcher, ff_lookup, iter) => {
                let (_, doc_address) = iter.next()?;
                let ctid_ff = ff_lookup.entry(doc_address.segment_ord).or_insert_with(|| {
                    FFType::new(
                        searcher
                            .segment_reader(doc_address.segment_ord)
                            .fast_fields(),
                        "ctid",
                    )
                });
                let scored = SearchIndexScore {
                    ctid: ctid_ff
                        .as_u64(doc_address.doc_id)
                        .expect("ctid should be present"),
                    bm25: 1.0,
                };
                Some((scored, doc_address))
            }
            SearchResults::SingleSegment(_, searcher, ff_lookup, iter) => {
                let (score, doc_address) = iter.next()?;

                let ctid_ff = ff_lookup.entry(doc_address.segment_ord).or_insert_with(|| {
                    FFType::new(
                        searcher
                            .segment_reader(doc_address.segment_ord)
                            .fast_fields(),
                        "ctid",
                    )
                });
                let scored = SearchIndexScore {
                    ctid: ctid_ff
                        .as_u64(doc_address.doc_id)
                        .expect("ctid should be present"),
                    bm25: score,
                };
                Some((scored, doc_address))
            }
            SearchResults::AllSegments(_, searcher, ff_lookup, iter) => {
                let (score, doc_address) = iter.next()?;

                let ctid_ff = ff_lookup.entry(doc_address.segment_ord).or_insert_with(|| {
                    FFType::new(
                        searcher
                            .segment_reader(doc_address.segment_ord)
                            .fast_fields(),
                        "ctid",
                    )
                });
                let scored = SearchIndexScore {
                    ctid: ctid_ff
                        .as_u64(doc_address.doc_id)
                        .expect("ctid should be present"),
                    bm25: score,
                };
                Some((scored, doc_address))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SearchResults::None => (0, Some(0)),
            SearchResults::TopNByScore(_, _, _, iter) => iter.size_hint(),
            SearchResults::TopNByTweakedScore(_, _, _, iter) => iter.size_hint(),
            SearchResults::TopNByField(_, _, _, iter) => iter.size_hint(),
            SearchResults::SingleSegment(_, _, _, iter) => iter.size_hint(),
            SearchResults::AllSegments(_, _, _, iter) => iter.size_hint(),
        }
    }
}

impl SearchResults {
    pub fn len(&self) -> Option<usize> {
        match self {
            SearchResults::None => Some(0),
            SearchResults::TopNByScore(count, ..) => Some(*count),
            SearchResults::TopNByTweakedScore(count, ..) => Some(*count),
            SearchResults::TopNByField(count, ..) => Some(*count),
            SearchResults::SingleSegment(count, ..) => Some(*count),
            SearchResults::AllSegments(count, ..) => Some(*count),
        }
    }
}

#[derive(Clone)]
pub struct SearchIndexReader {
    index_oid: pg_sys::Oid,
    searcher: Searcher,
    schema: SearchIndexSchema,
    underlying_reader: IndexReader,
    underlying_index: Index,

    // [`PinnedBuffer`] has a Drop impl, so we hold onto it but don't otherwise use it
    //
    // also, it's an Arc b/c if we're clone'd (we do derive it, after all), we only want this
    // buffer dropped once
    _cleanup_lock: Arc<Option<PinnedBuffer>>,
}

impl SearchIndexReader {
    pub fn open(
        index_relation: &PgRelation,
        directory_type: BlockDirectoryType,
        needs_cleanup_lock: bool,
    ) -> Result<Self> {
        // It is possible for index only scans and custom scans, which only check the visibility map
        // and do not fetch tuples from the heap, to suffer from the concurrent TID recycling problem.
        // This problem occurs due to a race condition: after vacuum is called, a concurrent index only or custom scan
        // reads in some dead ctids. ambulkdelete finishes immediately after, and Postgres updates its visibility map,
        //rendering those dead ctids visible. The concurrent scan then returns the wrong results.
        // To prevent this, ambulkdelete acquires an exclusive cleanup lock. Readers must also acquire this lock (shared)
        // to prevent a reader from reading dead ctids right before ambulkdelete finishes.
        //
        // It's sufficient, and **required** for parallel scans to operate correctly, for us to hold onto
        // a pinned but unlocked buffer.
        let cleanup_lock = if needs_cleanup_lock {
            let bman = BufferManager::new(index_relation.oid());
            Some(bman.get_buffer(CLEANUP_LOCK).unlock())
        } else {
            None
        };

        let directory = directory_type.directory(index_relation, AllowedMergePolicy::None);
        let mut index = Index::open(directory)?;
        let schema = SearchIndexSchema::open(index.schema(), index_relation);

        setup_tokenizers(&mut index, index_relation);
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();

        Ok(Self {
            index_oid: index_relation.oid(),
            searcher,
            schema,
            underlying_reader: reader,
            underlying_index: index,
            _cleanup_lock: Arc::new(cleanup_lock),
        })
    }

    pub fn key_field(&self) -> SearchField {
        self.schema.key_field()
    }

    pub fn query(&self, search_query_input: &SearchQueryInput) -> Box<dyn Query> {
        let mut parser = QueryParser::for_index(
            &self.underlying_index,
            self.schema
                .fields
                .iter()
                .map(|search_field| search_field.id.0)
                .collect::<Vec<_>>(),
        );
        search_query_input
            .clone()
            .into_tantivy_query(
                &(
                    unsafe { &PgRelation::with_lock(self.index_oid, pg_sys::AccessShareLock as _) },
                    &self.schema,
                ),
                &mut parser,
                &self.searcher,
            )
            .expect("must be able to parse query")
    }

    pub fn get_doc(&self, doc_address: DocAddress) -> tantivy::Result<TantivyDocument> {
        self.searcher.doc(doc_address)
    }

    /// Returns the index size, in bytes, according to tantivy
    pub fn byte_size(&self) -> Result<u64> {
        Ok(self
            .underlying_reader
            .searcher()
            .space_usage()
            .map(|space| space.total().get_bytes())?)
    }

    pub fn segment_readers(&self) -> &[SegmentReader] {
        self.searcher.segment_readers()
    }

    pub fn schema(&self) -> &SearchIndexSchema {
        &self.schema
    }

    pub fn searcher(&self) -> &Searcher {
        &self.searcher
    }

    pub fn validate_checksum(&self) -> Result<HashSet<PathBuf>> {
        Ok(self.underlying_index.validate_checksum()?)
    }

    pub fn snippet_generator(
        &self,
        field_name: &str,
        query: &SearchQueryInput,
    ) -> (tantivy::schema::Field, SnippetGenerator) {
        let field = self
            .schema
            .get_search_field(&SearchFieldName(field_name.into()))
            .expect("cannot generate snippet, field does not exist");

        match self.schema.schema.get_field_entry(field.into()).field_type() {
            FieldType::Str(_) => {
                let field:tantivy::schema::Field = field.into();
                let generator = SnippetGenerator::create(&self.searcher, &self.query(query), field)
                    .unwrap_or_else(|err| panic!("failed to create snippet generator for field: {field_name}... {err}"));
                (field, generator)
            }
            _ => panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        }
    }

    /// Search the Tantivy index for matching documents.
    ///
    /// The order of returned docs is unspecified.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search(
        &self,
        need_scores: bool,
        _sort_segments_by_ctid: bool,
        query: &SearchQueryInput,
        _estimated_rows: Option<usize>,
    ) -> SearchResults {
        let collector = vec_collector::VecCollector::new(None, need_scores);
        let results = self.collect(query, collector, need_scores);
        let len = results.iter().map(|iter| iter.len()).sum();
        SearchResults::AllSegments(
            len,
            self.searcher.clone(),
            Default::default(),
            results.into_iter().flatten(),
        )
    }

    /// Search a specific index segment for matching documents.
    ///
    /// The order of returned docs is unspecified.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_segment(
        &self,
        need_scores: bool,
        segment_ord: SegmentOrdinal,
        query: &SearchQueryInput,
    ) -> SearchResults {
        let collector = vec_collector::VecCollector::new(None, need_scores);
        let weight = self
            .query(query)
            .weight(if need_scores {
                tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                }
            } else {
                tantivy::query::EnableScoring::Disabled {
                    schema: &self.schema.schema,
                    searcher_opt: Some(&self.searcher),
                }
            })
            .expect("weight should be constructable");
        let segment_reader = self.searcher.segment_reader(segment_ord);
        let results = collector
            .collect_segment(weight.as_ref(), segment_ord, segment_reader)
            .expect("single segment collection should succeed");
        SearchResults::SingleSegment(
            results.len(),
            self.searcher.clone(),
            Default::default(),
            results.into_iter(),
        )
    }

    /// Search the Tantivy index for the "top N" matching documents.
    ///
    /// The documents are returned in score order.  Most relevant first if `sortdir` is [`SortDirection::Desc`],
    /// or least relevant first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_top_n(
        &self,
        query: &SearchQueryInput,
        sort_field: Option<String>,
        sortdir: SortDirection,
        n: usize,
        need_scores: bool,
    ) -> SearchResults {
        if let Some(sort_field) = sort_field {
            self.top_by_field(query, sort_field, sortdir, n)
        } else {
            self.top_by_score(query, sortdir, n, need_scores)
        }
    }

    /// Search the Tantivy index for the "top N" matching documents in a specific segment.
    ///
    /// The documents are returned in score order.  Most relevant first if `sortdir` is [`SortDirection::Desc`],
    /// or least relevant first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_top_n_in_segment(
        &self,
        segment_ord: SegmentOrdinal,
        query: &SearchQueryInput,
        sort_field: Option<String>,
        sortdir: SortDirection,
        n: usize,
        need_scores: bool,
    ) -> SearchResults {
        if let Some(sort_field) = sort_field {
            assert!(
                !need_scores,
                "cannot sort by field and get scores in the same query"
            );
            self.top_by_field_in_segment(segment_ord, query, sort_field, sortdir, n)
        } else {
            self.top_by_score_in_segment(segment_ord, query, sortdir, n, need_scores)
        }
    }

    fn top_by_field(
        &self,
        query: &SearchQueryInput,
        sort_field: String,
        sortdir: SortDirection,
        n: usize,
    ) -> SearchResults {
        let sort_field = self
            .schema
            .get_search_field(&SearchFieldName(sort_field.clone()))
            .expect("sort field should exist in index schema");

        let collector =
            TopDocs::with_limit(n).order_by_u64_field(sort_field.name.0.clone(), sortdir.into());
        let top_docs = self.collect(query, collector, true);
        SearchResults::TopNByField(
            top_docs.len(),
            self.searcher.clone(),
            Default::default(),
            top_docs.into_iter(),
        )
    }

    fn top_by_score(
        &self,
        query: &SearchQueryInput,
        sortdir: SortDirection,
        n: usize,
        need_scores: bool,
    ) -> SearchResults {
        match sortdir {
            // requires tweaking the score, which is a bit slower
            SortDirection::Asc => {
                let collector = TopDocs::with_limit(n).tweak_score(
                    move |_segment_reader: &tantivy::SegmentReader| {
                        move |_doc: DocId, original_score: Score| TweakedScore {
                            dir: sortdir,
                            score: original_score,
                        }
                    },
                );
                let top_docs = self.collect(query, collector, true);
                SearchResults::TopNByTweakedScore(
                    top_docs.len(),
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            // can use tantivy's score directly
            SortDirection::Desc => {
                let collector = TopDocs::with_limit(n);
                let top_docs = self.collect(query, collector, true);
                SearchResults::TopNByScore(
                    top_docs.len(),
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            SortDirection::None => {
                let enabled_scoring = if need_scores {
                    tantivy::query::EnableScoring::Enabled {
                        searcher: &self.searcher,
                        statistics_provider: &self.searcher,
                    }
                } else {
                    tantivy::query::EnableScoring::Disabled {
                        schema: self.searcher.schema(),
                        searcher_opt: Some(&self.searcher),
                    }
                };

                struct WeightLimitQuery {
                    query: Box<dyn Query>,
                    weight: limit::WeightLimit,
                }
                impl Debug for WeightLimitQuery {
                    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
                        Ok(())
                    }
                }

                impl QueryClone for WeightLimitQuery {
                    fn box_clone(&self) -> Box<dyn Query> {
                        Box::new(WeightLimitQuery {
                            query: self.query.box_clone(),
                            weight: self.weight.clone(),
                        })
                    }
                }

                impl Query for WeightLimitQuery {
                    fn weight(
                        &self,
                        _enable_scoring: EnableScoring<'_>,
                    ) -> tantivy::Result<Box<dyn Weight>> {
                        Ok(Box::new(self.weight.clone()))
                    }
                }

                let query = self.query(query);
                let weight = query
                    .weight(enabled_scoring)
                    .expect("creating a Weight from a Query should not fail");
                let query: Box<dyn Query> = Box::new(WeightLimitQuery {
                    query,
                    weight: limit::WeightLimit::new(n, weight),
                });
                let collector = vec_collector::VecCollector::new(Some(n), need_scores);
                let results = self
                    .searcher
                    .search_with_executor(
                        &query,
                        &collector,
                        &Executor::SingleThread,
                        enabled_scoring,
                    )
                    .expect("should be able to collect top-n");
                let len = results.iter().map(|iter| iter.len()).sum();
                SearchResults::AllSegments(
                    len,
                    self.searcher.clone(),
                    Default::default(),
                    results.into_iter().flatten(),
                )
            }
        }
    }

    /// Search the Tantivy index for the "top N" matching documents (ordered by a field) in a specific segment.
    ///
    /// The documents are returned in field order.  Largest first if `sortdir` is [`SortDirection::Desc`],
    /// or smallest first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    fn top_by_field_in_segment(
        &self,
        segment_ord: SegmentOrdinal,
        query: &SearchQueryInput,
        sort_field: String,
        sortdir: SortDirection,
        n: usize,
    ) -> SearchResults {
        let sort_field = self
            .schema
            .get_search_field(&SearchFieldName(sort_field.clone()))
            .expect("sort field should exist in index schema");

        let collector =
            TopDocs::with_limit(n).order_by_u64_field(sort_field.name.0.clone(), sortdir.into());
        let query = self.query(query);
        let weight = query
            .weight(tantivy::query::EnableScoring::Enabled {
                searcher: &self.searcher,
                statistics_provider: &self.searcher,
            })
            .expect("creating a Weight from a Query should not fail");
        let top_docs = collector
            .collect_segment(
                weight.as_ref(),
                segment_ord,
                self.searcher.segment_reader(segment_ord),
            )
            .expect("should be able to collect top-n in segment");
        let top_docs = collector
            .merge_fruits(vec![top_docs])
            .expect("should be able to merge top-n in segment");
        SearchResults::TopNByField(
            top_docs.len(),
            self.searcher.clone(),
            Default::default(),
            top_docs.into_iter(),
        )
    }

    /// Search the Tantivy index for the "top N" matching documents (ordered by score) in a specific segment.
    ///
    /// The documents are returned in score order.  Most relevant first if `sortdir` is [`SortDirection::Desc`],
    /// or least relevant first if it's [`SortDirection::Asc`].
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    fn top_by_score_in_segment(
        &self,
        segment_ord: SegmentOrdinal,
        query: &SearchQueryInput,
        sortdir: SortDirection,
        n: usize,
        need_scores: bool,
    ) -> SearchResults {
        let query = self.query(query);
        let weight = query
            .weight(tantivy::query::EnableScoring::Enabled {
                searcher: &self.searcher,
                statistics_provider: &self.searcher,
            })
            .expect("creating a Weight from a Query should not fail");

        match sortdir {
            // requires tweaking the score, which is a bit slower
            SortDirection::Asc => {
                let collector = TopDocs::with_limit(n).tweak_score(
                    move |_segment_reader: &tantivy::SegmentReader| {
                        move |_doc: DocId, original_score: Score| TweakedScore {
                            dir: sortdir,
                            score: original_score,
                        }
                    },
                );
                let top_docs = collector
                    .collect_segment(
                        weight.as_ref(),
                        segment_ord,
                        self.searcher.segment_reader(segment_ord),
                    )
                    .expect("should be able to collect top-n in segment");

                let top_docs = collector
                    .merge_fruits(vec![top_docs])
                    .expect("should be able to merge top-n in segment");

                SearchResults::TopNByTweakedScore(
                    top_docs.len(),
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            // can use tantivy's score directly
            SortDirection::Desc => {
                let collector = TopDocs::with_limit(n);
                let top_docs = collector
                    .collect_segment(
                        weight.as_ref(),
                        segment_ord,
                        self.searcher.segment_reader(segment_ord),
                    )
                    .expect("should be able to collect top-n in segment");

                let top_docs = collector
                    .merge_fruits(vec![top_docs])
                    .expect("should be able to merge top-n in segment");

                SearchResults::TopNByScore(
                    top_docs.len(),
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            SortDirection::None => {
                let collector = vec_collector::VecCollector::new(Some(n), need_scores);
                let weight = limit::WeightLimit::new(n, weight);
                let segment_reader = self.searcher.segment_reader(segment_ord);
                let results = collector
                    .collect_segment(&weight, segment_ord, segment_reader)
                    .expect("single segment collection should succeed");
                SearchResults::SingleSegment(
                    results.len(),
                    self.searcher.clone(),
                    Default::default(),
                    results.into_iter(),
                )
            }
        }
    }

    pub fn estimate_docs(&self, search_query_input: &SearchQueryInput) -> Option<usize> {
        let readers = self.searcher.segment_readers();
        let (ordinal, largest_reader) = readers
            .iter()
            .enumerate()
            .max_by_key(|(_, reader)| reader.num_docs())?;

        let collector = tantivy::collector::Count;
        let schema = self.schema.schema.clone();
        let query = self.query(search_query_input);
        let weight = match query.weight(tantivy::query::EnableScoring::Disabled {
            schema: &schema,
            searcher_opt: Some(&self.searcher),
        }) {
            // created the Weight, no problem
            Ok(weight) => weight,

            // got an error trying to create the weight.  This *likely* means
            // the query requires scoring, so try again with scoring enabled.
            // I've seen this with the `MoreLikeThis` query type.
            //
            // NB:  we could just return `None` here and let the caller deal with it?
            //      a deciding factor might be if users complain that query planning
            //      is too slow when they use constructs like `MoreLikeThis`
            Err(TantivyError::InvalidArgument(_)) => query
                .weight(tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                })
                .expect("creating a Weight from a Query should not fail"),

            // something completely unexpected happen, so just panic
            Err(e) => panic!("{:?}", e),
        };

        let count = collector
            .collect_segment(weight.as_ref(), ordinal as SegmentOrdinal, largest_reader)
            .expect("counting docs in the largest segment should not fail")
            .max(1); // want to assume at least 1 matching document
        let segment_doc_proportion =
            largest_reader.num_docs() as f64 / self.searcher.num_docs() as f64;

        Some((count as f64 / segment_doc_proportion).ceil() as usize)
    }

    pub fn collect<C: Collector + 'static>(
        &self,
        query: &SearchQueryInput,
        collector: C,
        need_scores: bool,
    ) -> C::Fruit {
        let owned_query = self.query(query);
        self.searcher
            .search_with_executor(
                &owned_query,
                &collector,
                &Executor::SingleThread,
                if need_scores {
                    tantivy::query::EnableScoring::Enabled {
                        searcher: &self.searcher,
                        statistics_provider: &self.searcher,
                    }
                } else {
                    tantivy::query::EnableScoring::Disabled {
                        schema: &self.schema.schema,
                        searcher_opt: Some(&self.searcher),
                    }
                },
            )
            .expect("search should not fail")
    }
}

mod limit {
    use std::sync::Arc;
    use tantivy::query::{Explanation, Scorer, Weight};
    use tantivy::{DocId, DocSet, Score, SegmentReader, TERMINATED};

    struct LimitingScorer {
        limit: u32,
        inner: Box<dyn Scorer>,
        cnt: u32,
    }

    impl DocSet for LimitingScorer {
        fn advance(&mut self) -> DocId {
            self.cnt += 1;
            if self.cnt >= self.limit {
                return TERMINATED;
            }
            self.inner.advance()
        }

        fn doc(&self) -> DocId {
            self.inner.doc()
        }

        fn size_hint(&self) -> u32 {
            self.limit
        }
    }

    impl Scorer for LimitingScorer {
        fn score(&mut self) -> Score {
            self.inner.score()
        }
    }

    #[derive(Clone)]
    pub struct WeightLimit {
        limit: u32,
        inner: Arc<Box<dyn Weight>>,
    }

    impl WeightLimit {
        pub fn new(limit: usize, inner: Box<dyn Weight>) -> Self {
            Self {
                limit: limit.try_into().expect("limit should not exceed u32::MAX"),
                inner: Arc::new(inner),
            }
        }
    }

    impl Weight for WeightLimit {
        fn scorer(&self, reader: &SegmentReader, boost: Score) -> tantivy::Result<Box<dyn Scorer>> {
            let scorer = self.inner.scorer(reader, boost)?;
            Ok(Box::new(LimitingScorer {
                limit: self.limit,
                inner: scorer,
                cnt: 0,
            }))
        }

        fn explain(&self, reader: &SegmentReader, doc: DocId) -> tantivy::Result<Explanation> {
            self.inner.explain(reader, doc)
        }
    }
}

mod vec_collector {

    use std::sync::atomic::{AtomicUsize, Ordering};

    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::query::Weight;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    #[derive(Default)]
    pub enum FruitStyle {
        #[default]
        Empty,
        Scored(
            SegmentOrdinal,
            std::vec::IntoIter<Score>,
            std::vec::IntoIter<DocId>,
        ),
        Blocks(
            usize,
            SegmentOrdinal,
            std::iter::Flatten<std::vec::IntoIter<std::vec::IntoIter<DocId>>>,
        ),
    }

    impl FruitStyle {
        pub fn len(&self) -> usize {
            match self {
                FruitStyle::Empty => 0,
                FruitStyle::Scored(_, iter, ..) => iter.len(),
                FruitStyle::Blocks(count, ..) => *count,
            }
        }
    }

    impl Iterator for FruitStyle {
        type Item = (Score, DocAddress);

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                FruitStyle::Empty => None,
                FruitStyle::Scored(segment_ord, score, doc) => {
                    let doc = doc.next()?;
                    let score = score.next()?;
                    let doc_address = DocAddress::new(*segment_ord, doc);
                    Some((score, doc_address))
                }
                FruitStyle::Blocks(_, segment_ord, doc) => {
                    let doc = doc.next()?;
                    Some((0.0, DocAddress::new(*segment_ord, doc)))
                }
            }
        }
    }

    /// A [`Collector`] that collects all matching documents into a [`Vec`].
    pub struct VecCollector {
        limit: Option<usize>,
        need_scores: bool,
        count: AtomicUsize,
    }

    impl VecCollector {
        pub fn new(limit: Option<usize>, need_scores: bool) -> Self {
            Self {
                limit,
                need_scores,
                count: Default::default(),
            }
        }
    }

    impl Collector for VecCollector {
        type Fruit = Vec<FruitStyle>;
        type Child = VecSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            _segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            Ok(VecSegmentCollector {
                segment_ord: segment_local_id,
                scored: (vec![], vec![]),
                doc_blocks: vec![],
            })
        }

        fn requires_scoring(&self) -> bool {
            self.need_scores
        }

        fn merge_fruits(
            &self,
            segment_fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
        ) -> tantivy::Result<Self::Fruit> {
            // NB:  we never call this function, but best to implement it anyways
            Ok(segment_fruits)
        }

        fn collect_segment(
            &self,
            weight: &dyn Weight,
            segment_ord: u32,
            reader: &SegmentReader,
        ) -> tantivy::Result<<Self::Child as SegmentCollector>::Fruit> {
            let mut segment_collector = self.for_segment(segment_ord, reader)?;

            if self.limit.is_none() || self.count.load(Ordering::Relaxed) < self.limit.unwrap_or(0)
            {
                match (reader.alive_bitset(), self.requires_scoring()) {
                    (Some(alive_bitset), true) => {
                        weight.for_each(reader, &mut |doc, score| {
                            if alive_bitset.is_alive(doc) {
                                self.count.fetch_add(1, Ordering::Relaxed);
                                segment_collector.collect(doc, score);
                            }
                        })?;
                    }
                    (Some(alive_bitset), false) => {
                        weight.for_each_no_score(reader, &mut |docs| {
                            for doc in docs.iter().cloned() {
                                if alive_bitset.is_alive(doc) {
                                    self.count.fetch_add(1, Ordering::Relaxed);
                                    segment_collector.collect(doc, 0.0);
                                }
                            }
                        })?;
                    }
                    (None, true) => {
                        weight.for_each(reader, &mut |doc, score| {
                            self.count.fetch_add(1, Ordering::Relaxed);
                            segment_collector.collect(doc, score);
                        })?;
                    }
                    (None, false) => {
                        weight.for_each_no_score(reader, &mut |docs| {
                            self.count.fetch_add(docs.len(), Ordering::Relaxed);
                            segment_collector.collect_block(docs);
                        })?;
                    }
                }
            }

            Ok(segment_collector.harvest())
        }
    }

    pub struct VecSegmentCollector {
        segment_ord: SegmentOrdinal,
        scored: (Vec<DocId>, Vec<Score>),
        doc_blocks: Vec<std::vec::IntoIter<DocId>>,
    }

    impl SegmentCollector for VecSegmentCollector {
        type Fruit = FruitStyle;

        fn collect(&mut self, doc: DocId, score: Score) {
            self.scored.0.push(doc);
            self.scored.1.push(score);
        }

        #[allow(clippy::unnecessary_to_owned)]
        fn collect_block(&mut self, docs: &[DocId]) {
            self.doc_blocks.push(docs.to_vec().into_iter());
        }

        fn harvest(mut self) -> Self::Fruit {
            if !self.doc_blocks.is_empty() {
                if !self.scored.0.is_empty() {
                    self.doc_blocks.push(self.scored.0.into_iter());
                }
                FruitStyle::Blocks(
                    self.doc_blocks.iter().map(|i| i.len()).sum(),
                    self.segment_ord,
                    self.doc_blocks.into_iter().flatten(),
                )
            } else {
                FruitStyle::Scored(
                    self.segment_ord,
                    self.scored.1.into_iter(),
                    self.scored.0.into_iter(),
                )
            }
        }
    }
}
