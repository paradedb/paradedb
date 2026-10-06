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

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Bound;
use std::sync::Arc;

use super::proximity::{ProximityClause, query::ProximityQuery};
use crate::index::stats::{
    SegmentStats,
    distribution::{Distribution, DistributionManifest},
};
use tantivy::SegmentReader;
use tantivy::query::*;
use tantivy::schema::{Field, IndexRecordOption, Term, Type};

/// Estimates queries using pg_search's shared statistics and planner context.
trait QueryEstimate {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>>;
}

#[derive(Clone, Copy, Debug)]
struct Estimate {
    fraction: f64,
    work: u64,
}

struct Context<'a> {
    reader: &'a SegmentReader,
    planner: Option<(*mut pgrx::pg_sys::PlannerInfo, pgrx::pg_sys::Index)>,
    stats: SegmentStats,
    manifest: DistributionManifest,
    distributions: RefCell<HashMap<usize, Option<Arc<Distribution>>>>,
}

#[derive(Debug, Clone)]
pub(super) struct UnresolvedQuery;

impl QueryEstimate for dyn Query {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        if let Some(inner) = self.matching_query() {
            return inner.estimate_docs(ctx);
        }
        macro_rules! estimate_as {
            ($($query:ty),* $(,)?) => {$(
                if let Some(query) = self.downcast_ref::<$query>() {
                    return query.estimate_docs(ctx);
                }
            )*};
        }
        estimate_as! {
            TermQuery,
            BooleanQuery,
            DisjunctionMaxQuery,
            TermSetQuery,
            PhraseQuery,
            PhrasePrefixQuery,
            RegexQuery,
            RegexPhraseQuery,
            FuzzyTermQuery,
            ExistsQuery,
            RangeQuery,
            InvertedIndexRangeQuery,
            FastFieldRangeQuery,
            MoreLikeThisQuery,
            AllQuery,
            EmptyQuery,
            UnresolvedQuery,
            super::heap_field_filter::HeapFilterQuery,
            super::score::ScoreFilter,
            super::more_like_this::MoreLikeThisQuery,
            ProximityQuery,
        }
        Ok(None)
    }
}

impl QueryEstimate for AllQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate::prior(1.0, ctx.reader)))
    }
}

impl QueryEstimate for EmptyQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate::count(0, 0, ctx.reader)))
    }
}

/// Uses the number of documents containing the term from the inverted index.
impl QueryEstimate for TermQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let term = self.term();
        if !ctx
            .reader
            .schema()
            .get_field_entry(term.field())
            .is_indexed()
        {
            let bound = Bound::Included(term.clone());
            return fast_field::range((&bound, &bound), ctx);
        }
        ctx.term(term).map(Some)
    }
}

/// Combines child probabilities assuming independence, including the required number of optional matches.
impl QueryEstimate for BooleanQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut children = Vec::new();
        for (occur, child) in self.clauses() {
            let Some(child) = child.as_ref().estimate_docs(ctx)? else {
                return Ok(None);
            };
            children.push((*occur, child));
        }
        Ok(Some(Estimate::combine(
            children,
            self.get_minimum_number_should_match(),
            ctx.reader.max_doc(),
        )))
    }
}

/// The tie breaker affects scores; matching is the union of the disjuncts.
impl QueryEstimate for DisjunctionMaxQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut children = Vec::new();
        for child in self.disjuncts() {
            let Some(child) = child.as_ref().estimate_docs(ctx)? else {
                return Ok(None);
            };
            children.push((Occur::Should, child));
        }
        Ok(Some(Estimate::combine(children, 1, ctx.reader.max_doc())))
    }
}

/// Estimates the union of the individual terms using their document frequencies.
impl QueryEstimate for TermSetQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut children = Vec::new();
        for term in self.terms() {
            let query = TermQuery::new(term.clone(), IndexRecordOption::Basic);
            let Some(child) = query.estimate_docs(ctx)? else {
                return Ok(None);
            };
            children.push((Occur::Should, child));
        }
        Ok(Some(Estimate::combine(children, 1, ctx.reader.max_doc())))
    }
}

/// Multiplies the term probabilities and discounts for words having to occur together.
impl QueryEstimate for PhraseQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let terms = self
            .phrase_terms()
            .iter()
            .map(|term| ctx.term(term))
            .collect::<tantivy::Result<Vec<_>>>()?;
        Ok(Some(Estimate::phrase(&terms, self.slop(), ctx.reader)))
    }
}

/// Uses the rarest complete word; a prefix alone uses a bounded dictionary lookup.
impl QueryEstimate for PhrasePrefixQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let terms = self.phrase_terms();
        if terms.is_empty() {
            return ctx
                .matching_terms(
                    self.field(),
                    self.prefix().serialized_value_bytes(),
                    |_| true,
                    self.max_expansions() as usize,
                )
                .map(Some);
        }
        let mut rarest = ctx.reader.max_doc();
        let mut work = 0u64;
        for term in &terms {
            let count = ctx.reader.inverted_index(term.field())?.doc_freq(term)?;
            rarest = rarest.min(count);
            work = work.saturating_add(count as u64);
        }
        Ok(Some(Estimate::count(
            rarest,
            work.saturating_add(rarest as u64 * terms.len() as u64),
            ctx.reader,
        )))
    }
}

/// Reads document frequencies for matching words, stopping at the dictionary budget.
impl QueryEstimate for RegexQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        ctx.regex(self, usize::MAX).map(Some)
    }
}

/// Combines the regex word estimates and discounts for their required positions.
impl QueryEstimate for RegexPhraseQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut terms = Vec::new();
        for term in self.phrase_terms() {
            let value = term.value();
            let Some(pattern) = value.as_str() else {
                return Ok(None);
            };
            let query = RegexQuery::from_pattern(pattern, self.field())?;
            terms.push(ctx.regex(&query, self.max_expansions() as usize)?);
        }
        Ok(Some(Estimate::phrase(&terms, self.slop(), ctx.reader)))
    }
}

/// Reads frequencies for words within the edit distance, with the same dictionary budget.
impl QueryEstimate for FuzzyTermQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        use levenshtein_automata::{Distance, LevenshteinAutomatonBuilder};
        let term = self.term();
        let bytes = term.value();
        let word = if bytes.typ() == Type::Json {
            std::str::from_utf8(term.serialized_value_bytes()).ok()
        } else {
            bytes.as_str()
        };
        let Some(word) = word else {
            return Ok(None);
        };
        if word.len() > 256 || self.distance() > 2 {
            return Ok(Some(Estimate::prior(0.01, ctx.reader)));
        }
        if self.distance() == 0 && !self.prefix() {
            return TermQuery::new(term.clone(), IndexRecordOption::Basic).estimate_docs(ctx);
        }
        static BUILDERS: std::sync::LazyLock<[[LevenshteinAutomatonBuilder; 2]; 3]> =
            std::sync::LazyLock::new(|| {
                std::array::from_fn(|distance| {
                    std::array::from_fn(|transpose| {
                        LevenshteinAutomatonBuilder::new(distance as u8, transpose != 0)
                    })
                })
            });
        let builder =
            &BUILDERS[self.distance() as usize][usize::from(self.transposition_cost_one())];
        let dfa = if self.prefix() {
            builder.build_prefix_dfa(word)
        } else {
            builder.build_dfa(word)
        };
        let path = bytes.as_json().map(|(path, _)| path).unwrap_or_default();
        ctx.matching_terms(
            term.field(),
            path,
            |term| matches!(dfa.eval(term), Distance::Exact(_)),
            usize::MAX,
        )
        .map(Some)
    }
}

/// Uses the document presence counts, or the aligned samples when several JSON columns qualify.
impl QueryEstimate for ExistsQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut wanted = Vec::new();
        // Resolve schema names using the fast-field reader, including escaped JSON paths.
        let Some(canonical) = ctx.reader.fast_fields().resolve_field(self.field_name())? else {
            return Ok(Some(Estimate::count(0, 0, ctx.reader)));
        };
        let prefix = format!("{canonical}\u{1}");
        for (ordinal, (name, _)) in ctx.manifest.columns.iter().enumerate() {
            if name == &canonical || (self.json_subpaths() && name.starts_with(&prefix)) {
                let Some(summary) = ctx.distribution(ordinal) else {
                    return Ok(None);
                };
                wanted.push(summary);
            }
        }
        if wanted.is_empty() {
            return Ok(Some(Estimate::count(0, 0, ctx.reader)));
        }
        if wanted.len() == 1 {
            return Ok(Some(Estimate::count(
                wanted[0].present_docs,
                ctx.reader.max_doc() as u64,
                ctx.reader,
            )));
        }
        let samples = wanted[0].sample.len();
        if wanted.iter().any(|s| s.sample.len() != samples) {
            return Ok(None);
        }
        let matched = (0..samples)
            .filter(|&doc| wanted.iter().any(|s| !s.sample[doc].is_empty()))
            .count();
        Ok(Some(Estimate {
            fraction: matched as f64 / samples.max(1) as f64,
            work: ctx.reader.max_doc() as u64,
        }))
    }
}

impl QueryEstimate for RangeQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        if matches!(self.bounds(), (Bound::Unbounded, Bound::Unbounded)) {
            return Ok(Some(Estimate::prior(1.0, ctx.reader)));
        }
        if ctx.reader.schema().get_field_entry(self.field()).is_fast()
            && !matches!(self.value_type(), Type::Facet | Type::Custom | Type::Vector)
        {
            fast_field::range(self.bounds(), ctx)
        } else {
            ctx.inverted_range(self.bounds()).map(Some)
        }
    }
}

impl QueryEstimate for InvertedIndexRangeQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        ctx.inverted_range(self.bounds()).map(Some)
    }
}

impl QueryEstimate for FastFieldRangeQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        fast_field::range(self.bounds(), ctx)
    }
}

/// Uses the rarer required side, since every match must contain both sides.
impl QueryEstimate for ProximityQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        self.sides(self.left(), self.right(), ctx)
    }
}

impl QueryEstimate for MoreLikeThisQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        // The rewritten document is unavailable here, so use a 1% fallback.
        Ok(Some(Estimate::prior(0.01, ctx.reader)))
    }
}

impl QueryEstimate for super::more_like_this::MoreLikeThisQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        // The rewritten document is unavailable here, so use a 1% fallback.
        Ok(Some(Estimate::prior(0.01, ctx.reader)))
    }
}

impl QueryEstimate for UnresolvedQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate::prior(
            crate::PARAMETERIZED_SELECTIVITY,
            ctx.reader,
        )))
    }
}

/// Scores have no stored distribution, so assume one third of candidates survive the bound.
impl QueryEstimate for super::score::ScoreFilter {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let Some(mut inner) = self.query.as_ref().estimate_docs(ctx)? else {
            return Ok(None);
        };
        if self.bounds.is_empty() {
            inner.fraction = 0.0;
        } else if !self.bounds.contains(&(Bound::Unbounded, Bound::Unbounded)) {
            inner.fraction /= 3.0;
        }
        Ok(Some(inner))
    }
}

/// PostgreSQL estimates the original heap predicates; their selectivity never reduces index work.
impl QueryEstimate for super::heap_field_filter::HeapFilterQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let Some(mut inner) = self.indexed_query.as_ref().estimate_docs(ctx)? else {
            return Ok(None);
        };
        let mut filters: Vec<&super::heap_field_filter::HeapFieldFilter> = Vec::new();
        for filter in self.always_filters.iter().chain(&self.recheck_filters) {
            if !filters.as_slice().contains(&filter) {
                filters.push(filter);
            }
        }
        if filters.is_empty() {
            return Ok(Some(inner));
        }
        let fraction = if let Some((root, relid)) = ctx.planner {
            unsafe {
                let mut clauses = pgrx::list::PgList::<pgrx::pg_sys::Node>::new();
                for filter in &filters {
                    let expression = filter.get_expression_node();
                    if expression.is_null() {
                        return Ok(Some(Estimate {
                            fraction: inner.fraction * 0.5,
                            ..inner
                        }));
                    }
                    clauses.push(expression);
                }
                pgrx::pg_sys::clauselist_selectivity(
                    root,
                    clauses.as_ptr(),
                    relid as i32,
                    pgrx::pg_sys::JoinType::JOIN_INNER,
                    std::ptr::null_mut(),
                )
            }
        } else {
            0.5
        };
        inner.work = inner
            .work
            .saturating_add((inner.fraction * ctx.reader.max_doc() as f64).ceil() as u64);
        inner.fraction *= fraction.clamp(0.0, 1.0);
        Ok(Some(inner))
    }
}

impl Query for UnresolvedQuery {
    fn weight(&self, _: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        Err(tantivy::TantivyError::InvalidArgument(
            "PostgreSQL query expression has not been evaluated".into(),
        ))
    }
}

pub(crate) fn estimate_docs(
    query: &dyn Query,
    reader: &SegmentReader,
    planner: Option<(*mut pgrx::pg_sys::PlannerInfo, pgrx::pg_sys::Index)>,
) -> tantivy::Result<Option<(u32, u64)>> {
    // Old segments retain the old planner, including queries which only need term frequencies.
    let Some(stats) = SegmentStats::of_reader(reader).ok().flatten() else {
        return Ok(None);
    };
    let Some(manifest) = stats.distributions().ok().flatten() else {
        return Ok(None);
    };
    if reader.max_doc() == 0 {
        return Ok(Some((0, 0)));
    }
    let ctx = Context {
        reader,
        planner: planner.filter(|(root, _)| !root.is_null()),
        stats,
        manifest,
        distributions: Default::default(),
    };
    Ok(query.estimate_docs(&ctx)?.map(|estimate| {
        (
            (estimate.fraction.clamp(0.0, 1.0) * reader.max_doc() as f64).ceil() as u32,
            estimate.work,
        )
    }))
}

impl Estimate {
    fn prior(fraction: f64, reader: &SegmentReader) -> Self {
        Self {
            fraction,
            work: reader.max_doc() as u64,
        }
    }
    fn count(count: u32, work: u64, reader: &SegmentReader) -> Self {
        Self {
            fraction: if reader.max_doc() == 0 {
                0.0
            } else {
                count as f64 / reader.max_doc() as f64
            },
            work,
        }
    }

    fn phrase(terms: &[Self], slop: u32, reader: &SegmentReader) -> Self {
        let candidates = Estimate::combine(
            terms.iter().map(|&term| (Occur::Must, term)),
            0,
            reader.max_doc(),
        )
        .fraction;
        let checks = (10 * terms.len()).max(1) as f64;
        let work = terms
            .iter()
            .fold(0u64, |work, term| work.saturating_add(term.work));
        Self {
            fraction: candidates * ((slop as f64 + 1.0) / checks).min(1.0),
            work: work
                .saturating_add((candidates * reader.max_doc() as f64 * checks).ceil() as u64),
        }
    }
    fn combine(
        clauses: impl IntoIterator<Item = (Occur, Estimate)>,
        minimum_should_match: usize,
        max_doc: u32,
    ) -> Estimate {
        let mut must = 1.0;
        let mut excludes = 1.0;
        let mut should = Vec::new();
        let mut required_work = None;
        let mut optional_work = 0u64;
        for (occur, estimate) in clauses {
            match occur {
                Occur::Must => {
                    must *= estimate.fraction;
                    required_work = Some(
                        required_work.map_or(estimate.work, |work: u64| work.min(estimate.work)),
                    );
                }
                Occur::Should => {
                    should.push(estimate.fraction);
                    optional_work = optional_work.saturating_add(estimate.work);
                }
                Occur::MustNot => excludes *= 1.0 - estimate.fraction,
            }
        }
        let needed = if required_work.is_some() {
            minimum_should_match
        } else {
            minimum_should_match.max(1)
        };
        let optional = if needed == 0 {
            1.0
        } else if needed > should.len() {
            0.0
        } else {
            let mut probabilities = vec![0.0; needed + 1];
            probabilities[0] = 1.0;
            for p in should {
                probabilities[needed] += probabilities[needed - 1] * p;
                for k in (1..needed).rev() {
                    probabilities[k] = probabilities[k] * (1.0 - p) + probabilities[k - 1] * p;
                }
                probabilities[0] *= 1.0 - p;
            }
            probabilities[needed]
        };
        let fraction = (must * optional * excludes).clamp(0.0, 1.0);
        let work = required_work
            .unwrap_or(optional_work)
            .max((fraction * max_doc as f64).ceil() as u64);
        Estimate { fraction, work }
    }
}

impl Context<'_> {
    fn distribution(&self, ordinal: usize) -> Option<Arc<Distribution>> {
        self.distributions
            .borrow_mut()
            .entry(ordinal)
            .or_insert_with(|| {
                self.stats
                    .distribution(&self.manifest, ordinal)
                    .ok()?
                    .map(Arc::new)
            })
            .clone()
    }

    fn term(&self, term: &Term) -> tantivy::Result<Estimate> {
        let count = self.reader.inverted_index(term.field())?.doc_freq(term)?;
        Ok(Estimate::count(count, count as u64, self.reader))
    }

    fn matching_terms(
        &self,
        field: Field,
        prefix: &[u8],
        matches: impl Fn(&[u8]) -> bool,
        max_expansions: usize,
    ) -> tantivy::Result<Estimate> {
        let end = prefix_end(prefix);
        self.terms_in_range(
            field,
            (
                Bound::Included(prefix),
                end.as_deref().map_or(Bound::Unbounded, Bound::Excluded),
            ),
            matches,
            max_expansions,
            0.01,
        )
    }

    fn terms_in_range(
        &self,
        field: Field,
        bounds: (Bound<&[u8]>, Bound<&[u8]>),
        matches: impl Fn(&[u8]) -> bool,
        max_expansions: usize,
        fallback: f64,
    ) -> tantivy::Result<Estimate> {
        use tantivy_common::HasLen;
        const TERMS: usize = 4096;
        const BYTES: usize = 1 << 20;
        let inverted = self.reader.inverted_index(field)?;
        let dictionary = inverted.terms();
        let prior = Estimate::prior(fallback, self.reader);
        if dictionary
            .file_slice_for_range(bounds, Some((TERMS + 1) as u64))?
            .len()
            > BYTES
        {
            return Ok(prior);
        }
        let mut stream = dictionary.range();
        match bounds.0 {
            Bound::Included(v) => stream = stream.ge(v),
            Bound::Excluded(v) => stream = stream.gt(v),
            Bound::Unbounded => {}
        }
        match bounds.1 {
            Bound::Included(v) => stream = stream.le(v),
            Bound::Excluded(v) => stream = stream.lt(v),
            Bound::Unbounded => {}
        }
        let mut stream = stream.limit((TERMS + 1) as u64).into_stream()?;
        let mut terms = 0;
        let mut bytes = 0;
        let mut clauses = Vec::new();
        while stream.advance() {
            terms += 1;
            bytes += stream.key().len();
            if terms > TERMS || bytes > BYTES {
                return Ok(prior);
            }
            if matches(stream.key()) {
                if clauses.len() == max_expansions {
                    return Ok(prior);
                }
                let count = stream.value().doc_freq;
                clauses.push((
                    Occur::Should,
                    Estimate::count(count, count as u64, self.reader),
                ));
            }
        }
        Ok(Estimate::combine(clauses, 1, self.reader.max_doc()))
    }

    fn regex(&self, query: &RegexQuery, limit: usize) -> tantivy::Result<Estimate> {
        use tantivy_fst::Automaton;
        let automaton = query.regex();
        let prefix = automaton_prefix(automaton);
        self.matching_terms(
            query.field(),
            &prefix,
            |bytes| {
                let mut state = automaton.start();
                for &byte in bytes {
                    state = automaton.accept(&state, byte);
                    if !automaton.can_match(&state) {
                        return false;
                    }
                }
                automaton.is_match(&state)
            },
            limit,
        )
    }

    fn inverted_range(&self, bounds: (&Bound<Term>, &Bound<Term>)) -> tantivy::Result<Estimate> {
        let bounds = (bounds.0.as_ref(), bounds.1.as_ref());
        let term = match bounds {
            (Bound::Included(t) | Bound::Excluded(t), _)
            | (_, Bound::Included(t) | Bound::Excluded(t)) => t,
            _ => return Ok(Estimate::prior(1.0, self.reader)),
        };
        self.terms_in_range(
            term.field(),
            (
                bounds.0.map(Term::serialized_value_bytes),
                bounds.1.map(Term::serialized_value_bytes),
            ),
            |_| true,
            usize::MAX,
            1.0 / 3.0,
        )
    }
}

impl ProximityQuery {
    fn sides(
        &self,
        left: &ProximityClause,
        right: &ProximityClause,
        ctx: &Context<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        let (Some(left), Some(right)) = (self.clause(left, ctx)?, self.clause(right, ctx)?) else {
            return Ok(None);
        };
        let fraction = left.fraction.min(right.fraction);
        Ok(Some(Estimate {
            fraction,
            work: left
                .work
                .saturating_add(right.work)
                .saturating_add((fraction * ctx.reader.max_doc() as f64).ceil() as u64),
        }))
    }
    fn clause(
        &self,
        clause: &ProximityClause,
        ctx: &Context<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        match clause {
            ProximityClause::Uninitialized => Ok(Some(Estimate::count(0, 0, ctx.reader))),
            ProximityClause::Term(word) => TermQuery::new(
                Term::from_field_text(self.field(), word),
                IndexRecordOption::Basic,
            )
            .estimate_docs(ctx),
            ProximityClause::Regex {
                pattern,
                max_expansions,
            } => ctx
                .regex(
                    &RegexQuery::from_pattern(pattern.as_str(), self.field())?,
                    *max_expansions,
                )
                .map(Some),
            ProximityClause::Clauses(clauses) => {
                let mut estimates = Vec::new();
                for clause in clauses {
                    let Some(child) = self.clause(clause, ctx)? else {
                        return Ok(None);
                    };
                    estimates.push((Occur::Should, child));
                }
                Ok(Some(Estimate::combine(estimates, 1, ctx.reader.max_doc())))
            }
            ProximityClause::Proximity { left, right, .. } => self.sides(left, right, ctx),
        }
    }
}

fn prefix_end(prefix: &[u8]) -> Option<Vec<u8>> {
    let mut end = prefix.to_vec();
    while let Some(byte) = end.pop() {
        if byte < u8::MAX {
            end.push(byte + 1);
            return Some(end);
        }
    }
    None
}

fn automaton_prefix(automaton: &impl tantivy_fst::Automaton) -> Vec<u8> {
    let mut prefix = Vec::new();
    let mut state = automaton.start();
    while prefix.len() < 64 && !automaton.is_match(&state) {
        let mut next = None;
        for byte in 0..=u8::MAX {
            let next_state = automaton.accept(&state, byte);
            if automaton.can_match(&next_state) {
                if next.is_some() {
                    return prefix;
                }
                next = Some((byte, next_state));
            }
        }
        let Some((byte, next_state)) = next else {
            break;
        };
        prefix.push(byte);
        state = next_state;
    }
    prefix
}

mod fast_field;

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::query::more_like_this::MoreLikeThisQueryBuilder;
    use crate::query::proximity::{ProximityClause, ProximityDistance, query::ProximityQuery};
    use crate::query::score::ScoreFilter;
    use pgrx::pg_test;
    use std::ops::Bound::{Excluded, Included, Unbounded};
    use tantivy::schema::{FAST, INDEXED, STRING, Schema, TEXT};
    use tantivy::{Index, IndexWriter, TantivyDocument, doc};

    fn test_index(
        schema: Schema,
        docs: impl IntoIterator<Item = TantivyDocument>,
        stats: bool,
    ) -> Index {
        let mut builder = Index::builder().schema(schema);
        if stats {
            builder =
                builder.register_plugin(std::sync::Arc::new(crate::index::stats::StatsPlugin));
        }
        let index = builder.create_in_ram().unwrap();
        let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
        for doc in docs {
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
        index
    }

    fn fixture(stats: bool) -> (Index, Field, Field, Field, Field) {
        let mut schema = Schema::builder();
        let text = schema.add_text_field("text", TEXT);
        let literal = schema.add_text_field("literal", STRING);
        let number = schema.add_u64_field("number", FAST);
        let label = schema.add_text_field("label", FAST);
        let docs = [
            ("red cat", 10u64, "apple"),
            ("red dog", 20, "banana"),
            ("blue cat", 30, "cherry"),
            ("green fish", 40, "date"),
        ].map(|(words, value, name)| doc!(text => words, literal => words, number => value, label => name));
        (
            test_index(schema.build(), docs, stats),
            text,
            literal,
            number,
            label,
        )
    }

    fn estimate_query(index: &Index, query: &dyn Query) -> Option<(u32, u64)> {
        let reader = index.reader().unwrap();
        estimate_docs(query, &reader.searcher().segment_readers()[0], None).unwrap()
    }

    fn term(field: Field, word: &str) -> Box<dyn Query> {
        Box::new(TermQuery::new(
            Term::from_field_text(field, word),
            IndexRecordOption::Basic,
        ))
    }

    #[pg_test]
    fn term_counts_do_not_need_frequencies_or_positions() {
        let (index, _, literal, _, _) = fixture(true);
        for (word, count) in [("red cat", 1), ("absent", 0)] {
            assert_eq!(
                estimate_query(&index, term(literal, word).as_ref()),
                Some((count, count as u64))
            );
        }
    }

    #[pg_test]
    fn fast_ranges_and_string_endpoints() {
        let (index, _, _, number, label) = fixture(true);
        for (lower, upper, count) in [
            (Included(20), Excluded(40), 2),
            (Excluded(20), Included(40), 2),
            (Unbounded, Excluded(10), 0),
            (Included(41), Unbounded, 0),
        ] {
            let query = RangeQuery::new(
                lower.map(|v| Term::from_field_u64(number, v)),
                upper.map(|v| Term::from_field_u64(number, v)),
            );
            assert_eq!(estimate_query(&index, &query), Some((count, 4)));
        }
        for (lower, upper, count) in [
            (Included("banana"), Excluded("date"), 2),
            (Excluded("banana"), Included("date"), 2),
            (Included("blueberry"), Included("cranberry"), 1),
            (Included("absent"), Included("absent"), 0),
        ] {
            let query = FastFieldRangeQuery::new(
                lower.map(|v| Term::from_field_text(label, v)),
                upper.map(|v| Term::from_field_text(label, v)),
            );
            assert_eq!(estimate_query(&index, &query), Some((count, 4)));
        }
    }

    #[pg_test]
    fn nested_wrappers_and_parsed_queries_use_native_estimates() {
        let (index, text, _, number, _) = fixture(true);
        let query = BooleanQuery::new(vec![
            (Occur::Must, term(text, "red")),
            (
                Occur::Must,
                Box::new(RangeQuery::new(
                    Included(Term::from_field_u64(number, 20)),
                    Unbounded,
                )),
            ),
        ]);
        let plain = estimate_query(&index, &query);
        let wrapped =
            ConstScoreQuery::new(BoostQuery::new(ConstScoreQuery::new(query, 7.0), 2.0), 9.0);
        assert_eq!(estimate_query(&index, &wrapped), plain);
        let parser = QueryParser::for_index(&index, vec![text]);
        for expression in [
            "red",
            "red AND cat",
            "red OR blue",
            "\"red cat\"",
            "red AND NOT dog",
            "number:[20 TO 40]",
        ] {
            assert!(
                estimate_query(&index, parser.parse_query(expression).unwrap().as_ref()).is_some(),
                "{expression}"
            );
        }
    }

    #[pg_test]
    fn boolean_minimum_should_match_and_exclusions() {
        let half = Estimate {
            fraction: 0.5,
            work: 50,
        };
        for (needed, expected) in [(1, 0.875), (2, 0.5), (3, 0.125), (4, 0.0)] {
            let result = Estimate::combine([(Occur::Should, half); 3], needed, 100);
            assert_eq!((result.fraction, result.work), (expected, 150));
        }
        for (occurs, expected) in [
            (vec![Occur::Must, Occur::MustNot], 0.25),
            (vec![Occur::Must, Occur::Should], 0.5),
            (vec![Occur::MustNot], 0.0),
        ] {
            assert_eq!(
                Estimate::combine(occurs.into_iter().map(|occur| (occur, half)), 0, 100).fraction,
                expected
            );
        }
    }

    #[pg_test]
    fn every_native_leaf_has_an_estimate() {
        let (index, text, _, number, label) = fixture(true);
        let words = || {
            vec![
                Term::from_field_text(text, "red"),
                Term::from_field_text(text, "cat"),
            ]
        };
        let queries: &[&dyn Query] = &[
            &AllQuery,
            &EmptyQuery,
            &TermSetQuery::new(words()),
            &DisjunctionMaxQuery::new(vec![term(text, "red"), term(text, "cat")]),
            &PhraseQuery::new(words()),
            &PhrasePrefixQuery::new(words()),
            &PhrasePrefixQuery::new(vec![Term::from_field_text(text, "ca")]),
            &RegexQuery::from_pattern("c.*", text).unwrap(),
            &RegexPhraseQuery::new(text, vec!["r.*".into(), "c.*".into()]),
            &FuzzyTermQuery::new(Term::from_field_text(text, "cot"), 1, true),
            &ExistsQuery::new("number".into(), false),
            &ExistsQuery::new("label".into(), false),
            &InvertedIndexRangeQuery::new(
                Included(Term::from_field_text(text, "cat")),
                Included(Term::from_field_text(text, "dog")),
            ),
            &RangeQuery::new(Included(Term::from_field_u64(number, 20)), Unbounded),
            &FastFieldRangeQuery::new(Included(Term::from_field_text(label, "banana")), Unbounded),
            &ProximityQuery::new(
                text,
                ProximityClause::Term("red".into()),
                ProximityDistance::AnyOrder(2),
                ProximityClause::Term("cat".into()),
            ),
            &ScoreFilter::new(vec![(Excluded(0.0), Unbounded)], term(text, "red")),
            &UnresolvedQuery,
        ];
        for query in queries {
            let (count, _) = estimate_query(&index, *query).unwrap_or_else(|| panic!("{query:?}"));
            assert!(count <= 4, "{query:?}");
        }
    }

    #[pg_test]
    fn missing_statistics_disables_the_whole_new_path() {
        let (index, text, _, _, _) = fixture(false);
        let query = BooleanQuery::new(vec![
            (Occur::Should, term(text, "red")),
            (Occur::Should, Box::new(AllQuery)),
        ]);
        assert_eq!(estimate_query(&index, &query), None);
    }

    #[pg_test]
    fn arrays_and_json_types_count_documents() {
        let mut schema = Schema::builder();
        let values = schema.add_u64_field("values", FAST);
        let json = schema.add_json_field("json", FAST | TEXT);
        let schema = schema.build();
        let docs = [
            r#"{"values":[1,1,2],"json":{"a":[1,2],"b":"x"}}"#,
            r#"{"values":[3],"json":{"a":"one"}}"#,
            r#"{"json":{"b":"y"}}"#,
        ]
        .map(|source| TantivyDocument::parse_json(&schema, source).unwrap());
        let index = test_index(schema, docs, true);
        let mut bound = Term::from_field_json_path(json, "a", false);
        bound.append_type_and_fast_value(1u64);
        let queries: &[(&dyn Query, u32)] = &[
            (
                &RangeQuery::new(
                    Included(Term::from_field_u64(values, 1)),
                    Included(Term::from_field_u64(values, 2)),
                ),
                1,
            ),
            (&ExistsQuery::new("json.a".into(), false), 2),
            (&ExistsQuery::new("json".into(), true), 3),
            (
                &FastFieldRangeQuery::new(Included(bound.clone()), Included(bound)),
                1,
            ),
        ];
        for (query, count) in queries {
            assert_eq!(
                estimate_query(&index, *query).unwrap().0,
                *count,
                "{query:?}"
            );
        }
    }

    #[pg_test]
    fn broad_regex_stops_at_the_dictionary_budget() {
        let mut schema = Schema::builder();
        let text = schema.add_text_field("text", STRING);
        schema.add_u64_field("id", FAST | INDEXED);
        let index = test_index(
            schema.build(),
            (0..5000).map(|i| doc!(text => format!("word{i:05}"))),
            true,
        );
        assert_eq!(
            estimate_query(&index, &RegexQuery::from_pattern(".*", text).unwrap()),
            Some((50, 5000))
        );
    }

    #[pg_test]
    fn missing_required_summary_does_not_produce_a_partial_estimate() {
        let mut schema = Schema::builder();
        let text = schema.add_text_field("text", STRING);
        let values = schema.add_u64_field("values", FAST);
        let mut document = doc!(text => "red");
        for value in 0..20000 {
            document.add_u64(values, value);
        }
        let index = test_index(schema.build(), [document], true);
        assert_eq!(
            estimate_query(&index, term(text, "red").as_ref()),
            Some((1, 1))
        );
        let query = BooleanQuery::new(vec![
            (Occur::Must, term(text, "red")),
            (
                Occur::Must,
                Box::new(RangeQuery::new(
                    Included(Term::from_field_u64(values, 1)),
                    Unbounded,
                )),
            ),
        ]);
        assert_eq!(estimate_query(&index, &query), None);
    }

    #[pg_test]
    fn estimation_does_not_build_weights() {
        #[derive(Debug, Clone)]
        struct MetadataOnly(Box<dyn Query>);
        impl Query for MetadataOnly {
            fn weight(&self, _: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
                panic!("estimation constructed a weight")
            }
            fn matching_query(&self) -> Option<&dyn Query> {
                Some(self.0.as_ref())
            }
        }
        let (index, text, _, _, _) = fixture(true);
        let query = BoostQuery::new(
            ConstScoreQuery::new(MetadataOnly(term(text, "red")), 1.0),
            2.0,
        );
        assert_eq!(estimate_query(&index, &query), Some((2, 2)));
    }

    #[pg_test]
    fn fast_range_value_types_preserve_their_order() {
        let mut schema = Schema::builder();
        let signed = schema.add_i64_field("signed", FAST);
        let float = schema.add_f64_field("float", FAST);
        let boolean = schema.add_bool_field("boolean", FAST);
        let date = schema.add_date_field("date", FAST);
        let ip = schema.add_ip_addr_field("ip", FAST);
        let bytes = schema.add_bytes_field("bytes", FAST);
        let facet = schema.add_facet_field("facet", tantivy::schema::FacetOptions::default());
        let docs = (0..4).map(|i| {
            doc!(signed => i - 2, float => i as f64 - 1.5,
            boolean => i % 2 == 0, date => tantivy::DateTime::from_timestamp_secs(i),
            ip => std::net::Ipv6Addr::from(i as u128), bytes => vec![i as u8],
            facet => tantivy::schema::Facet::from(&format!("/value{i}")))
        });
        let index = test_index(schema.build(), docs, true);
        for (bound, equality, count) in [
            (Term::from_field_i64(signed, 0), false, 2),
            (Term::from_field_f64(float, 0.5), false, 2),
            (
                Term::from_field_date(date, tantivy::DateTime::from_timestamp_secs(2)),
                false,
                2,
            ),
            (
                Term::from_field_ip_addr(ip, std::net::Ipv6Addr::from(2)),
                false,
                2,
            ),
            (Term::from_field_bytes(bytes, &[2]), false, 2),
            (Term::from_field_bool(boolean, true), true, 2),
            (
                Term::from_facet(facet, &tantivy::schema::Facet::from("/value2")),
                true,
                1,
            ),
        ] {
            let upper = if equality {
                Included(bound.clone())
            } else {
                Unbounded
            };
            let query = RangeQuery::new(Included(bound), upper);
            assert_eq!(
                estimate_query(&index, &query).unwrap().0,
                count,
                "{query:?}"
            );
        }
    }

    #[pg_test]
    fn metadata_estimation_never_fetches_mlt_document() {
        let (index, _, _, _, _) = fixture(true);
        let query = MoreLikeThisQueryBuilder::new(Default::default(), None).with_field_value(
            "missing".into(),
            crate::postgres::pdb_owned_value::PdbOwnedValue::U64(1),
            None,
            pgrx::pg_sys::Oid::INVALID,
        );
        assert_eq!(estimate_query(&index, &query), Some((1, 4)));
    }
}
