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

//! Per-document matching ranges of a [`ProximityClause`].
//!
//! Unlike the nested-proximity path in `weight.rs`, which flattens a sub-clause into bare
//! positions, a [`SpanSource`] keeps every match as a `[start, end)` range of token positions, so
//! a multi-token clause is known end to end at any nesting depth. `ProximityClause::Not` needs
//! that to decide whether an exclusion touches an occurrence, so a clause that contains one is
//! evaluated here as a whole.

use crate::query::proximity::{ProximityClause, ProximityDistance, WhichTerms};
use std::cmp::Ordering;
use tantivy::postings::Postings;
use tantivy::query::SimpleUnion;
use tantivy::schema::{Field, IndexRecordOption};
use tantivy::{DocId, DocSet, SegmentReader, TERMINATED, Term};

/// A half-open range of token positions, `[start, end)`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Span {
    start: u32,
    end: u32,
}

impl Span {
    /// Number of tokens between `self` and a later `other`; negative when they overlap.
    #[inline(always)]
    fn gap_before(self, other: Span) -> i64 {
        other.start as i64 - self.end as i64
    }
}

/// A docset over the documents in which a clause matches, exposing the matching spans of the
/// current document.
pub enum SpanSource {
    Terms {
        postings: SimpleUnion<Box<dyn Postings>>,
        positions: Vec<u32>,
        spans: Vec<Span>,
        spans_doc: DocId,
    },
    Union {
        sources: Vec<SpanSource>,
        spans: Vec<Span>,
        spans_doc: DocId,
    },
    Near {
        left: Box<SpanSource>,
        right: Box<SpanSource>,
        distance: ProximityDistance,
        spans: Vec<Span>,
        doc: DocId,
    },
    /// The spans of `include` that no span of `exclude` ends at most `pre` tokens before and
    /// starts at most `post` tokens after.
    Not {
        include: Box<SpanSource>,
        exclude: Box<SpanSource>,
        pre: i32,
        post: i32,
        /// Each exclusion start, with the furthest end of the exclusions up to it.
        reach: Vec<(u32, u32)>,
        spans: Vec<Span>,
        doc: DocId,
    },
}

impl SpanSource {
    /// Builds the source for `clause` on one segment, positioned on its first match.
    pub fn build(
        field: Field,
        reader: &SegmentReader,
        clause: &ProximityClause,
    ) -> tantivy::Result<Self> {
        let build = |clause: &ProximityClause| Self::build(field, reader, clause);
        let mut source = match clause {
            ProximityClause::Uninitialized => SpanSource::terms(Vec::new()),
            ProximityClause::Term(_) | ProximityClause::Regex { .. } => {
                let mut postings = Vec::new();
                read_leaf(field, reader, clause, &mut postings)?;
                SpanSource::terms(postings)
            }
            ProximityClause::Clauses(clauses) => {
                // Plain terms share a single union; nested clauses keep their own spans.
                let mut postings = Vec::new();
                let mut sources = Vec::new();
                for clause in clauses {
                    match clause {
                        ProximityClause::Term(_) | ProximityClause::Regex { .. } => {
                            read_leaf(field, reader, clause, &mut postings)?
                        }
                        _ => sources.push(build(clause)?),
                    }
                }
                if !postings.is_empty() || sources.is_empty() {
                    sources.push(SpanSource::terms(postings));
                }
                if sources.len() == 1 {
                    sources.pop().unwrap()
                } else {
                    SpanSource::Union {
                        sources,
                        spans: Vec::new(),
                        spans_doc: TERMINATED,
                    }
                }
            }
            ProximityClause::Proximity {
                left,
                distance,
                right,
            } => SpanSource::Near {
                left: Box::new(build(left)?),
                right: Box::new(build(right)?),
                distance: *distance,
                spans: Vec::new(),
                doc: TERMINATED,
            },
            ProximityClause::Not {
                include,
                exclude,
                pre,
                post,
            } => SpanSource::Not {
                include: Box::new(build(include)?),
                exclude: Box::new(build(exclude)?),
                pre: *pre,
                post: *post,
                reach: Vec::new(),
                spans: Vec::new(),
                doc: TERMINATED,
            },
        };
        source.align();
        Ok(source)
    }

    fn terms(postings: Vec<Box<dyn Postings>>) -> Self {
        SpanSource::Terms {
            postings: SimpleUnion::build(postings),
            positions: Vec::new(),
            spans: Vec::new(),
            spans_doc: TERMINATED,
        }
    }

    /// The spans matched in the current document, sorted and deduplicated.
    pub fn spans(&mut self) -> &[Span] {
        let doc = self.doc();
        match self {
            SpanSource::Terms {
                postings,
                positions,
                spans,
                spans_doc,
            } => {
                if *spans_doc != doc {
                    *spans_doc = doc;
                    spans.clear();
                    if doc != TERMINATED {
                        postings.positions(positions);
                        spans.extend(positions.iter().map(|&p| Span {
                            start: p,
                            end: p + 1,
                        }));
                    }
                }
                spans
            }
            SpanSource::Union {
                sources,
                spans,
                spans_doc,
            } => {
                if *spans_doc != doc {
                    *spans_doc = doc;
                    spans.clear();
                    for source in sources.iter_mut().filter(|source| source.doc() == doc) {
                        spans.extend_from_slice(source.spans());
                    }
                    spans.sort_unstable();
                    spans.dedup();
                }
                spans
            }
            SpanSource::Near { spans, .. } | SpanSource::Not { spans, .. } => spans,
        }
    }

    /// Moves a `Near` or `Not` forward to the first document, at or after the current one of its
    /// driving child, that has at least one span.
    fn align(&mut self) -> DocId {
        match self {
            SpanSource::Near {
                left,
                right,
                distance,
                spans,
                doc,
            } => loop {
                let (l, r) = (left.doc(), right.doc());
                if l == TERMINATED || r == TERMINATED {
                    *doc = TERMINATED;
                    spans.clear();
                    return TERMINATED;
                }
                match l.cmp(&r) {
                    Ordering::Less => {
                        left.seek(r);
                    }
                    Ordering::Greater => {
                        right.seek(l);
                    }
                    Ordering::Equal => {
                        near_spans(left.spans(), right.spans(), *distance, spans);
                        if !spans.is_empty() {
                            *doc = l;
                            return l;
                        }
                        left.advance();
                    }
                }
            },
            SpanSource::Not {
                include,
                exclude,
                pre,
                post,
                reach,
                spans,
                doc,
            } => loop {
                let current = include.doc();
                if current == TERMINATED {
                    *doc = TERMINATED;
                    spans.clear();
                    return TERMINATED;
                }
                reach.clear();
                if exclude.seek(current) == current {
                    let mut end = 0;
                    reach.extend(exclude.spans().iter().map(|exclusion| {
                        end = end.max(exclusion.end);
                        (exclusion.start, end)
                    }));
                }
                spans.clear();
                spans.extend(include.spans().iter().copied().filter(|span| {
                    // Of the exclusions starting at most `post` tokens after `span`, none may end
                    // at most `pre` tokens before it.
                    let starting = reach.partition_point(|&(start, _)| {
                        start as i64 <= span.end as i64 + *post as i64
                    });
                    starting == 0
                        || (reach[starting - 1].1 as i64) < span.start as i64 - *pre as i64
                }));
                if !spans.is_empty() {
                    *doc = current;
                    return current;
                }
                include.advance();
            },
            _ => self.doc(),
        }
    }
}

impl DocSet for SpanSource {
    fn advance(&mut self) -> DocId {
        match self {
            SpanSource::Terms { postings, .. } => postings.advance(),
            SpanSource::Union { sources, .. } => {
                let doc = sources
                    .iter()
                    .map(SpanSource::doc)
                    .min()
                    .unwrap_or(TERMINATED);
                for source in sources.iter_mut().filter(|source| source.doc() == doc) {
                    source.advance();
                }
                self.doc()
            }
            SpanSource::Near { left, .. } => {
                left.advance();
                self.align()
            }
            SpanSource::Not { include, .. } => {
                include.advance();
                self.align()
            }
        }
    }

    fn seek(&mut self, target: DocId) -> DocId {
        if self.doc() >= target {
            return self.doc();
        }
        match self {
            SpanSource::Terms { postings, .. } => postings.seek(target),
            SpanSource::Union { sources, .. } => {
                for source in sources.iter_mut() {
                    source.seek(target);
                }
                self.doc()
            }
            SpanSource::Near { left, .. } => {
                left.seek(target);
                self.align()
            }
            SpanSource::Not { include, .. } => {
                include.seek(target);
                self.align()
            }
        }
    }

    fn doc(&self) -> DocId {
        match self {
            SpanSource::Terms { postings, .. } => postings.doc(),
            SpanSource::Union { sources, .. } => sources
                .iter()
                .map(SpanSource::doc)
                .min()
                .unwrap_or(TERMINATED),
            SpanSource::Near { doc, .. } | SpanSource::Not { doc, .. } => *doc,
        }
    }

    fn size_hint(&self) -> u32 {
        match self {
            SpanSource::Terms { postings, .. } => postings.size_hint(),
            SpanSource::Union { sources, .. } => {
                sources.iter().map(SpanSource::size_hint).max().unwrap_or(0)
            }
            SpanSource::Near { left, right, .. } => left.size_hint().min(right.size_hint()),
            SpanSource::Not { include, .. } => include.size_hint(),
        }
    }
}

impl SpanSource {
    /// The fieldnorm of the current document, when the postings carry it.
    pub fn fieldnorm_id(&self) -> Option<u8> {
        match self {
            SpanSource::Terms { postings, .. } => postings.fieldnorm_id(),
            SpanSource::Union { sources, .. } => {
                let doc = self.doc();
                sources
                    .iter()
                    .find(|source| source.doc() == doc)
                    .and_then(SpanSource::fieldnorm_id)
            }
            SpanSource::Near { left, .. } => left.fieldnorm_id(),
            SpanSource::Not { include, .. } => include.fieldnorm_id(),
        }
    }
}

/// Pairs `left` and `right` spans under the rule `ProxIter` applies to positions, keeping every
/// pairing so that an exclusion rejecting one cannot hide another. Like Lucene's minimum-slop
/// matches, a span that contains another match is dropped.
fn near_spans(left: &[Span], right: &[Span], distance: ProximityDistance, out: &mut Vec<Span>) {
    out.clear();
    let max_gap = distance.distance() as i64;
    pair_followers(left, right, max_gap, out);
    if !distance.in_order() {
        pair_followers(right, left, max_gap, out);
    }
    retain_minimal(out);
}

/// Pairs each span of `first` with the span of `then` that follows it and ends first, since every
/// other such pairing contains that one. `b` follows `a` when it starts at or after the last token
/// of `a`, at most `max_gap` tokens after its end; their pairing spans `[a.start, b.end)`.
fn pair_followers(first: &[Span], then: &[Span], max_gap: i64, out: &mut Vec<Span>) {
    // The earliest end from each span of `then` onwards: past it, no span can end first.
    let mut earliest_end = then
        .iter()
        .rev()
        .scan(u32::MAX, |earliest, b| {
            *earliest = (*earliest).min(b.end);
            Some(*earliest)
        })
        .collect::<Vec<_>>();
    earliest_end.reverse();
    for &a in first {
        let from = then.partition_point(|b| b.start < a.start.max(a.end - 1));
        let mut end: Option<u32> = None;
        for (&b, &earliest) in then[from..].iter().zip(&earliest_end[from..]) {
            if a.gap_before(b) > max_gap || end.is_some_and(|end| earliest >= end) {
                break;
            }
            if end.is_none_or(|end| b.end < end) {
                end = Some(b.end);
            }
        }
        if let Some(end) = end {
            out.push(Span {
                start: a.start,
                end,
            });
        }
    }
}

/// Sorts `spans` and drops the ones that contain another.
fn retain_minimal(spans: &mut Vec<Span>) {
    // Sorted by start, longest first, each span pops the ones that contain it.
    spans.sort_unstable_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut len = 0;
    for i in 0..spans.len() {
        let span = spans[i];
        while len > 0 && spans[len - 1].end >= span.end {
            len -= 1;
        }
        spans[len] = span;
        len += 1;
    }
    spans.truncate(len);
}

/// Appends the postings of a `Term` or `Regex` clause, expanding a regex like `##` does.
fn read_leaf(
    field: Field,
    reader: &SegmentReader,
    clause: &ProximityClause,
    postings: &mut Vec<Box<dyn Postings>>,
) -> tantivy::Result<()> {
    let inverted_index = reader.inverted_index(field)?;
    for term in clause.terms(field, Some(reader), WhichTerms::All)? {
        let term = Term::from_field_text(field, term.as_str());
        if let Some(segment_postings) =
            inverted_index.read_postings(&term, IndexRecordOption::WithFreqsAndPositions)?
        {
            postings.push(Box::new(segment_postings));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(ranges: &[(u32, u32)]) -> Vec<Span> {
        let mut spans: Vec<Span> = ranges
            .iter()
            .map(|&(start, end)| Span { start, end })
            .collect();
        spans.sort_unstable();
        spans
    }

    /// Every pairing, then the minimal ones.
    fn all_pairs(left: &[Span], right: &[Span], distance: ProximityDistance) -> Vec<Span> {
        let max_gap = distance.distance() as i64;
        let follows = |a: Span, b: Span| {
            b.start >= a.start && b.start + 1 >= a.end && a.gap_before(b) <= max_gap
        };
        let mut out = Vec::new();
        for &l in left {
            for &r in right {
                if follows(l, r) || (!distance.in_order() && follows(r, l)) {
                    out.push(Span {
                        start: l.start.min(r.start),
                        end: l.end.max(r.end),
                    });
                }
            }
        }
        retain_minimal(&mut out);
        out
    }

    fn check(left: &[Span], right: &[Span]) {
        for max_gap in 0..6 {
            for distance in [
                ProximityDistance::InOrder(max_gap),
                ProximityDistance::AnyOrder(max_gap),
            ] {
                let mut out = Vec::new();
                near_spans(left, right, distance, &mut out);
                assert_eq!(
                    out,
                    all_pairs(left, right, distance),
                    "{left:?} {right:?} {distance:?}"
                );
            }
        }
    }

    #[test]
    fn near_spans_keeps_the_minimal_pairings() {
        // `a b` repeated
        let a = spans(&[(0, 1), (2, 3), (4, 5), (6, 7)]);
        let b = spans(&[(1, 2), (3, 4), (5, 6), (7, 8)]);
        check(&a, &b);
        check(&b, &a);
        // same position, overlapping and nested spans, as a union can hold
        let nested = spans(&[(0, 1), (0, 3), (2, 3), (2, 6), (4, 5), (9, 10)]);
        let terms = spans(&[(0, 1), (1, 2), (3, 4), (5, 6), (8, 9), (12, 13)]);
        check(&nested, &terms);
        check(&terms, &nested);
        check(&nested, &nested);
        check(&a, &[]);
        check(&[], &a);
    }

    #[test]
    fn near_spans_bounds_the_pairings() {
        let n = 6000;
        let a = (0..n).map(|i| (2 * i, 2 * i + 1)).collect::<Vec<_>>();
        let b = (0..n).map(|i| (2 * i + 1, 2 * i + 2)).collect::<Vec<_>>();
        let mut out = Vec::new();
        near_spans(
            &spans(&a),
            &spans(&b),
            ProximityDistance::AnyOrder(2 * n),
            &mut out,
        );
        assert_eq!(out.len(), 2 * n as usize - 1);
    }

    /// `n` times `a`, then `n` times `c`, then `b`: the `c`s, and the `a`s with one long `a … b`.
    fn terms_and_a_long_span(n: u32) -> (Vec<Span>, Vec<Span>) {
        let c = (n..2 * n).map(|i| (i, i + 1)).collect::<Vec<_>>();
        let mut a = (0..n).map(|i| (i, i + 1)).collect::<Vec<_>>();
        a.push((n - 1, 2 * n + 1));
        (spans(&c), spans(&a))
    }

    #[test]
    fn near_spans_bounds_the_search_by_the_real_ends() {
        let (c, a) = terms_and_a_long_span(20);
        check(&c, &a);
        let n = 36000;
        let (c, a) = terms_and_a_long_span(n);
        let mut out = Vec::new();
        near_spans(&c, &a, ProximityDistance::AnyOrder(0), &mut out);
        assert_eq!(out, spans(&[(n - 1, n + 1)]));
    }
}
