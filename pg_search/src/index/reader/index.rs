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

use crate::index::directory::blocking::BlockingDirectory;
use crate::index::directory::channel::{
    ChannelDirectory, ChannelRequest, ChannelRequestHandler, ChannelResponse,
};
use crate::query::SearchQueryInput;
use crate::schema::{SearchField, SearchFieldName, SearchIndexSchema};
use anyhow::Result;
use pgrx::{pg_sys, PgRelation};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use tantivy::collector::{Collector, TopDocs};
use tantivy::fastfield::Column;
use tantivy::index::Index;
use tantivy::query::QueryParser;
use tantivy::schema::FieldType;
use tantivy::{
    query::Query, DocAddress, DocId, IndexReader, Order, Score, Searcher, SegmentOrdinal,
    TantivyDocument, TantivyError,
};
use tantivy::{snippet::SnippetGenerator, Executor};

use crate::postgres::storage::block::METADATA_BLOCKNO;
use crate::postgres::storage::utils::BM25BufferCache;

/// Represents a matching document from a tantivy search.  Typically it is returned as an Iterator
/// Item alongside the originating tantivy [`DocAddress`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchIndexScore {
    pub ctid: u64,
    pub bm25: f32,
}

impl SearchIndexScore {
    #[inline]
    pub fn new(ffcolumn: &Column<u64>, doc: DocId, score: Score) -> Self {
        Self {
            ctid: ffcolumn
                .first(doc)
                .expect("ctid should have a non-null value"),
            bm25: score,
        }
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
}

/// An iterator of the different styles of search results we can return
#[derive(Default)]
pub enum SearchResults {
    #[default]
    None,

    TopNByScore(usize, std::vec::IntoIter<(OrderedScore, DocAddress)>),

    TopNByField(usize, std::vec::IntoIter<(SearchIndexScore, DocAddress)>),

    #[allow(clippy::type_complexity)]
    BufferedChannel(
        std::iter::Flatten<crossbeam::channel::IntoIter<Vec<(SearchIndexScore, DocAddress)>>>,
    ),

    #[allow(clippy::type_complexity)]
    UnscoredBufferedChannel(
        crossbeam::channel::IntoIter<(SegmentOrdinal, Column<u64>, std::vec::IntoIter<DocId>)>,
        Option<(SegmentOrdinal, Column<u64>, std::vec::IntoIter<DocId>)>,
    ),

    Channel(crossbeam::channel::IntoIter<(SearchIndexScore, DocAddress)>),

    SingleSegment(vec_collector::FruitStyle),
}

#[derive(PartialEq, Clone)]
pub struct OrderedScore {
    dir: SortDirection,
    score: SearchIndexScore,
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

impl Debug for SearchResults {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchResults::None => write!(f, "SearchResults::None"),
            SearchResults::TopNByScore(count, iter) => {
                write!(f, "SearchResults::TopNByScore({count}, {:?})", iter.len())
            }
            SearchResults::TopNByField(count, iter) => {
                write!(f, "SearchResults::TopNByField({count}, {:?})", iter.len())
            }
            SearchResults::BufferedChannel(iter) => {
                write!(f, "SearchResults::BufferedChannel(~{:?})", iter.size_hint())
            }
            SearchResults::UnscoredBufferedChannel(iter, _) => {
                write!(
                    f,
                    "SearchResults::UnscoredBufferedChannel(~{:?})",
                    iter.size_hint()
                )
            }
            SearchResults::Channel(iter) => {
                write!(f, "SearchResults::Channel(~{:?})", iter.size_hint())
            }
            SearchResults::SingleSegment(iter) => {
                write!(f, "SearchResults::SingleSegment({:?})", iter.size_hint())
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
            SearchResults::TopNByScore(_, iter) => iter
                .next()
                .map(|(OrderedScore { score, .. }, doc_address)| (score, doc_address)),
            SearchResults::TopNByField(_, iter) => iter.next(),
            SearchResults::BufferedChannel(iter) => iter.next(),
            SearchResults::UnscoredBufferedChannel(iter, buffer) => loop {
                if buffer.is_none() {
                    *buffer = Some(iter.next()?);
                }
                let (segment_ord, ctid_ff, doc) = buffer.as_mut().unwrap();
                if let Some(doc) = doc.next() {
                    let doc_address = DocAddress::new(*segment_ord, doc);
                    let scored = SearchIndexScore::new(ctid_ff, doc, 0.0);
                    return Some((scored, doc_address));
                }
                *buffer = None;
            },
            SearchResults::Channel(iter) => iter.next(),
            SearchResults::SingleSegment(iter) => iter.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SearchResults::None => (0, Some(0)),
            SearchResults::TopNByScore(_, iter) => iter.size_hint(),
            SearchResults::TopNByField(_, iter) => iter.size_hint(),
            SearchResults::BufferedChannel(iter) => iter.size_hint(),
            SearchResults::UnscoredBufferedChannel(iter, _) => iter.size_hint(),
            SearchResults::Channel(iter) => iter.size_hint(),
            SearchResults::SingleSegment(iter) => iter.size_hint(),
        }
    }
}

impl SearchResults {
    pub fn len(&self) -> Option<usize> {
        match self {
            SearchResults::None => Some(0),
            SearchResults::TopNByScore(count, _) => Some(*count),
            SearchResults::TopNByField(count, _) => Some(*count),
            SearchResults::BufferedChannel(_) => None,
            SearchResults::UnscoredBufferedChannel(..) => None,
            SearchResults::Channel(_) => None,
            SearchResults::SingleSegment(_) => None,
        }
    }
}

#[derive(Clone)]
pub struct SearchIndexReader {
    pub index_oid: pg_sys::Oid,
    pub searcher: Searcher,
    pub schema: SearchIndexSchema,
    pub underlying_reader: IndexReader,
    pub underlying_index: Index,
}

impl SearchIndexReader {
    pub fn new(
        index_relation: &PgRelation,
        index: Index,
        searcher: Searcher,
        reader: IndexReader,
        schema: SearchIndexSchema,
    ) -> Self {
        Self {
            // NB:  holds the relation open with an AccessShareLock, which seems appropriate
            index_oid: index_relation.oid(),
            searcher,
            schema,
            underlying_reader: reader,
            underlying_index: index,
        }
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

    pub fn snippet_generator(
        &self,
        field_name: &str,
        query: &SearchQueryInput,
    ) -> SnippetGenerator {
        let field = self
            .schema
            .get_search_field(&SearchFieldName(field_name.into()))
            .expect("cannot generate snippet, field does not exist");

        match self.schema.schema.get_field_entry(field.into()).field_type() {
            FieldType::Str(_) => {
                SnippetGenerator::create(&self.searcher, &self.query(query), field.into())
                    .unwrap_or_else(|err| panic!("failed to create snippet generator for field: {field_name}... {err}"))
            }
            _ => panic!("failed to create snippet generator for field: {field_name}... can only highlight text fields")
        }
    }

    /// Search the Tantivy index for matching documents, in the background, streaming the matching
    /// documents back as they're found.
    ///
    /// The order of returned docs is unspecified.
    ///
    /// It has no understanding of Postgres MVCC visibility.  It is the caller's responsibility to
    /// handle that, if it's necessary.
    pub fn search_via_channel(
        &self,
        need_scores: bool,
        _sort_segments_by_ctid: bool,
        query: &SearchQueryInput,
        _estimated_rows: Option<usize>,
    ) -> SearchResults {
        // let estimated_rows = estimated_rows.unwrap_or(0);
        //
        // if estimated_rows == 0 || estimated_rows > 5_000 || sort_segments_by_ctid {
        //     if need_scores {
        //         let (sender, receiver) = crossbeam::channel::bounded(
        //             std::thread::available_parallelism().unwrap().get(),
        //         );
        //         let collector = buffered_channel::BufferedChannelCollector::new(
        //             need_scores,
        //             sort_segments_by_ctid,
        //             sender,
        //         );
        //         let searcher = self.searcher.clone();
        //         let owned_query = query.box_clone();
        //         std::thread::spawn(move || {
        //             searcher
        //                 .search_with_executor(
        //                     &owned_query,
        //                     &collector,
        //                     executor,
        //                     tantivy::query::EnableScoring::Enabled {
        //                         searcher: &searcher,
        //                         statistics_provider: &searcher,
        //                     },
        //                 )
        //                 .expect("failed to search")
        //         });
        //
        //         SearchResults::BufferedChannel(receiver.into_iter().flatten())
        //     } else {
        //         let (sender, receiver) = crossbeam::channel::unbounded();
        //         let collector =
        //             unscored_buffered_channel::UnscoredBufferedChannelCollector::new(sender);
        //         let searcher = self.searcher.clone();
        //         let schema = self.schema.schema.clone();
        //
        //         let owned_query = query.box_clone();
        //         std::thread::spawn(move || {
        //             searcher
        //                 .search_with_executor(
        //                     &owned_query,
        //                     &collector,
        //                     executor,
        //                     tantivy::query::EnableScoring::Disabled {
        //                         schema: &schema,
        //                         searcher_opt: Some(&searcher),
        //                     },
        //                 )
        //                 .expect("failed to search")
        //         });
        //
        //         SearchResults::UnscoredBufferedChannel(receiver.into_iter(), None)
        //     }
        // } else {
        //     let (sender, receiver) = crossbeam::channel::unbounded();
        //     let collector = channel::ChannelCollector::new(need_scores, sender);
        //     let searcher = self.searcher.clone();
        //     let schema = self.schema.schema.clone();
        //
        //     let owned_query = query.box_clone();
        //     std::thread::spawn(move || {
        //         searcher
        //             .search_with_executor(
        //                 &owned_query,
        //                 &collector,
        //                 executor,
        //                 if need_scores {
        //                     tantivy::query::EnableScoring::Enabled {
        //                         searcher: &searcher,
        //                         statistics_provider: &searcher,
        //                     }
        //                 } else {
        //                     tantivy::query::EnableScoring::Disabled {
        //                         schema: &schema,
        //                         searcher_opt: Some(&searcher),
        //                     }
        //                 },
        //             )
        //             .expect("failed to search")
        //     });
        //
        //     SearchResults::Channel(receiver.into_iter())
        // }

        let cache = unsafe { BM25BufferCache::open(self.index_oid) };
        let lock = unsafe { cache.get_buffer(METADATA_BLOCKNO, Some(pg_sys::BUFFER_LOCK_SHARE)) };

        let (search_sender, search_receiver) = crossbeam::channel::unbounded();
        let (request_sender, request_receiver) = crossbeam::channel::unbounded::<ChannelRequest>();
        let (response_sender, response_receiver) =
            crossbeam::channel::unbounded::<ChannelResponse>();

        let collector = channel::ChannelCollector::new(need_scores, search_sender);

        let owned_query = self.query(&query);
        std::thread::spawn(move || {
            let channel_directory =
                ChannelDirectory::new(request_sender.clone(), response_receiver.clone());
            let channel_index = Index::open(channel_directory).expect("channel index should open");
            let reader = channel_index
                .reader_builder()
                .reload_policy(tantivy::ReloadPolicy::Manual)
                .try_into()
                .unwrap();
            let searcher = reader.searcher();
            let schema = channel_index.schema();

            searcher
                .search_with_executor(
                    &owned_query,
                    &collector,
                    &Executor::SingleThread,
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
                .expect("failed to search");

            request_sender.send(ChannelRequest::Terminate).unwrap();
        });

        let blocking_directory = BlockingDirectory::new(self.index_oid);
        let mut handler = ChannelRequestHandler::open(
            blocking_directory,
            self.index_oid,
            response_sender,
            request_receiver,
        );
        let _ = handler.receive_blocking(Some(|_| false)).unwrap();

        unsafe { pg_sys::UnlockReleaseBuffer(lock) };
        SearchResults::Channel(search_receiver.into_iter())
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
        let collector = vec_collector::VecCollector::new(need_scores);
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
        SearchResults::SingleSegment(results.into_iter())
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
    ) -> SearchResults {
        if let Some(sort_field) = sort_field {
            self.top_by_field(query, sort_field, sortdir, n)
        } else {
            self.top_by_score(query, sortdir, n)
        }
    }

    fn top_by_field(
        &self,
        query: &SearchQueryInput,
        sort_field: String,
        sortdir: SortDirection,
        n: usize,
    ) -> SearchResults {
        impl From<SortDirection> for tantivy::Order {
            fn from(value: SortDirection) -> Self {
                match value {
                    SortDirection::Asc => Order::Asc,
                    SortDirection::Desc => Order::Desc,
                }
            }
        }

        let sort_field = self
            .schema
            .get_search_field(&SearchFieldName(sort_field.clone()))
            .expect("sort field should exist in index schema");

        let collector =
            TopDocs::with_limit(n).order_by_u64_field(&sort_field.name.0, sortdir.into());
        let top_docs = self
            .searcher
            .search_with_executor(
                &self.query(query),
                &collector,
                &Executor::SingleThread,
                tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                },
            )
            .expect("failed to search");

        let top_docs = top_docs
            .into_iter()
            .map(|(_, doc_address)| {
                let ctid = self
                    .searcher
                    .segment_reader(doc_address.segment_ord)
                    .fast_fields()
                    .u64("ctid")
                    .expect("ctid should be a fast field");
                (
                    SearchIndexScore::new(&ctid, doc_address.doc_id, 1.0),
                    doc_address,
                )
            })
            .collect::<Vec<_>>();

        SearchResults::TopNByField(top_docs.len(), top_docs.into_iter())
    }

    fn top_by_score(
        &self,
        query: &SearchQueryInput,
        sortdir: SortDirection,
        n: usize,
    ) -> SearchResults {
        let collector =
            TopDocs::with_limit(n).tweak_score(move |segment_reader: &tantivy::SegmentReader| {
                let ctid_ff = segment_reader
                    .fast_fields()
                    .u64("ctid")
                    .expect("ctid should be a fast field");

                move |doc: DocId, original_score: Score| OrderedScore {
                    dir: sortdir,
                    score: SearchIndexScore::new(&ctid_ff, doc, original_score),
                }
            });

        let top_docs = self
            .searcher
            .search_with_executor(
                &self.query(query),
                &collector,
                &Executor::SingleThread,
                tantivy::query::EnableScoring::Enabled {
                    searcher: &self.searcher,
                    statistics_provider: &self.searcher,
                },
            )
            .expect("failed to search")
            .into_iter();

        SearchResults::TopNByScore(top_docs.len(), top_docs.into_iter())
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
}

#[allow(dead_code)]
mod buffered_channel {
    use crate::index::reader::index::SearchIndexScore;
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::fastfield::Column;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    /// A [`Collector`] that uses a crossbeam channel to stream the results directly out of
    /// each segment, in parallel, as tantivy find each doc.
    pub struct BufferedChannelCollector {
        need_scores: bool,
        sender: crossbeam::channel::Sender<Vec<(SearchIndexScore, DocAddress)>>,
        sort_segments_by_ctid: bool,
    }

    impl BufferedChannelCollector {
        pub fn new(
            need_scores: bool,
            sort_segments_by_ctid: bool,
            sender: crossbeam::channel::Sender<Vec<(SearchIndexScore, DocAddress)>>,
        ) -> Self {
            Self {
                need_scores,
                sender,
                sort_segments_by_ctid,
            }
        }
    }

    impl Collector for BufferedChannelCollector {
        type Fruit = ();
        type Child = BufferedChannelSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            Ok(BufferedChannelSegmentCollector {
                segment_ord: segment_local_id,
                sender: self.sender.clone(),
                fruit: Vec::new(),
                ctid_ff: segment_reader
                    .fast_fields()
                    .u64("ctid")
                    .expect("ctid should be a u64 fast field"),
                sort_by_ctid: self.sort_segments_by_ctid,
            })
        }

        fn requires_scoring(&self) -> bool {
            self.need_scores
        }

        fn merge_fruits(&self, _segment_fruits: Vec<()>) -> tantivy::Result<Self::Fruit> {
            Ok(())
        }
    }

    pub struct BufferedChannelSegmentCollector {
        segment_ord: SegmentOrdinal,
        sender: crossbeam::channel::Sender<Vec<(SearchIndexScore, DocAddress)>>,
        fruit: Vec<(SearchIndexScore, DocAddress)>,
        ctid_ff: Column<u64>,
        sort_by_ctid: bool,
    }

    impl SegmentCollector for BufferedChannelSegmentCollector {
        type Fruit = ();

        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            self.fruit.push((
                SearchIndexScore::new(&self.ctid_ff, doc, score),
                doc_address,
            ));
        }

        fn collect_block(&mut self, docs: &[DocId]) {
            let mut collected = docs
                .iter()
                .map(|doc: &DocId| {
                    let doc = *doc;
                    let doc_address = DocAddress::new(self.segment_ord, doc);
                    (SearchIndexScore::new(&self.ctid_ff, doc, 0.0), doc_address)
                })
                .collect::<Vec<_>>();
            if self.sort_by_ctid {
                collected.sort_unstable_by_key(|(scored, _)| scored.ctid);
            }

            // send the block over the channel right now
            // if send fails that likely means the receiver was dropped so we have nowhere
            // to send the result.  That's okay
            self.sender.send(collected).ok();
        }

        fn harvest(self) -> Self::Fruit {
            let mut fruit = self.fruit;
            if !fruit.is_empty() {
                if self.sort_by_ctid {
                    fruit.sort_unstable_by_key(|(scored, _)| scored.ctid);
                }

                // if send fails that likely means the receiver was dropped so we have nowhere
                // to send the result.  That's okay
                self.sender.send(fruit).ok();
            }
        }
    }
}

#[allow(dead_code)]
mod unscored_buffered_channel {
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::fastfield::Column;
    use tantivy::{DocId, Score, SegmentOrdinal, SegmentReader};

    /// A [`Collector`] that uses a crossbeam channel to stream the results directly out of
    /// each segment, in parallel, as tantivy find each doc.
    pub struct UnscoredBufferedChannelCollector {
        sender:
            crossbeam::channel::Sender<(SegmentOrdinal, Column<u64>, std::vec::IntoIter<DocId>)>,
    }

    impl UnscoredBufferedChannelCollector {
        pub fn new(
            sender: crossbeam::channel::Sender<(
                SegmentOrdinal,
                Column<u64>,
                std::vec::IntoIter<DocId>,
            )>,
        ) -> Self {
            Self { sender }
        }
    }

    impl Collector for UnscoredBufferedChannelCollector {
        type Fruit = ();
        type Child = UnscoredBufferedChannelSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            Ok(UnscoredBufferedChannelSegmentCollector {
                segment_ord: segment_local_id,
                sender: self.sender.clone(),
                fruit: Vec::new(),
                ctid_ff: segment_reader
                    .fast_fields()
                    .u64("ctid")
                    .expect("ctid should be a u64 fast field"),
            })
        }

        fn requires_scoring(&self) -> bool {
            false
        }

        fn merge_fruits(&self, _segment_fruits: Vec<()>) -> tantivy::Result<Self::Fruit> {
            Ok(())
        }
    }

    pub struct UnscoredBufferedChannelSegmentCollector {
        segment_ord: SegmentOrdinal,
        sender:
            crossbeam::channel::Sender<(SegmentOrdinal, Column<u64>, std::vec::IntoIter<DocId>)>,
        fruit: Vec<DocId>,
        ctid_ff: Column<u64>,
    }

    impl SegmentCollector for UnscoredBufferedChannelSegmentCollector {
        type Fruit = ();

        fn collect(&mut self, doc: DocId, _score: Score) {
            self.fruit.push(doc);
        }

        #[allow(clippy::unnecessary_to_owned)]
        fn collect_block(&mut self, docs: &[DocId]) {
            // send the block over the channel right now
            // if send fails that likely means the receiver was dropped so we have nowhere
            // to send the result.  That's okay
            self.sender
                .send((
                    self.segment_ord,
                    self.ctid_ff.clone(),
                    docs.to_vec().into_iter(),
                ))
                .ok();
        }

        fn harvest(self) -> Self::Fruit {
            if !self.fruit.is_empty() {
                // if send fails that likely means the receiver was dropped so we have nowhere
                // to send the result.  That's okay
                self.sender
                    .send((self.segment_ord, self.ctid_ff, self.fruit.into_iter()))
                    .ok();
            }
        }
    }
}

mod channel {
    use crate::index::reader::index::SearchIndexScore;
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::fastfield::Column;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    /// A [`Collector`] that uses a crossbeam channel to stream the results directly out of
    /// each segment, in parallel, as tantivy find each doc.
    pub struct ChannelCollector {
        need_scores: bool,
        sender: crossbeam::channel::Sender<(SearchIndexScore, DocAddress)>,
    }

    impl ChannelCollector {
        pub fn new(
            need_scores: bool,
            sender: crossbeam::channel::Sender<(SearchIndexScore, DocAddress)>,
        ) -> Self {
            Self {
                need_scores,
                sender,
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
            Ok(ChannelSegmentCollector {
                segment_ord: segment_local_id,
                sender: self.sender.clone(),
                ctid_ff: segment_reader
                    .fast_fields()
                    .u64("ctid")
                    .expect("ctid should be a u64 fast field"),
            })
        }

        fn requires_scoring(&self) -> bool {
            self.need_scores
        }

        fn merge_fruits(&self, _segment_fruits: Vec<()>) -> tantivy::Result<Self::Fruit> {
            Ok(())
        }
    }

    pub struct ChannelSegmentCollector {
        segment_ord: SegmentOrdinal,
        sender: crossbeam::channel::Sender<(SearchIndexScore, DocAddress)>,
        ctid_ff: Column<u64>,
    }

    impl SegmentCollector for ChannelSegmentCollector {
        type Fruit = ();

        fn collect(&mut self, doc: DocId, score: Score) {
            let doc_address = DocAddress::new(self.segment_ord, doc);
            self.sender
                .send((
                    SearchIndexScore::new(&self.ctid_ff, doc, score),
                    doc_address,
                ))
                .ok();
        }

        fn collect_block(&mut self, docs: &[DocId]) {
            for doc in docs {
                let doc = *doc;
                let doc_address = DocAddress::new(self.segment_ord, doc);
                if self
                    .sender
                    .send((SearchIndexScore::new(&self.ctid_ff, doc, 0.0), doc_address))
                    .is_err()
                {
                    // channel likely closed, so get out
                    break;
                }
            }
        }

        fn harvest(self) -> Self::Fruit {}
    }
}

mod vec_collector {
    use crate::index::reader::index::SearchIndexScore;
    use tantivy::collector::{Collector, SegmentCollector};
    use tantivy::fastfield::Column;
    use tantivy::{DocAddress, DocId, Score, SegmentOrdinal, SegmentReader};

    #[derive(Default)]
    pub enum FruitStyle {
        #[default]
        Empty,
        Scored(
            SegmentOrdinal,
            Column<u64>,
            std::vec::IntoIter<DocId>,
            std::vec::IntoIter<Score>,
        ),
        Blocks(
            SegmentOrdinal,
            Column<u64>,
            std::iter::Flatten<std::vec::IntoIter<std::vec::IntoIter<DocId>>>,
        ),
    }

    impl Iterator for FruitStyle {
        type Item = (SearchIndexScore, DocAddress);

        fn next(&mut self) -> Option<Self::Item> {
            match self {
                FruitStyle::Empty => None,
                FruitStyle::Scored(segment_ord, ctid, doc, score) => {
                    let doc = doc.next()?;
                    let doc_address = DocAddress::new(*segment_ord, doc);
                    let scored = SearchIndexScore::new(ctid, doc, score.next()?);
                    Some((scored, doc_address))
                }
                FruitStyle::Blocks(segment_ord, ctid, doc) => {
                    let doc = doc.next()?;
                    let doc_address = DocAddress::new(*segment_ord, doc);
                    let scored = SearchIndexScore::new(ctid, doc, 0.0);
                    Some((scored, doc_address))
                }
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            match self {
                FruitStyle::Empty => (0, None),
                FruitStyle::Scored(_, _, iter, _) => iter.size_hint(),
                FruitStyle::Blocks(_, _, iter) => iter.size_hint(),
            }
        }

        fn count(self) -> usize
        where
            Self: Sized,
        {
            match self {
                FruitStyle::Empty => 0,
                FruitStyle::Scored(_, _, iter, _) => iter.count(),
                FruitStyle::Blocks(_, _, iter) => iter.count(),
            }
        }
    }

    /// A [`Collector`] that collects all matching documents into a [`Vec`].
    pub struct VecCollector {
        need_scores: bool,
    }

    impl VecCollector {
        pub fn new(need_scores: bool) -> Self {
            Self { need_scores }
        }
    }

    impl Collector for VecCollector {
        type Fruit = Vec<FruitStyle>;
        type Child = VecSegmentCollector;

        fn for_segment(
            &self,
            segment_local_id: SegmentOrdinal,
            segment_reader: &SegmentReader,
        ) -> tantivy::Result<Self::Child> {
            Ok(VecSegmentCollector {
                segment_ord: segment_local_id,
                scored: (vec![], vec![]),
                blocks: vec![],
                ctid_ff: segment_reader
                    .fast_fields()
                    .u64("ctid")
                    .expect("ctid should be a u64 fast field"),
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
    }

    pub struct VecSegmentCollector {
        segment_ord: SegmentOrdinal,
        scored: (Vec<DocId>, Vec<Score>),
        blocks: Vec<std::vec::IntoIter<DocId>>,
        ctid_ff: Column<u64>,
    }

    impl SegmentCollector for VecSegmentCollector {
        type Fruit = FruitStyle;

        fn collect(&mut self, doc: DocId, score: Score) {
            self.scored.0.push(doc);
            self.scored.1.push(score);
        }

        #[allow(clippy::unnecessary_to_owned)]
        fn collect_block(&mut self, docs: &[DocId]) {
            self.blocks.push(docs.to_vec().into_iter());
        }

        fn harvest(mut self) -> Self::Fruit {
            if !self.blocks.is_empty() {
                if !self.scored.0.is_empty() {
                    self.blocks.push(self.scored.0.into_iter());
                }
                FruitStyle::Blocks(
                    self.segment_ord,
                    self.ctid_ff,
                    self.blocks.into_iter().flatten(),
                )
            } else {
                FruitStyle::Scored(
                    self.segment_ord,
                    self.ctid_ff,
                    self.scored.0.into_iter(),
                    self.scored.1.into_iter(),
                )
            }
        }
    }
}
