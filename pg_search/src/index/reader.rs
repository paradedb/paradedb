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

use super::score::SearchIndexScore;
use super::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::schema::{SearchConfig, SearchFieldName, SearchIndexSchema};
use anyhow::Result;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tantivy::collector::{Collector, TopDocs};
use tantivy::columnar::{ColumnValues, StrColumn};
use tantivy::fastfield::FastFieldReaders;
use tantivy::schema::{FieldType, Value};
use tantivy::{
    query::Query, DocAddress, DocId, Score, Searcher, SegmentOrdinal, TantivyDocument, TantivyError,
};
use tantivy::{snippet::SnippetGenerator, Executor};
use tracing::debug;

const CACHE_NUM_BLOCKS: usize = 10;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// An iterator of the different styles of search results we can return
#[derive(Default)]
pub enum SearchResults {
    #[default]
    None,
    AllFeatures(usize, std::vec::IntoIter<(SearchIndexScore, DocAddress)>),

    TopN(usize, std::vec::IntoIter<(SearchIndexScore, DocAddress)>),

    #[allow(clippy::type_complexity)]
    FastPath(
        std::iter::Flatten<
            crossbeam::channel::IntoIter<Vec<tantivy::Result<(SearchIndexScore, DocAddress)>>>,
        >,
    ),
}

impl Debug for SearchResults {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchResults::None => write!(f, "SearchResults::None"),
            SearchResults::AllFeatures(count, iter) => {
                write!(
                    f,
                    "SearchResults::AllFeatures({count}, {:?})",
                    iter.size_hint()
                )
            }
            SearchResults::TopN(count, iter) => {
                write!(f, "SearchResults::TopN({count}, {:?})", iter.size_hint())
            }
            SearchResults::FastPath(iter) => {
                write!(f, "SearchResults::FastPath({:?})", iter.size_hint())
            }
        }
    }
}

impl Iterator for SearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SearchResults::None => None,
            SearchResults::AllFeatures(_, iter) => iter.next(),
            SearchResults::TopN(_, iter) => iter.next(),
            SearchResults::FastPath(iter) => iter
                .next()
                .map(|result| result.unwrap_or_else(|e| panic!("{e}"))),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SearchResults::None => (0, Some(0)),
            SearchResults::AllFeatures(_, iter) => iter.size_hint(),
            SearchResults::TopN(_, iter) => iter.size_hint(),
            SearchResults::FastPath(iter) => iter.size_hint(),
        }
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        match self {
            SearchResults::None => 0,
            SearchResults::AllFeatures(count, _) => count,
            SearchResults::TopN(count, _) => count,
            SearchResults::FastPath(iter) => iter.count(),
        }
    }
}

impl SearchResults {
    pub fn len(&self) -> Option<usize> {
        match self {
            SearchResults::None => Some(0),
            SearchResults::AllFeatures(count, _) => Some(*count),
            SearchResults::TopN(count, _) => Some(*count),
            SearchResults::FastPath(_) => None,
        }
    }
}

#[derive(Clone)]
pub struct SearchIndexReader {
    pub searcher: Searcher,
    pub schema: SearchIndexSchema,
    pub underlying_reader: tantivy::IndexReader,
}

impl SearchIndexReader {
    pub fn new(search_index: &SearchIndex) -> Result<Self> {
        let schema = search_index.schema.clone();
        let reader = search_index
            .underlying_index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()?;
        let searcher = reader.searcher();
        Ok(SearchIndexReader {
            searcher,
            schema: schema.clone(),
            underlying_reader: reader,
        })
    }

    pub fn get_doc(&self, doc_address: DocAddress) -> tantivy::Result<TantivyDocument> {
        self.searcher.doc(doc_address)
    }

    /// Scan the index and use the provided callback to search for Documents with ctid
    /// values that need to be deleted.
    pub fn get_ctids_to_delete(
        &self,
        should_delete: impl Fn(u64) -> bool,
    ) -> Result<(Vec<u64>, u32)> {
        let mut not_deleted: u32 = 0;
        let mut ctids_to_delete: Vec<u64> = vec![];

        let ctid_field = self.schema.ctid_field().id.0;
        for segment_reader in self.searcher.segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for doc in store_reader.iter::<TantivyDocument>(segment_reader.alive_bitset()) {
                // if a document failed to deserialize, that's probably a hard error indicating the
                // index is corrupt.  So return that back to the caller immediately
                let doc = doc?;

                if let Some(ctid) = doc.get_first(ctid_field).and_then(|ctid| ctid.as_u64()) {
                    if should_delete(ctid) {
                        ctids_to_delete.push(ctid);
                    } else {
                        not_deleted += 1;
                    }
                } else {
                    // NB:  in a perfect world, this shouldn't happen.  But we did have a bug where
                    // the "ctid" field was not being `STORED`, which caused this
                    debug!(
                        "document `{doc:?}` in segment `{}` has no ctid",
                        segment_reader.segment_id()
                    );
                }
            }
        }

        Ok((ctids_to_delete, not_deleted))
    }

    /// Returns the index size, in bytes, according to tantivy
    pub fn byte_size(&self) -> Result<u64> {
        Ok(self
            .underlying_reader
            .searcher()
            .space_usage()
            .map(|space| space.total().get_bytes())?)
    }

    pub fn snippet_generator(&self, field_name: &str, query: &dyn Query) -> SnippetGenerator {
        let field = self
            .schema
            .get_search_field(&SearchFieldName(field_name.into()))
            .expect("cannot generate snippet, field does not exist");

        match self.schema.schema.get_field_entry(field.into()).field_type() {
            FieldType::Str(_) => {
                SnippetGenerator::create(&self.searcher, query, field.into())
                    .unwrap_or_else(|err| panic!("failed to create snippet generator for field: {field_name}... {err}"))
            }
            _ => panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        }
    }

    /// Search the Tantivy index for matching documents.
    ///
    /// This method will do the minimal amount of work necessary to return [`SearchResults`].  If,
    /// for example, it determines that scoring and sorting are not strictly necessary, it will
    /// use a "fast path" for searching where the returned [`SearchIndexScore`] will be minimally
    /// populated with only the "ctid" value for each matching document.
    ///
    /// The order of returned docs is unspecified here if there is no limit or orderby and stable_sort
    /// is false.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_minimal(
        &self,
        include_key: bool,
        executor: &'static Executor,
        config: &SearchConfig,
        query: &dyn Query,
    ) -> SearchResults {
        match (
            config.limit_rows,
            config.stable_sort.unwrap_or(true),
            config.order_by_field.clone(),
        ) {
            // no limit, no stable sorting, and no sort field
            //
            // this we can use a channel to stream the results and also elide doing key lookups.
            // this is our "fast path"
            (None, false, None) => SearchResults::FastPath(
                self.search_via_channel(executor, include_key, config, query)
                    .into_iter()
                    .flatten(),
            ),

            // at least one of limit, stable sorting, or a sort field, so we gotta do it all,
            // including retrieving the key field
            _ => {
                let results = self.search_with_top_docs(executor, true, config, query);
                SearchResults::AllFeatures(results.len(), results.into_iter())
            }
        }
    }

    pub fn search_top_n(
        &self,
        executor: &'static Executor,
        query: &dyn Query,
        sortdir: SortDirection,
        n: usize,
    ) -> SearchResults {
        #[derive(PartialEq, Clone)]
        struct OrderedScore {
            dir: SortDirection,
            score: Score,
        }

        impl PartialOrd for OrderedScore {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                let cmp = self.score.partial_cmp(&other.score);
                match self.dir {
                    SortDirection::Desc => cmp,
                    SortDirection::Asc => cmp.map(|o| o.reverse()),
                }
            }
        }

        let collector = TopDocs::with_limit(n).tweak_score(move |_: &tantivy::SegmentReader| {
            move |_: DocId, original_score: Score| OrderedScore {
                dir: sortdir,
                score: original_score,
            }
        });

        let results = self
            .searcher
            .search_with_executor(
                query,
                &collector,
                executor,
                tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                },
            )
            .expect("failed to search")
            .into_iter();

        let mut top_docs = Vec::with_capacity(results.len());
        for (OrderedScore { score, .. }, doc_address) in results {
            let segment_reader = self.searcher.segment_reader(doc_address.segment_ord);
            let fast_fields = segment_reader.fast_fields();
            let ctid_ff = FFType::new(fast_fields, "ctid");

            let ctid = ctid_ff
                .as_u64(doc_address.doc_id)
                .expect("DocId should have a ctid");

            let scored = SearchIndexScore {
                bm25: score,
                key: None,
                ctid,
                order_by: None,
                sort_asc: false,
            };

            top_docs.push((scored, doc_address));
        }

        SearchResults::TopN(top_docs.len(), top_docs.into_iter())
    }

    fn search_via_channel(
        &self,
        executor: &'static Executor,
        include_key: bool,
        config: &SearchConfig,
        query: &dyn Query,
    ) -> crossbeam::channel::Receiver<Vec<Result<(SearchIndexScore, DocAddress), TantivyError>>>
    {
        let (sender, receiver) = crossbeam::channel::unbounded();
        let collector =
            collector::ChannelCollector::new(sender, config.key_field.clone(), include_key);
        let searcher = self.searcher.clone();
        let schema = self.schema.schema.clone();
        let need_scores =
            config.need_scores || SearchConfig::contains_more_like_this(&config.query);

        let owned_query = query.box_clone();
        std::thread::spawn(move || {
            searcher
                .search_with_executor(
                    &owned_query,
                    &collector,
                    executor,
                    if need_scores {
                        tantivy::query::EnableScoring::Enabled {
                            searcher: &searcher,
                            statistics_provider: &searcher,
                        }
                    } else {
                        tantivy::query::EnableScoring::Disabled {
                            schema: &schema,
                            searcher_opt: Some(&searcher),
                        }
                    },
                )
                .expect("failed to search")
        });
        receiver
    }

    fn search_with_top_docs(
        &self,
        executor: &'static Executor,
        include_key: bool,
        config: &SearchConfig,
        query: &dyn Query,
    ) -> Vec<(SearchIndexScore, DocAddress)> {
        // Extract limit and offset from the query config or set defaults.
        let limit = config.limit_rows.unwrap_or_else(|| {
            // We use unwrap_or_else here so this block doesn't run unless
            // we actually need the default value. This is important, because there can
            // be some cost to Tantivy API calls.
            let num_docs = self.searcher.num_docs() as usize;
            if num_docs > 0 {
                num_docs // The collector will panic if it's passed a limit of 0.
            } else {
                1 // Since there's no docs to return anyways, just use 1.
            }
        });

        let offset = config.offset_rows.unwrap_or(0);
        let key_field_name = config.key_field.clone();
        let orderby_field = config.order_by_field.clone();
        let sort_asc = config.is_sort_ascending();

        let collector = TopDocs::with_limit(limit).and_offset(offset).tweak_score(
            move |segment_reader: &tantivy::SegmentReader| {
                let fast_fields = segment_reader.fast_fields();
                let ctid_ff = FFType::new(fast_fields, "ctid");
                let key_ff = include_key.then(|| FFType::new(fast_fields, key_field_name.as_str()));
                let orderby_ff = orderby_field
                    .as_ref()
                    .map(|name| FFType::new(fast_fields, name));

                move |doc: DocId, original_score: Score| SearchIndexScore {
                    bm25: original_score,
                    key: key_ff.as_ref().map(|key_ff| key_ff.value(doc)),
                    ctid: ctid_ff
                        .as_u64(doc)
                        .expect("expected the `ctid` field to be a u64"),

                    order_by: orderby_ff.as_ref().map(|fftype| fftype.value(doc)),
                    sort_asc,
                }
            },
        );

        self.searcher
            .search_with_executor(
                query,
                &collector,
                executor,
                tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                },
            )
            .expect("failed to search")
    }

    pub fn estimate_docs(&self, query: &dyn Query) -> Option<usize> {
        let readers = self.searcher.segment_readers();
        let (ordinal, largest_reader) = readers
            .iter()
            .enumerate()
            .max_by_key(|(_, reader)| reader.num_docs())?;

        let collector = tantivy::collector::Count;
        let schema = self.schema.schema.clone();
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
}

/// Helper for working with different "fast field" types as if they're all one type
enum FFType {
    Text(StrColumn),
    I64(Arc<dyn ColumnValues<i64>>),
    F64(Arc<dyn ColumnValues<f64>>),
    U64(Arc<dyn ColumnValues<u64>>),
    Bool(Arc<dyn ColumnValues<bool>>),
    Date(Arc<dyn ColumnValues<tantivy::DateTime>>),
}

impl FFType {
    /// Construct the proper [`FFType`] for the specified `field_name`, which
    /// should be a known field name in the Tantivy index
    fn new(ffr: &FastFieldReaders, field_name: &str) -> Self {
        if let Ok(Some(ff)) = ffr.str(field_name) {
            Self::Text(ff)
        } else if let Ok(ff) = ffr.u64(field_name) {
            Self::U64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ffr.i64(field_name) {
            Self::I64(ff.first_or_default_col(0))
        } else if let Ok(ff) = ffr.f64(field_name) {
            Self::F64(ff.first_or_default_col(0.0))
        } else if let Ok(ff) = ffr.bool(field_name) {
            Self::Bool(ff.first_or_default_col(false))
        } else if let Ok(ff) = ffr.date(field_name) {
            Self::Date(ff.first_or_default_col(tantivy::DateTime::MIN))
        } else {
            panic!("`{field_name}` is missing or is not configured as a fast field")
        }
    }

    /// Given a [`DocId`], what is its "fast field" value?
    #[inline(always)]
    fn value(&self, doc: DocId) -> TantivyValue {
        let value = match self {
            FFType::Text(ff) => {
                let mut s = String::new();
                let ord = ff
                    .term_ords(doc)
                    .next()
                    .expect("term ord should be retrievable");
                ff.ord_to_str(ord, &mut s)
                    .expect("string should be retrievable for term ord");
                TantivyValue(s.into())
            }
            FFType::I64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::F64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::U64(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::Bool(ff) => TantivyValue(ff.get_val(doc).into()),
            FFType::Date(ff) => TantivyValue(ff.get_val(doc).into()),
        };

        value
    }

    /// Given a [`DocId`], what is its "fast field" value?  In the case of a String field, we
    /// don't reconstruct the full string, and instead return the term ord as a u64
    #[inline(always)]
    #[allow(dead_code)]
    fn value_fast(&self, doc: DocId) -> TantivyValue {
        let value = match self {
            FFType::Text(ff) => {
                // just use the first term ord here.  that's enough to do a tie-break quickly
                let ord = ff
                    .term_ords(doc)
                    .next()
                    .expect("term ord should be retrievable");
                TantivyValue(ord.into())
            }
            other => other.value(doc),
        };

        value
    }

    /// Given a [`DocId`], what is its u64 "fast field" value?
    ///
    /// If this [`FFType`] isn't [`FFType::U64`], this function returns [`None`].
    #[inline(always)]
    fn as_u64(&self, doc: DocId) -> Option<u64> {
        if let FFType::U64(ff) = self {
            Some(ff.get_val(doc))
        } else {
            None
        }
    }
}

mod collector {
    use crate::index::reader::FFType;
    use crate::index::score::SearchIndexScore;
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    /// A [`Collector`] that uses a crossbeam channel to stream the results directly out of
    /// each segment, in parallel, as tantivy find each doc.
    pub struct ChannelCollector {
        sender: crossbeam::channel::Sender<Vec<tantivy::Result<(SearchIndexScore, DocAddress)>>>,
        key_field_name: String,
        include_key: bool,
    }

    impl ChannelCollector {
        pub fn new(
            sender: crossbeam::channel::Sender<
                Vec<tantivy::Result<(SearchIndexScore, DocAddress)>>,
            >,
            key_field_name: String,
            include_key: bool,
        ) -> Self {
            Self {
                sender,
                key_field_name,
                include_key,
            }
        }
    }

    impl Collector for ChannelCollector {
        type Fruit = ();
        type Child = ChannelSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            let fast_fields = segment_reader.fast_fields();
            let ctid_ff = FFType::new(fast_fields, "ctid");
            let key_ff = self
                .include_key
                .then(|| FFType::new(fast_fields, &self.key_field_name));
            Ok(ChannelSegmentCollector {
                segment_ord: segment_local_id,
                ctid_ff,
                key_ff,
                sender: self.sender.clone(),
                fruit: Vec::new(),
            })
        }

        fn requires_scoring(&self) -> bool {
            true
        }

        fn merge_fruits(&self, _segment_fruits: Vec<()>) -> tantivy::Result<Self::Fruit> {
            Ok(())
        }
    }

    pub struct ChannelSegmentCollector {
        segment_ord: SegmentOrdinal,
        ctid_ff: FFType,
        key_ff: Option<FFType>,
        sender: crossbeam::channel::Sender<Vec<tantivy::Result<(SearchIndexScore, DocAddress)>>>,
        fruit: Vec<tantivy::Result<(SearchIndexScore, DocAddress)>>,
    }

    impl SegmentCollector for ChannelSegmentCollector {
        type Fruit = ();

        fn collect(&mut self, doc: DocId, score: Score) {
            if let Some(ctid) = self.ctid_ff.as_u64(doc) {
                let key = self.key_ff.as_ref().map(|key_ff| key_ff.value(doc));

                let doc_address = DocAddress::new(self.segment_ord, doc);
                let scored = SearchIndexScore {
                    bm25: score,
                    key,
                    ctid,
                    order_by: None,
                    sort_asc: false,
                };

                self.fruit.push(Ok((scored, doc_address)))
            }
        }

        fn harvest(mut self) -> Self::Fruit {
            // ordering by ctid helps to avoid random heap access, at least for the docs that
            // were found in this segment.  But we don't need to do it if we're also retrieving
            // the "key_field".
            if self.key_ff.is_none() {
                self.fruit.sort_by_key(|result| {
                    result.as_ref().map(|(scored, _)| scored.ctid).unwrap_or(0)
                });
            }

            // if send fails that likely means the receiver was dropped so we have nowhere
            // to send the result.  That's okay
            self.sender.send(self.fruit).ok();
        }
    }
}
