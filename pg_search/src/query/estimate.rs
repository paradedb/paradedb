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

use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops::Bound;

use crate::index::stats::{
    SegmentStats,
    distribution::{Distribution, DistributionManifest},
};
use tantivy::SegmentReader;
use tantivy::query::*;
use tantivy::schema::{Field, Term, Type};

/// Estimates queries using pg_search's shared statistics and planner context.
trait QueryEstimate {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>>;
}

#[derive(Clone, Copy, Debug)]
struct Estimate {
    fraction: f64,
    work: u64,
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
}

struct Context<'a> {
    reader: &'a SegmentReader,
    planner: Option<(*mut pgrx::pg_sys::PlannerInfo, pgrx::pg_sys::Index)>,
    stats: OnceCell<Option<(SegmentStats, DistributionManifest)>>,
    distributions: std::cell::RefCell<HashMap<usize, Option<std::sync::Arc<Distribution>>>>,
}

impl<'a> Context<'a> {
    fn stats(&self) -> Option<&(SegmentStats, DistributionManifest)> {
        self.stats
            .get_or_init(|| {
                let stats = SegmentStats::of_reader(self.reader).ok()??;
                let manifest = stats.distributions().ok()??;
                Some((stats, manifest))
            })
            .as_ref()
    }
    fn distribution(&self, ordinal: usize) -> Option<std::sync::Arc<Distribution>> {
        let (stats, manifest) = self.stats()?;
        self.distributions
            .borrow_mut()
            .entry(ordinal)
            .or_insert_with(|| {
                stats
                    .distribution(manifest, ordinal)
                    .ok()?
                    .map(std::sync::Arc::new)
            })
            .clone()
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
    let mut has_must = false;
    for (occur, estimate) in clauses {
        match occur {
            Occur::Must => {
                has_must = true;
                must *= estimate.fraction;
                required_work =
                    Some(required_work.map_or(estimate.work, |work: u64| work.min(estimate.work)));
            }
            Occur::Should => {
                should.push(estimate.fraction);
                optional_work = optional_work.saturating_add(estimate.work);
            }
            Occur::MustNot => excludes *= 1.0 - estimate.fraction,
        }
    }
    let needed = if has_must {
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

fn terms_matching(
    field: Field,
    prefix: &[u8],
    matches: impl Fn(&[u8]) -> bool,
    max_expansions: usize,
    ctx: &Context<'_>,
) -> tantivy::Result<Option<Estimate>> {
    use tantivy_common::HasLen;
    const TERMS: usize = 4096;
    const BYTES: usize = 1 << 20;
    let inverted = ctx.reader.inverted_index(field)?;
    let dictionary = inverted.terms();
    let end = prefix_end(prefix);
    let upper = end.as_deref().map_or(Bound::Unbounded, Bound::Excluded);
    if dictionary
        .file_slice_for_range((Bound::Included(prefix), upper), Some((TERMS + 1) as u64))?
        .len()
        > BYTES
    {
        return Ok(Some(Estimate::prior(0.01, ctx.reader)));
    }
    let mut stream = dictionary.range().ge(prefix);
    if let Some(end) = &end {
        stream = stream.lt(end);
    }
    let mut stream = stream.limit((TERMS + 1) as u64).into_stream()?;
    let mut terms = 0;
    let mut bytes = 0;
    let mut clauses = Vec::new();
    while stream.advance() {
        terms += 1;
        bytes += stream.key().len();
        if terms > TERMS || bytes > BYTES {
            return Ok(Some(Estimate::prior(0.01, ctx.reader)));
        }
        if matches(stream.key()) {
            if clauses.len() == max_expansions {
                return Ok(Some(Estimate::prior(0.01, ctx.reader)));
            }
            let count = stream.value().doc_freq;
            clauses.push((
                Occur::Should,
                Estimate::count(count, count as u64, ctx.reader),
            ));
        }
    }
    Ok(Some(combine(clauses, 1, ctx.reader.max_doc())))
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
            return fast_field::range(
                (
                    &Bound::Included(term.clone()),
                    &Bound::Included(term.clone()),
                ),
                ctx,
            );
        }
        let count = ctx.reader.inverted_index(term.field())?.doc_freq(term)?;
        Ok(Some(Estimate::count(count, count as u64, ctx.reader)))
    }
}

/// Combines child probabilities assuming independence, including the required number of optional matches.
impl QueryEstimate for BooleanQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut children = Vec::new();
        for (occur, child) in self.clauses() {
            let Some(child) = QueryEstimate::estimate_docs(child.as_ref(), ctx)? else {
                return Ok(None);
            };
            children.push((*occur, child));
        }
        Ok(Some(combine(
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
            let Some(child) = QueryEstimate::estimate_docs(child.as_ref(), ctx)? else {
                return Ok(None);
            };
            children.push((Occur::Should, child));
        }
        Ok(Some(combine(children, 1, ctx.reader.max_doc())))
    }
}

/// Estimates the union of the individual terms using their document frequencies.
impl QueryEstimate for TermSetQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut children = Vec::new();
        for term in self.terms() {
            let query = TermQuery::new(term.clone(), tantivy::schema::IndexRecordOption::Basic);
            let Some(child) = QueryEstimate::estimate_docs(&query, ctx)? else {
                return Ok(None);
            };
            children.push((Occur::Should, child));
        }
        Ok(Some(combine(children, 1, ctx.reader.max_doc())))
    }
}

/// Multiplies the term probabilities and discounts for words having to occur together.
impl QueryEstimate for PhraseQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut clauses = Vec::new();
        let terms = self.phrase_terms();
        let mut work = 0u64;
        for term in &terms {
            let count = ctx.reader.inverted_index(term.field())?.doc_freq(term)?;
            work = work.saturating_add(count as u64);
            clauses.push((
                Occur::Must,
                Estimate::count(count, count as u64, ctx.reader),
            ));
        }
        let candidates = combine(clauses, 0, ctx.reader.max_doc()).fraction;
        let checks = (10 * terms.len()).max(1) as u64;
        let fraction = candidates * ((self.slop() as f64 + 1.0) / checks as f64).min(1.0);
        work = work.saturating_add(
            (candidates * ctx.reader.max_doc() as f64 * checks as f64).ceil() as u64,
        );
        Ok(Some(Estimate { fraction, work }))
    }
}

/// Uses the rarest complete word; a prefix alone uses a bounded dictionary lookup.
impl QueryEstimate for PhrasePrefixQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let terms = self.phrase_terms();
        if terms.is_empty() {
            return terms_matching(
                self.field(),
                self.prefix().serialized_value_bytes(),
                |_| true,
                self.max_expansions() as usize,
                ctx,
            );
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
        estimate_regex(self, ctx, usize::MAX)
    }
}
fn estimate_regex(
    query: &RegexQuery,
    ctx: &Context<'_>,
    limit: usize,
) -> tantivy::Result<Option<Estimate>> {
    use tantivy_fst::Automaton;
    let automaton = query.regex();
    let prefix = automaton_prefix(automaton);
    terms_matching(
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
        ctx,
    )
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
        if word.len() > 256 {
            return Ok(Some(Estimate::prior(0.01, ctx.reader)));
        }
        if self.distance() > 2 {
            return Ok(Some(Estimate::prior(0.01, ctx.reader)));
        }
        if self.distance() == 0 && !self.prefix() {
            return QueryEstimate::estimate_docs(
                &TermQuery::new(term.clone(), tantivy::schema::IndexRecordOption::Basic),
                ctx,
            );
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
        terms_matching(
            term.field(),
            path,
            |term| matches!(dfa.eval(term), Distance::Exact(_)),
            usize::MAX,
            ctx,
        )
    }
}

/// Combines the regex word estimates and discounts for their required positions.
impl QueryEstimate for RegexPhraseQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let mut children = Vec::new();
        let mut cost = 0u64;
        for term in self.phrase_terms() {
            let pattern = term.value();
            let Some(pattern) = pattern.as_str() else {
                return Ok(None);
            };
            let query = RegexQuery::from_pattern(pattern, self.field())?;
            let Some(child) = estimate_regex(&query, ctx, self.max_expansions() as usize)? else {
                return Ok(None);
            };
            cost = cost.saturating_add(child.work);
            children.push((Occur::Must, child));
        }
        let checks = (children.len() * 10).max(1) as f64;
        let candidates = combine(children, 0, ctx.reader.max_doc()).fraction;
        let fraction = candidates * ((self.slop() as f64 + 1.0) / checks).min(1.0);
        Ok(Some(Estimate {
            fraction,
            work: cost
                .saturating_add((candidates * ctx.reader.max_doc() as f64 * checks).ceil() as u64),
        }))
    }
}

/// Uses the document presence counts, or the aligned samples when several JSON columns qualify.
impl QueryEstimate for ExistsQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let Some((_, manifest)) = ctx.stats() else {
            return Ok(None);
        };
        let mut wanted = Vec::new();
        // Resolve schema names using the fast-field reader, including escaped JSON paths.
        let Some(canonical) = ctx.reader.fast_fields().resolve_field(self.field_name())? else {
            return Ok(Some(Estimate::count(0, 0, ctx.reader)));
        };
        let prefix = format!("{canonical}\u{1}");
        for (ordinal, (name, _)) in manifest.columns.iter().enumerate() {
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
            fraction: if samples == 0 {
                0.0
            } else {
                matched as f64 / samples as f64
            },
            work: ctx.reader.max_doc() as u64,
        }))
    }
}

/// Uses the rarer required side, since every match must contain both sides.
impl QueryEstimate for crate::query::proximity::query::ProximityQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        self.sides(self.left(), self.right(), ctx)
    }
}
impl crate::query::proximity::query::ProximityQuery {
    fn sides(
        &self,
        left: &crate::query::proximity::ProximityClause,
        right: &crate::query::proximity::ProximityClause,
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
        clause: &crate::query::proximity::ProximityClause,
        ctx: &Context<'_>,
    ) -> tantivy::Result<Option<Estimate>> {
        use crate::query::proximity::ProximityClause;
        match clause {
            ProximityClause::Uninitialized => Ok(Some(Estimate::count(0, 0, ctx.reader))),
            ProximityClause::Term(word) => QueryEstimate::estimate_docs(
                &TermQuery::new(
                    Term::from_field_text(self.field(), word),
                    tantivy::schema::IndexRecordOption::Basic,
                ),
                ctx,
            ),
            ProximityClause::Regex {
                pattern,
                max_expansions,
            } => estimate_regex(
                &RegexQuery::from_pattern(pattern.as_str(), self.field())?,
                ctx,
                *max_expansions,
            ),
            ProximityClause::Clauses(clauses) => {
                let mut estimates = Vec::new();
                for clause in clauses {
                    let Some(child) = self.clause(clause, ctx)? else {
                        return Ok(None);
                    };
                    estimates.push((Occur::Should, child));
                }
                Ok(Some(combine(estimates, 1, ctx.reader.max_doc())))
            }
            ProximityClause::Proximity { left, right, .. } => self.sides(left, right, ctx),
        }
    }
}

pub(crate) fn estimate_docs(
    query: &dyn Query,
    reader: &SegmentReader,
    planner: Option<(*mut pgrx::pg_sys::PlannerInfo, pgrx::pg_sys::Index)>,
) -> tantivy::Result<Option<(u32, u64)>> {
    let ctx = Context {
        reader,
        planner: planner.filter(|(root, _)| !root.is_null()),
        stats: OnceCell::new(),
        distributions: Default::default(),
    };
    // Old segments retain the old planner, including queries which only need term frequencies.
    if ctx.stats().is_none() {
        return Ok(None);
    }
    if reader.max_doc() == 0 {
        return Ok(Some((0, 0)));
    }
    let estimate = QueryEstimate::estimate_docs(query, &ctx)?;
    Ok(estimate.map(|estimate| {
        (
            (estimate.fraction.clamp(0.0, 1.0) * reader.max_doc() as f64).ceil() as u32,
            estimate.work,
        )
    }))
}

impl QueryEstimate for dyn Query {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        if let Some(inner) = self.matching_query() {
            return QueryEstimate::estimate_docs(inner, ctx);
        }
        macro_rules! estimate_as {
            ($($query:ty),* $(,)?) => {$(
                if let Some(query) = self.downcast_ref::<$query>() {
                    return QueryEstimate::estimate_docs(query, ctx);
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
            crate::query::proximity::query::ProximityQuery,
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

impl QueryEstimate for RangeQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        if matches!(self.bounds(), (Bound::Unbounded, Bound::Unbounded)) {
            return Ok(Some(Estimate::prior(1.0, ctx.reader)));
        }
        if ctx
            .reader
            .schema()
            .get_field_entry(self.field())
            .field_type()
            .is_fast()
            && !matches!(self.value_type(), Type::Facet | Type::Custom | Type::Vector)
        {
            fast_field::range(self.bounds(), ctx)
        } else {
            estimate_inverted_range(self.bounds(), ctx)
        }
    }
}

impl QueryEstimate for InvertedIndexRangeQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        estimate_inverted_range(self.bounds(), ctx)
    }
}

impl QueryEstimate for FastFieldRangeQuery {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        fast_field::range(self.bounds(), ctx)
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

/// Combines frequencies in the term range, or uses a one-third prior when the lookup exceeds its budget.
fn estimate_inverted_range(
    bounds: (&Bound<Term>, &Bound<Term>),
    ctx: &Context<'_>,
) -> tantivy::Result<Option<Estimate>> {
    use tantivy_common::HasLen;
    let bounds = (bounds.0.as_ref(), bounds.1.as_ref());
    let term = match bounds {
        (Bound::Included(t) | Bound::Excluded(t), _)
        | (_, Bound::Included(t) | Bound::Excluded(t)) => t,
        _ => return Ok(Some(Estimate::prior(1.0, ctx.reader))),
    };
    let inverted = ctx.reader.inverted_index(term.field())?;
    let dictionary = inverted.terms();
    let lower = bounds.0.map(Term::serialized_value_bytes);
    let upper = bounds.1.map(Term::serialized_value_bytes);
    if dictionary
        .file_slice_for_range((lower, upper), Some(4097))?
        .len()
        > 1 << 20
    {
        return Ok(Some(Estimate::prior(1.0 / 3.0, ctx.reader)));
    }
    let mut stream = dictionary.range();
    match lower {
        Bound::Included(v) => stream = stream.ge(v),
        Bound::Excluded(v) => stream = stream.gt(v),
        Bound::Unbounded => {}
    }
    match upper {
        Bound::Included(v) => stream = stream.le(v),
        Bound::Excluded(v) => stream = stream.lt(v),
        Bound::Unbounded => {}
    }
    let mut stream = stream.limit(4097).into_stream()?;
    let mut clauses = Vec::new();
    let mut bytes = 0;
    while stream.advance() {
        bytes += stream.key().len();
        if clauses.len() == 4096 || bytes > 1 << 20 {
            return Ok(Some(Estimate::prior(1.0 / 3.0, ctx.reader)));
        }
        let count = stream.value().doc_freq;
        clauses.push((
            Occur::Should,
            Estimate::count(count, count as u64, ctx.reader),
        ));
    }
    Ok(Some(combine(clauses, 1, ctx.reader.max_doc())))
}
#[derive(Debug, Clone)]
pub(super) struct UnresolvedQuery;
impl Query for UnresolvedQuery {
    fn weight(&self, _: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        Err(tantivy::TantivyError::InvalidArgument(
            "PostgreSQL query expression has not been evaluated".into(),
        ))
    }
}

/// Scores have no stored distribution, so assume one third of candidates survive the bound.
impl QueryEstimate for super::score::ScoreFilter {
    fn estimate_docs(&self, ctx: &Context<'_>) -> tantivy::Result<Option<Estimate>> {
        let Some(mut inner) = QueryEstimate::estimate_docs(self.query.as_ref(), ctx)? else {
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
        let Some(mut inner) = QueryEstimate::estimate_docs(self.indexed_query.as_ref(), ctx)?
        else {
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

mod fast_field;
#[cfg(any(test, feature = "pg_test"))]
mod tests;

#[cfg(any(test, feature = "pg_test"))]
mod backend_tests;
