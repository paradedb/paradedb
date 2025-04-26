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

use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::scorer_iter::DeferredScorer;
use crate::index::setup_tokenizers;
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
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use tantivy::collector::{Collector, TopDocs};
use tantivy::index::{Index, SegmentId};
use tantivy::query::{EnableScoring, QueryClone, QueryParser};
use tantivy::schema::FieldType;
use tantivy::termdict::TermOrdinal;
use tantivy::{
    query::Query, DocAddress, DocId, DocSet, IndexReader, Order, ReloadPolicy, Score, Searcher,
    SegmentOrdinal, SegmentReader, TantivyDocument,
};
use tantivy::{snippet::SnippetGenerator, Executor};
use crate::postgres::customscan::pdbscan::debug_document_id;

/// Represents a matching document from a tantivy search.  Typically, it is returned as an Iterator
/// Item alongside the originating tantivy [`DocAddress`]
#[derive(Debug, Clone, Copy)]
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

impl PartialEq for SearchIndexScore {
    fn eq(&self, other: &Self) -> bool {
        self.ctid == other.ctid && self.bm25.to_bits() == other.bm25.to_bits()
    }
}

// Manual implementation of Eq that delegates to PartialEq
// This is technically not completely correct for f32, but works for our ordering needs
impl Eq for SearchIndexScore {}

impl PartialOrd for SearchIndexScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.bm25.partial_cmp(&other.bm25) {
            Some(Ordering::Equal) => Some(self.ctid.cmp(&other.ctid)),
            Some(ordering) => Some(ordering),
            None => Some(self.ctid.cmp(&other.ctid)), // If score comparison fails, use ctid
        }
    }
}

impl Ord for SearchIndexScore {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by score (bm25)
        match self.bm25.partial_cmp(&other.bm25) {
            Some(Ordering::Equal) => self.ctid.cmp(&other.ctid), // For equal scores, sort by ctid
            Some(ordering) => ordering,
            None => self.ctid.cmp(&other.ctid), // If score comparison fails, use ctid
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
    None,
}

impl From<SortDirection> for Order {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Asc => Order::Asc,
            SortDirection::Desc => Order::Desc,
            SortDirection::None => Order::Asc,
        }
    }
}

pub type FastFieldCache = FxHashMap<SegmentOrdinal, FFType>;
/// An iterator of the different styles of search results we can return
#[allow(clippy::large_enum_variant)]
#[derive(Default)]
pub enum SearchResults {
    #[default]
    None,
    TopNByScore(
        Searcher,
        FastFieldCache,
        std::vec::IntoIter<(Score, DocAddress)>,
    ),
    TopNByTweakedScore(
        Searcher,
        FastFieldCache,
        std::vec::IntoIter<(TweakedScore, DocAddress)>,
    ),
    TopNByField(
        Searcher,
        FastFieldCache,
        std::vec::IntoIter<(TermOrdinal, DocAddress)>,
    ),
    SingleSegment(
        Searcher,
        SegmentOrdinal,
        Option<FFType>,
        scorer_iter::ScorerIter,
    ),
    AllSegments(Searcher, Option<FFType>, Vec<scorer_iter::ScorerIter>),
}

#[derive(PartialEq, Clone, Debug)]
pub struct TweakedScore {
    pub dir: SortDirection,
    pub score: Score,
    pub ctid: u64,
}

impl PartialOrd for TweakedScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let score_cmp = self.score.partial_cmp(&other.score);

        // First check scores
        match score_cmp {
            // If scores are equal, use CTID as tie-breaker for deterministic ordering
            Some(Ordering::Equal) => Some(self.ctid.cmp(&other.ctid)),

            // Otherwise, use the score comparison with the direction modifier
            Some(cmp) => match self.dir {
                SortDirection::Desc => Some(cmp),
                SortDirection::Asc => Some(cmp.reverse()),
                SortDirection::None => Some(Ordering::Equal),
            },

            // If scores can't be compared, fall back to CTID
            None => Some(self.ctid.cmp(&other.ctid)),
        }
    }
}

impl SearchResults {
    pub fn len(&self) -> Option<usize> {
        match self {
            SearchResults::None => Some(0),
            SearchResults::TopNByScore(_, _, iter) => Some(iter.len()),
            SearchResults::TopNByTweakedScore(_, _, iter) => Some(iter.len()),
            SearchResults::TopNByField(_, _, iter) => Some(iter.len()),
            SearchResults::SingleSegment(_, _, _, _) => None,
            SearchResults::AllSegments(_, _, _) => None,
        }
    }

    /// Helper function to get CTID from fast fields for a document
    /// This helps ensure stable ordering when joining with other tables
    fn get_ctid_from_fast_fields(searcher: &Searcher, doc_address: DocAddress) -> Option<u64> {
        let segment_reader = searcher.segment_reader(doc_address.segment_ord);
        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());
        ctid_ff.as_u64(doc_address.doc_id)
    }

    /// Convert AllSegments to TopNByScore for more deterministic results
    /// This is particularly important for CTEs which may not be materialized
    pub fn merge_all_segments(
        &self,
        _query: &SearchQueryInput,
        segment_iters: Vec<Box<dyn Iterator<Item = (f32, DocAddress)> + Send>>,
        limit: Option<usize>,
    ) -> SearchResults {
        pgrx::warning!(
            "merge_all_segments: Merging {} segment iterators with limit={:?} in context={:?}",
            segment_iters.len(),
            limit,
            unsafe { pg_sys::CurrentMemoryContext }
        );

        if segment_iters.is_empty() {
            pgrx::warning!("merge_all_segments: No segments to merge, returning None");
            return SearchResults::None;
        }

        // For CTE compatibility, always collect all results then sort them deterministically
        // This ensures consistent ordering regardless of segment processing order
        pgrx::warning!(
            "merge_all_segments: Collecting all results for deterministic ordering in context={:?}",
            unsafe { pg_sys::CurrentMemoryContext }
        );

        // Collect all results from all segments with CTIDs for later stable sorting
        let mut all_results = Vec::new();
        for (idx, mut iter) in segment_iters.into_iter().enumerate() {
            let mut count = 0;
            pgrx::warning!(
                "merge_all_segments: Processing segment iterator #{} in context={:?}",
                idx,
                unsafe { pg_sys::CurrentMemoryContext }
            );
            
            while let Some((score, doc_address)) = iter.next() {
                count += 1;
                
                // Get CTID for the document - critical for stable ordering
                let ctid = match self {
                    SearchResults::AllSegments(searcher, _, _) => 
                        Self::get_ctid_from_fast_fields(searcher, doc_address).unwrap_or(0),
                    _ => 0,
                };
                
                let doc_id_info = match self {
                    SearchResults::AllSegments(searcher, _, _) => debug_document_id(searcher, doc_address),
                    _ => "searcher not available".to_string(),
                };
                
                pgrx::warning!(
                    "merge_all_segments: Segment #{} - document with score={}, ctid={}, segment={}, doc_id={}, {} in context={:?}",
                    idx, score, ctid, doc_address.segment_ord, doc_address.doc_id, doc_id_info,
                    unsafe { pg_sys::CurrentMemoryContext }
                );
                
                // Special logging for company ID 15
                if doc_id_info.contains("15:") {
                    pgrx::warning!(
                        "merge_all_segments: COMPANY ID 15 FOUND in segment #{} with score={}, ctid={}, doc_id={}, {} in context={:?}",
                        idx, score, ctid, doc_address.doc_id, doc_id_info,
                        unsafe { pg_sys::CurrentMemoryContext }
                    );
                }
                
                // Store with score and CTID for stable ordering
                all_results.push((score, ctid, doc_address));
            }
            pgrx::warning!(
                "merge_all_segments: Segment #{} yielded {} documents in context={:?}",
                idx, count, unsafe { pg_sys::CurrentMemoryContext }
            );
        }

        pgrx::warning!(
            "merge_all_segments: Collected {} total results from all segments in context={:?}",
            all_results.len(),
            unsafe { pg_sys::CurrentMemoryContext }
        );

        if all_results.is_empty() {
            pgrx::warning!(
                "merge_all_segments: No results after collection, returning None in context={:?}",
                unsafe { pg_sys::CurrentMemoryContext }
            );
            return SearchResults::None;
        }

        // Sort deterministically - first by score (descending), then by CTID (ascending) for stability
        pgrx::warning!(
            "merge_all_segments: Sorting results by score (desc) and then CTID in context={:?}",
            unsafe { pg_sys::CurrentMemoryContext }
        );
        
        all_results.sort_by(|(score_a, ctid_a, doc_a), (score_b, ctid_b, doc_b)| {
            // First sort by score descending (higher scores first)
            let cmp = score_b
                .partial_cmp(score_a)
                .unwrap_or(std::cmp::Ordering::Equal);
                
            // For equal scores, sort by CTID for stable ordering
            let final_cmp = cmp
                .then_with(|| ctid_a.cmp(ctid_b)) // Use CTID for deterministic ordering
                .then_with(|| doc_a.segment_ord.cmp(&doc_b.segment_ord)) // Segment as fallback
                .then_with(|| doc_a.doc_id.cmp(&doc_b.doc_id)); // DocID as final tiebreaker
                
            pgrx::warning!(
                "merge_all_segments: Comparing doc({},{}) score={} ctid={} with doc({},{}) score={} ctid={} => {:?} in context={:?}",
                doc_a.segment_ord, doc_a.doc_id, score_a, ctid_a,
                doc_b.segment_ord, doc_b.doc_id, score_b, ctid_b,
                final_cmp,
                unsafe { pg_sys::CurrentMemoryContext }
            );
            
            final_cmp
        });

        // Apply limit if specified
        if let Some(limit) = limit {
            if limit < all_results.len() {
                pgrx::warning!(
                    "merge_all_segments: Applying limit {} to {} results in context={:?}",
                    limit,
                    all_results.len(),
                    unsafe { pg_sys::CurrentMemoryContext }
                );
                all_results.truncate(limit);
            }
        }

        // Log final sorted results with special focus on company ID 15
        for (i, (score, ctid, doc_address)) in all_results.iter().enumerate() {
            let doc_id_info = match self {
                SearchResults::AllSegments(searcher, _, _) => debug_document_id(searcher, *doc_address),
                _ => "searcher not available".to_string(),
            };
            
            pgrx::warning!(
                "merge_all_segments: Final sorted result #{}: score={}, ctid={}, segment={}, doc_id={}, {} in context={:?}",
                i, score, ctid, doc_address.segment_ord, doc_address.doc_id, doc_id_info,
                unsafe { pg_sys::CurrentMemoryContext }
            );
            
            // Special logging for company ID 15
            if doc_id_info.contains("15:") {
                pgrx::warning!(
                    "merge_all_segments: COMPANY ID 15 in final results at position #{}: ctid={}, {} in context={:?}",
                    i, ctid, doc_id_info, unsafe { pg_sys::CurrentMemoryContext }
                );
            }
        }

        // Return with just the score and doc_address components
        // The CTID was only used for stable sorting
        let final_results = all_results
            .into_iter()
            .map(|(score, _ctid, doc_address)| (score, doc_address))
            .collect::<Vec<_>>();

        // Return as TopNByScore for consistent behavior
        pgrx::warning!(
            "merge_all_segments: Returning TopNByScore with {} results in context={:?}",
            final_results.len(),
            unsafe { pg_sys::CurrentMemoryContext }
        );
        
        SearchResults::TopNByScore(
            match self {
                SearchResults::AllSegments(searcher, _, _) => searcher.clone(),
                _ => panic!("merge_all_segments must be called on AllSegments"),
            },
            Default::default(),
            final_results.into_iter(),
        )
    }

    // Helper method to get segment iterators for merge_all_segments
    fn get_segment_iterators(&self) -> Vec<Box<dyn Iterator<Item = (f32, DocAddress)> + Send>> {
        match self {
            SearchResults::AllSegments(searcher, _, _) => {
                // We can't use clone directly because ScorerIter doesn't implement Clone
                // Instead, create an empty vector and return it
                // The caller should handle this special case
                Vec::new() // Return empty vector - we can't clone ScorerIter
            }
            _ => Vec::new(),
        }
    }
}

impl Iterator for SearchResults {
    type Item = (SearchIndexScore, DocAddress);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (searcher, ff_lookup, (score, doc_address)) =
            match self {
                SearchResults::None => return None,
                SearchResults::TopNByScore(searcher, ff_lookup, iter) => {
                    (searcher, ff_lookup, iter.next()?)
                }
                SearchResults::TopNByTweakedScore(searcher, ff_lookup, iter) => {
                    let (score, doc_id) = iter.next()?;
                    (searcher, ff_lookup, (score.score, doc_id))
                }
                SearchResults::TopNByField(searcher, ff_lookup, iter) => {
                    let (_, doc_id) = iter.next()?;
                    (searcher, ff_lookup, (1.0, doc_id))
                }
                SearchResults::SingleSegment(searcher, segment_ord, fftype, iter) => {
                    let (score, doc_address) = iter.next()?;
                    let ctid_ff = fftype.get_or_insert_with(|| {
                        FFType::new_ctid(searcher.segment_reader(*segment_ord).fast_fields())
                    });
                    let scored = SearchIndexScore {
                        ctid: ctid_ff
                            .as_u64(doc_address.doc_id)
                            .expect("ctid should be present"),
                        bm25: score,
                    };

                    pgrx::warning!(
                    "SearchResults::next: SingleSegment returning ctid={}, score={}, segment={}",
                    scored.ctid, scored.bm25, doc_address.segment_ord
                );
                    return Some((scored, doc_address));
                }
                SearchResults::AllSegments(searcher, fftype, iters) => {
                    // For deterministic ordering from parallel workers, we need to
                    // collect all available docs, sort them, and return them in order
                    // This ensures predictable order in CTEs and JOIN operations
                    pgrx::warning!("SearchResults::next: Processing AllSegments with {} iterators", iters.len());
                    let mut all_results = Vec::new();

                    // Process all iterators to collect available results
                    for (i, iter) in iters.iter_mut().enumerate() {
                        pgrx::warning!("SearchResults::next: Processing iterator #{}", i);
                        let mut count = 0;
                        while let Some((score, doc_address)) = iter.next() {
                            count += 1;
                            let ctid_ff = fftype.get_or_insert_with(|| {
                                FFType::new_ctid(
                                    searcher
                                        .segment_reader(doc_address.segment_ord)
                                        .fast_fields(),
                                )
                            });
                            let ctid = ctid_ff
                                .as_u64(doc_address.doc_id)
                                .expect("ctid should be present");

                            pgrx::warning!(
                                "SearchResults::next: Iterator #{} yielded doc with ctid={}, score={}, segment={}, doc_id={}",
                                i, ctid, score, doc_address.segment_ord, doc_address.doc_id
                            );
                            all_results.push((SearchIndexScore { ctid, bm25: score }, doc_address));
                        }
                        pgrx::warning!(
                            "SearchResults::next: Iterator #{} yielded {} total documents",
                            i, count
                        );
                    }

                    pgrx::warning!(
                        "SearchResults::next: Collected {} total results from all iterators",
                        all_results.len()
                    );

                    // If we collected results, sort them deterministically
                    if !all_results.is_empty() {
                        // Sort by score (desc) and then by ctid for equal scores
                        all_results.sort_by(|(a, _), (b, _)| {
                            pgrx::warning!(
                                "SearchResults::next: Comparing scores a={} b={}, ctids a={} b={}",
                                a.bm25, b.bm25, a.ctid, b.ctid
                            );
                            b.cmp(a)
                        });

                        // Log sorted results
                        for (i, (score, doc_address)) in all_results.iter().enumerate() {
                            pgrx::warning!(
                                "SearchResults::next: Sorted result #{}: ctid={}, score={}, segment={}, doc_id={}",
                                i, score.ctid, score.bm25, doc_address.segment_ord, doc_address.doc_id
                            );
                        }

                        // Convert AllSegments to TopNByScore with sorted results
                        let scored_results = all_results
                            .into_iter()
                            .map(|(scored, addr)| (scored.bm25, addr))
                            .collect::<Vec<_>>();

                        pgrx::warning!(
                            "SearchResults::next: Converting AllSegments to TopNByScore with {} results",
                            scored_results.len()
                        );

                        *self = SearchResults::TopNByScore(
                            searcher.clone(),
                            Default::default(),
                            scored_results.into_iter(),
                        );

                        // Now that we've converted to TopNByScore, call next() again
                        return self.next();
                    }

                    // If we didn't collect any results, we're done
                    pgrx::warning!("SearchResults::next: No results collected from iterators, returning None");
                    return None;
                }
            };

        let ctid_ff = ff_lookup.entry(doc_address.segment_ord).or_insert_with(|| {
            FFType::new_ctid(
                searcher
                    .segment_reader(doc_address.segment_ord)
                    .fast_fields(),
            )
        });
        let scored = SearchIndexScore {
            ctid: ctid_ff
                .as_u64(doc_address.doc_id)
                .expect("ctid should be present"),
            bm25: score,
        };

        pgrx::warning!(
            "SearchResults::next: Returning ctid={}, score={}, segment={}",
            scored.ctid,
            scored.bm25,
            doc_address.segment_ord
        );
        Some((scored, doc_address))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SearchResults::None => (0, Some(0)),
            SearchResults::TopNByScore(_, _, iter) => iter.size_hint(),
            SearchResults::TopNByTweakedScore(_, _, iter) => iter.size_hint(),
            SearchResults::TopNByField(_, _, iter) => iter.size_hint(),
            SearchResults::SingleSegment(_, _, _, iter) => iter.size_hint(),
            SearchResults::AllSegments(_, _, iters) => {
                let hint = iters
                    .first()
                    .map(|iter| iter.size_hint())
                    .unwrap_or((0, Some(0)));
                (hint.0, hint.1.map(|n| n * iters.len()))
            }
        }
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        match self {
            SearchResults::None => 0,
            SearchResults::TopNByScore(_, _, iter) => iter.count(),
            SearchResults::TopNByTweakedScore(_, _, iter) => iter.count(),
            SearchResults::TopNByField(_, _, iter) => iter.count(),
            SearchResults::SingleSegment(_, _, _, iter) => iter.count(),
            SearchResults::AllSegments(_, _, iters) => {
                iters.into_iter().map(|iter| iter.count()).sum()
            }
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
    _cleanup_lock: Arc<PinnedBuffer>,
}

impl SearchIndexReader {
    pub fn open(index_relation: &PgRelation, mvcc_style: MvccSatisfies) -> Result<Self> {
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
        let cleanup_lock = BufferManager::new(index_relation.oid()).pinned_buffer(CLEANUP_LOCK);

        let directory = mvcc_style.directory(index_relation);
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

    pub fn segment_ids(&self) -> HashSet<SegmentId> {
        self.searcher
            .segment_readers()
            .iter()
            .map(|r| r.segment_id())
            .collect()
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
        estimated_rows: Option<usize>,
    ) -> SearchResults {
        pgrx::warning!(
            "SearchIndexReader::search: Starting search with need_scores={}, estimated_rows={:?}, memory_context={:?}",
            need_scores, estimated_rows, unsafe { pg_sys::CurrentMemoryContext }
        );
        
        // Log memory context details for diagnostics
        unsafe {
            if !pg_sys::CurrentMemoryContext.is_null() {
                // Use simple pointer logging instead of MemoryContextName
                pgrx::warning!(
                    "SearchIndexReader::search: Memory context address: {:p}",
                    pg_sys::CurrentMemoryContext
                );
            }
        }
        
        let tantivy_query = self.query(query);
        let iters = self
            .searcher()
            .segment_readers()
            .iter()
            .enumerate()
            .map(move |(segment_ord, segment_reader)| {
                pgrx::warning!(
                    "SearchIndexReader::search: Creating iterator for segment_ord={}, num_docs={}, memory_context={:?}",
                    segment_ord, segment_reader.num_docs(), unsafe { pg_sys::CurrentMemoryContext }
                );
                scorer_iter::ScorerIter::new(
                    DeferredScorer::new(
                        tantivy_query.box_clone(),
                        need_scores,
                        segment_reader.clone(),
                        self.searcher.clone(),
                    ),
                    segment_ord as SegmentOrdinal,
                    segment_reader.clone(),
                )
            })
            .collect::<Vec<_>>();

        pgrx::warning!(
            "SearchIndexReader::search: Created {} segment iterators in context={:?}",
            iters.len(),
            unsafe { pg_sys::CurrentMemoryContext }
        );

        // Create AllSegments for deterministic results
        let allsegments =
            SearchResults::AllSegments(self.searcher.clone(), Default::default(), iters);
            
        // Instead of attempting to get iterators (which can't be cloned),
        // collect all results directly into a vec by processing the AllSegments
        match &allsegments {
            SearchResults::AllSegments(_searcher, _fftype, _iters) => {
                // For CTE compatibility, always collect all results then sort them deterministically
                // This ensures consistent ordering regardless of segment processing order
                pgrx::warning!(
                    "SearchIndexReader::search: Collecting all results for deterministic ordering in context={:?}",
                    unsafe { pg_sys::CurrentMemoryContext }
                );

                // Collect all results from all segment iterators with CTID for deterministic ordering
                let mut all_results = Vec::new();
                for segment_reader in self.searcher().segment_readers() {
                    let segment_id = segment_reader.segment_id();
                    pgrx::warning!(
                        "SearchIndexReader::search: Processing segment_id={}, num_docs={}, context={:?}",
                        segment_id, segment_reader.num_docs(), unsafe { pg_sys::CurrentMemoryContext }
                    );
                    
                    // Create a new segment search for each segment
                    let segment_results = self.search_segment(need_scores, segment_id, query);
                    
                    // Process and collect results from this segment
                    let mut count = 0;
                    for result in segment_results {
                        count += 1;
                        let (score_info, doc_address) = (result.0, result.1);
                        let score = score_info.bm25;
                        let ctid = score_info.ctid;
                        
                        pgrx::warning!(
                            "SearchIndexReader::search: Segment {} yielded doc with ctid={}, score={}, segment={}, doc_id={}, context={:?}",
                            segment_id, ctid, score, doc_address.segment_ord, doc_address.doc_id,
                            unsafe { pg_sys::CurrentMemoryContext }
                        );
                        
                        // Special logging for company ID 15
                        if ctid == 15 {
                            pgrx::warning!(
                                "SearchIndexReader::search: COMPANY ID 15 FOUND in segment {}, ctid={}, doc_id={}, context={:?}",
                                segment_id, ctid, doc_address.doc_id, unsafe { pg_sys::CurrentMemoryContext }
                            );
                        }
                        
                        // Store score, CTID, and doc_address for deterministic ordering
                        all_results.push((score, ctid, doc_address));
                    }
                    pgrx::warning!(
                        "SearchIndexReader::search: Segment {} yielded {} total documents in context={:?}",
                        segment_id, count, unsafe { pg_sys::CurrentMemoryContext }
                    );
                }

                pgrx::warning!(
                    "SearchIndexReader::search: Collected {} total results from all segments in context={:?}",
                    all_results.len(),
                    unsafe { pg_sys::CurrentMemoryContext }
                );

                // Sort deterministically - first by score (descending), then by CTID for stable ordering
                all_results.sort_by(|(score_a, ctid_a, doc_a), (score_b, ctid_b, doc_b)| {
                    // First sort by score descending (higher scores first)
                    let cmp = score_b
                        .partial_cmp(score_a)
                        .unwrap_or(std::cmp::Ordering::Equal);
                        
                    // For equal scores, use CTID, then segment, then doc_id for complete determinism
                    let final_cmp = cmp
                        .then_with(|| ctid_a.cmp(ctid_b)) // CTID as primary tiebreaker
                        .then_with(|| doc_a.segment_ord.cmp(&doc_b.segment_ord))
                        .then_with(|| doc_a.doc_id.cmp(&doc_b.doc_id));
                    
                    pgrx::warning!(
                        "SearchIndexReader::search: Comparing doc({},{}) score={} ctid={} with doc({},{}) score={} ctid={} => {:?}",
                        doc_a.segment_ord, doc_a.doc_id, score_a, ctid_a,
                        doc_b.segment_ord, doc_b.doc_id, score_b, ctid_b,
                        final_cmp
                    );
                    
                    final_cmp
                });

                // Apply limit if specified
                if let Some(limit) = estimated_rows {
                    if limit < all_results.len() {
                        pgrx::warning!(
                            "SearchIndexReader::search: Applying limit {} to {} results in context={:?}",
                            limit, all_results.len(), unsafe { pg_sys::CurrentMemoryContext }
                        );
                        all_results.truncate(limit);
                    }
                }

                // Log final sorted results
                for (i, (score, ctid, doc_address)) in all_results.iter().enumerate() {
                    let doc_id_info = match self {
                        _ => debug_document_id(&self.searcher, *doc_address),
                    };
                    
                    pgrx::warning!(
                        "SearchIndexReader::search: Final sorted result #{}: score={}, ctid={}, segment={}, doc_id={}, {}, context={:?}",
                        i, score, ctid, doc_address.segment_ord, doc_address.doc_id, doc_id_info,
                        unsafe { pg_sys::CurrentMemoryContext }
                    );
                    
                    // Special logging for company ID 15
                    if *ctid == 15 || doc_id_info.contains("15") {
                        pgrx::warning!(
                            "SearchIndexReader::search: COMPANY ID 15 in final results at position #{}: ctid={}, {}, context={:?}",
                            i, ctid, doc_id_info, unsafe { pg_sys::CurrentMemoryContext }
                        );
                    }
                }

                // Convert back to (score, doc_address) format for returning
                let final_results = all_results
                    .into_iter()
                    .map(|(score, _ctid, doc_address)| (score, doc_address))
                    .collect::<Vec<_>>();

                // Return as TopNByScore for consistent behavior
                pgrx::warning!(
                    "SearchIndexReader::search: Returning TopNByScore with {} results in context={:?}",
                    final_results.len(),
                    unsafe { pg_sys::CurrentMemoryContext }
                );
                SearchResults::TopNByScore(
                    self.searcher.clone(),
                    Default::default(),
                    final_results.into_iter(),
                )
            }
            _ => allsegments, // Should never happen, but return as is
        }
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
        segment_id: SegmentId,
        query: &SearchQueryInput,
    ) -> SearchResults {
        let query = self.query(query);
        let (segment_ord, segment_reader) = self
            .searcher
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
        let iter = scorer_iter::ScorerIter::new(
            DeferredScorer::new(
                query,
                need_scores,
                segment_reader.clone(),
                self.searcher.clone(),
            ),
            segment_ord as SegmentOrdinal,
            segment_reader.clone(),
        );
        SearchResults::SingleSegment(
            self.searcher.clone(),
            segment_ord as SegmentOrdinal,
            None,
            iter,
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
        segment_id: SegmentId,
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
            self.top_by_field_in_segment(segment_id, query, sort_field, sortdir, n)
        } else {
            self.top_by_score_in_segment(segment_id, query, sortdir, n, need_scores)
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
                            ctid: 0,
                        }
                    },
                );
                let top_docs = self.collect(query, collector, true);
                SearchResults::TopNByTweakedScore(
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
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            SortDirection::None => self.search(need_scores, false, query, Some(n)),
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
        segment_id: SegmentId,
        query: &SearchQueryInput,
        sort_field: String,
        sortdir: SortDirection,
        n: usize,
    ) -> SearchResults {
        let (segment_ord, segment_reader) = self
            .searcher
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));
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
                segment_ord as SegmentOrdinal,
                segment_reader,
            )
            .expect("should be able to collect top-n in segment");
        let top_docs = collector
            .merge_fruits(vec![top_docs])
            .expect("should be able to merge top-n in segment");
        SearchResults::TopNByField(
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
        segment_id: SegmentId,
        query: &SearchQueryInput,
        sortdir: SortDirection,
        n: usize,
        _need_scores: bool,
    ) -> SearchResults {
        pgrx::warning!(
            "top_by_score_in_segment: Starting with segment_id={}, sortdir={:?}, n={}, context={:?}",
            segment_id, sortdir, n, unsafe { pg_sys::CurrentMemoryContext }
        );
    
        let (segment_ord, segment_reader) = self
            .searcher
            .segment_readers()
            .iter()
            .enumerate()
            .find(|(_, reader)| reader.segment_id() == segment_id)
            .unwrap_or_else(|| panic!("segment {segment_id} should exist"));

        pgrx::warning!(
            "top_by_score_in_segment: Found segment_ord={} with num_docs={}, context={:?}",
            segment_ord, segment_reader.num_docs(), unsafe { pg_sys::CurrentMemoryContext }
        );

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
                // Create a tweak_score function that also captures the document ID to ensure
                // deterministic ordering when scores are equal
                let collector = TopDocs::with_limit(n).tweak_score(
                    move |segment_reader: &tantivy::SegmentReader| {
                        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());
                        move |doc: DocId, original_score: Score| {
                            // Get the document's CTID
                            let ctid = ctid_ff.as_u64(doc).expect("ctid should be present");
                            
                            pgrx::warning!(
                                "top_by_score_in_segment: ASC - doc_id={}, score={}, ctid={}, context={:?}",
                                doc, original_score, ctid, unsafe { pg_sys::CurrentMemoryContext }
                            );
                            
                            // Special logging for company ID 15 - we'll check the full document elsewhere
                            // instead of directly comparing CTID which is unreliable

                            TweakedScore {
                                dir: sortdir,
                                score: original_score,
                                ctid, // Store CTID to use as secondary sort key
                            }
                        }
                    },
                );

                let top_docs = collector
                    .collect_segment(
                        weight.as_ref(),
                        segment_ord as SegmentOrdinal,
                        segment_reader,
                    )
                    .expect("should be able to collect top-n in segment");

                pgrx::warning!(
                    "top_by_score_in_segment: ASC - Collected docs from segment {} in context={:?}",
                    segment_id, unsafe { pg_sys::CurrentMemoryContext }
                );

                let top_docs = collector
                    .merge_fruits(vec![top_docs])
                    .expect("should be able to merge top-n in segment");

                pgrx::warning!(
                    "top_by_score_in_segment: ASC - Merged result has docs in context={:?}",
                    unsafe { pg_sys::CurrentMemoryContext }
                );
                
                // Log top docs details
                for (i, (score, doc_address)) in top_docs.iter().enumerate() {
                    let doc_id_info = debug_document_id(&self.searcher, *doc_address);
                    pgrx::warning!(
                        "top_by_score_in_segment: ASC - Result #{}: score={:?}, segment={}, doc_id={}, {}, context={:?}",
                        i, score, doc_address.segment_ord, doc_address.doc_id, doc_id_info,
                        unsafe { pg_sys::CurrentMemoryContext }
                    );
                    
                    // Special logging for company ID 15
                    if doc_id_info.contains("15:") {
                        pgrx::warning!(
                            "top_by_score_in_segment: COMPANY ID 15 in ASC results: {}, context={:?}",
                            doc_id_info, unsafe { pg_sys::CurrentMemoryContext }
                        );
                    }
                }

                SearchResults::TopNByTweakedScore(
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            // can use tantivy's score directly
            SortDirection::Desc => {
                // Create a custom collector that sorts by score and then by document ID
                // for deterministic ordering when scores are equal
                pgrx::warning!(
                    "top_by_score_in_segment: Using score+CTID ordering for deterministic results in context={:?}",
                    unsafe { pg_sys::CurrentMemoryContext }
                );

                // Custom collector to ensure deterministic ordering
                let collector = TopDocs::with_limit(n).tweak_score(
                    move |segment_reader: &tantivy::SegmentReader| {
                        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());
                        move |doc: DocId, original_score: Score| {
                            // Get the document's CTID for secondary sorting
                            let ctid = ctid_ff.as_u64(doc).expect("ctid should be present");
                            
                            pgrx::warning!(
                                "top_by_score_in_segment: DESC - doc_id={}, score={}, ctid={}, context={:?}",
                                doc, original_score, ctid, unsafe { pg_sys::CurrentMemoryContext }
                            );
                            
                            // Special logging for company ID 15 - we'll check the full document elsewhere
                            // instead of directly comparing CTID which is unreliable

                            // Store the original score and CTID for sorting
                            TweakedScore {
                                dir: SortDirection::Desc,
                                score: original_score,
                                ctid,
                            }
                        }
                    },
                );

                let top_docs = collector
                    .collect_segment(
                        weight.as_ref(),
                        segment_ord as SegmentOrdinal,
                        segment_reader,
                    )
                    .expect("should be able to collect top-n in segment");

                pgrx::warning!(
                    "top_by_score_in_segment: DESC - Collected docs from segment {} in context={:?}",
                    segment_id, unsafe { pg_sys::CurrentMemoryContext }
                );

                let top_docs = collector
                    .merge_fruits(vec![top_docs])
                    .expect("should be able to merge top-n in segment");

                pgrx::warning!(
                    "top_by_score_in_segment: DESC - Merged result has docs in context={:?}",
                    unsafe { pg_sys::CurrentMemoryContext }
                );
                
                // Log top docs details
                for (i, (score, doc_address)) in top_docs.iter().enumerate() {
                    let doc_id_info = debug_document_id(&self.searcher, *doc_address);
                    pgrx::warning!(
                        "top_by_score_in_segment: DESC - Result #{}: score={:?}, segment={}, doc_id={}, {}, context={:?}",
                        i, score, doc_address.segment_ord, doc_address.doc_id, doc_id_info,
                        unsafe { pg_sys::CurrentMemoryContext }
                    );
                    
                    // Special logging for company ID 15
                    if doc_id_info.contains("15") {
                        pgrx::warning!(
                            "top_by_score_in_segment: COMPANY ID 15 in DESC results: {}, score={:?}, context={:?}",
                            doc_id_info, score, unsafe { pg_sys::CurrentMemoryContext }
                        );
                    }
                }

                // Convert to TopNByTweakedScore to preserve our CTID-based ordering
                SearchResults::TopNByTweakedScore(
                    self.searcher.clone(),
                    Default::default(),
                    top_docs.into_iter(),
                )
            }

            SortDirection::None => {
                pgrx::warning!(
                    "top_by_score_in_segment: Using SortDirection::None in context={:?}",
                    unsafe { pg_sys::CurrentMemoryContext }
                );
                
                let iter = scorer_iter::ScorerIter::new(
                    DeferredScorer::new(
                        query,
                        false,
                        segment_reader.clone(),
                        self.searcher.clone(),
                    ),
                    segment_ord as SegmentOrdinal,
                    segment_reader.clone(),
                );
                SearchResults::SingleSegment(
                    self.searcher.clone(),
                    segment_ord as SegmentOrdinal,
                    None,
                    iter,
                )
            }
        }
    }

    pub fn estimate_docs(&self, search_query_input: &SearchQueryInput) -> Option<usize> {
        let largest_reader = self
            .searcher
            .segment_readers()
            .iter()
            .max_by_key(|reader| reader.num_docs())?;
        let query = self.query(search_query_input);
        let weight = query
            .weight(enable_scoring(
                search_query_input.need_scores(),
                &self.searcher,
            ))
            .expect("weight should be constructable");
        let mut scorer = weight
            .scorer(largest_reader, 1.0)
            .expect("counting docs in the largest segment should not fail");

        // investigate the size_hint.  it will often give us a good enough value
        let mut count = scorer.size_hint() as usize;
        if count == 0 {
            // but when it doesn't, we need to do a full count
            count = scorer.count_including_deleted() as usize;
        }
        let segment_doc_proportion =
            largest_reader.num_docs() as f64 / self.searcher.num_docs() as f64;

        Some((count as f64 / segment_doc_proportion).ceil() as usize)
    }

    fn collect<C: Collector + 'static>(
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
                enable_scoring(need_scores, &self.searcher),
            )
            .expect("search should not fail")
    }
}

fn enable_scoring(need_scores: bool, searcher: &Searcher) -> EnableScoring {
    if need_scores {
        EnableScoring::enabled_from_searcher(searcher)
    } else {
        EnableScoring::disabled_from_searcher(searcher)
    }
}

mod scorer_iter {
    use crate::index::reader::index::enable_scoring;
    use std::sync::OnceLock;
    use tantivy::query::{Query, Scorer};
    use tantivy::{DocAddress, DocId, DocSet, Score, Searcher, SegmentOrdinal, SegmentReader};

    pub struct DeferredScorer {
        query: Box<dyn Query>,
        need_scores: bool,
        segment_reader: SegmentReader,
        searcher: Searcher,
        scorer: OnceLock<Box<dyn Scorer>>,
    }

    impl DeferredScorer {
        pub fn new(
            query: Box<dyn Query>,
            need_scores: bool,
            segment_reader: SegmentReader,
            searcher: Searcher,
        ) -> Self {
            Self {
                query,
                need_scores,
                segment_reader,
                searcher,
                scorer: Default::default(),
            }
        }

        #[track_caller]
        #[inline(always)]
        fn scorer_mut(&mut self) -> &mut Box<dyn Scorer> {
            self.scorer();
            self.scorer
                .get_mut()
                .expect("deferred scorer should have been initialized")
        }

        #[track_caller]
        #[inline(always)]
        fn scorer(&self) -> &dyn Scorer {
            self.scorer.get_or_init(|| {
                let weight = self
                    .query
                    .weight(enable_scoring(self.need_scores, &self.searcher))
                    .expect("weight should be constructable");

                weight
                    .scorer(&self.segment_reader, 1.0)
                    .expect("scorer should be constructable")
            })
        }
    }

    impl DocSet for DeferredScorer {
        #[inline(always)]
        fn advance(&mut self) -> DocId {
            self.scorer_mut().advance()
        }

        #[inline(always)]
        fn doc(&self) -> DocId {
            self.scorer().doc()
        }

        fn size_hint(&self) -> u32 {
            self.scorer().size_hint()
        }
    }

    impl Scorer for DeferredScorer {
        #[inline(always)]
        fn score(&mut self) -> Score {
            self.scorer_mut().score()
        }
    }

    pub struct ScorerIter {
        deferred: DeferredScorer,
        segment_ord: SegmentOrdinal,
        segment_reader: SegmentReader,
    }

    impl ScorerIter {
        pub fn new(
            scorer: DeferredScorer,
            segment_ord: SegmentOrdinal,
            segment_reader: SegmentReader,
        ) -> Self {
            Self {
                deferred: scorer,
                segment_ord,
                segment_reader,
            }
        }
    }

    impl Iterator for ScorerIter {
        type Item = (Score, DocAddress);

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                let doc_id = self.deferred.doc();

                if doc_id == tantivy::TERMINATED {
                    // we've read all the docs
                    return None;
                } else if self
                    .segment_reader
                    .alive_bitset()
                    .map(|alive_bitset| alive_bitset.is_alive(doc_id))
                    // if there's no alive_bitset, the doc is alive
                    .unwrap_or(true)
                {
                    // this doc is alive
                    let score = self.deferred.score();
                    let this = (score, DocAddress::new(self.segment_ord, doc_id));

                    // move to the next doc for the next iteration
                    self.deferred.advance();

                    // return the live doc
                    return Some(this);
                }

                // this doc isn't alive, move to the next doc and loop around
                self.deferred.advance();
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, Some(self.deferred.size_hint() as usize))
        }
    }
}
