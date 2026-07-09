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

//! Inline, per-tuple evaluation of a [`SearchQueryInput`] against a single value.
//!
//! When a search operator (`@@@`, `&&&`, `|||`, `###`, `===`) cannot be pushed down to the BM25
//! index and is instead evaluated as an ordinary per-row qual, we evaluate the query directly
//! against the operator's left-hand-side value rather than searching the index and materializing
//! every matching key into memory.
//!
//! This is only possible when every predicate in the query targets the *same* field that is on the
//! left-hand side of the operator (the [`BoundField`]). The caller establishes that invariant (by
//! inspecting the operator's LHS `Var`) and raises an error when [`compile`] returns [`Unsupported`].
//!
//! For a **tokenized text** bound field, term/match predicates are compared against the tokens the
//! field's analyzer produces from the row value, matching index-time analysis. For a **raw** field
//! (numeric, keyword, uuid, bool, date, ...) values are compared exactly / by range.

use crate::api::{FieldName, HashSet};
use crate::postgres::types::TantivyValue;
use crate::query::pdb_query::pdb;
use crate::query::SearchQueryInput;
use std::ops::Bound;
use tantivy::tokenizer::TextAnalyzer;

/// A query (or sub-query) that cannot be evaluated inline against a single value.
///
/// The contained string is a human-readable reason, surfaced to the user in the error the caller
/// raises. It should name the offending construct (query type or field).
#[derive(Debug, Clone)]
pub struct Unsupported(pub String);

impl Unsupported {
    fn new(reason: impl Into<String>) -> Self {
        Unsupported(reason.into())
    }
}

/// How the bound field's row value is interpreted.
#[derive(Clone)]
pub enum FieldKind {
    /// Exact-match field (numeric, keyword, uuid, bool, date, ...): compared by equality/range.
    /// `integer` is true for integer-typed fields, which additionally support a bare-number or
    /// comparison query string (`col @@@ '7'`, `col @@@ '>50'`).
    Raw { integer: bool },
    /// Tokenized text field: the row value is analyzed into tokens before matching.
    Tokenized(TextAnalyzer),
}

/// The field whose value the operator receives on its left-hand side.
#[derive(Clone)]
pub struct BoundField {
    pub name: FieldName,
    pub kind: FieldKind,
}

/// The bound field's value for the current row, prepared for evaluation.
pub enum ElementValue {
    /// A raw scalar value (for [`FieldKind::Raw`]).
    Raw(TantivyValue),
    /// The set of tokens the analyzer produced from the row's text (for [`FieldKind::Tokenized`]).
    Text(HashSet<String>),
}

/// Analyze `text` into its ordered token strings using `analyzer` (index-time analysis).
pub fn analyze(analyzer: &TextAnalyzer, text: &str) -> Vec<String> {
    // `token_stream` requires `&mut self`; the analyzer is cheap to clone and this keeps callers
    // able to hold it behind a shared reference.
    let mut analyzer = analyzer.clone();
    let mut stream = analyzer.token_stream(text);
    let mut tokens = Vec::new();
    while stream.advance() {
        tokens.push(stream.token().text.clone());
    }
    tokens
}

/// A compiled predicate over a single row's bound-field value. Produced by [`compile`] and
/// evaluated once per row via [`Self::eval`]. Holds no index handles and allocates nothing per row.
#[derive(Debug, Clone)]
pub enum InlinePredicate {
    /// Matches every value.
    True,
    /// Matches nothing.
    False,
    /// The field is present. STRICT-ness guarantees a null never reaches us, so this is `True`.
    Exists,

    // --- raw-field leaves ---
    /// Exact equality against a single term value.
    Eq(TantivyValue),
    /// Membership in a set of term values.
    InSet(Vec<TantivyValue>),
    /// A (possibly half-open) range.
    Range {
        lower: Bound<TantivyValue>,
        upper: Bound<TantivyValue>,
    },

    // --- tokenized-field leaves (query terms are pre-analyzed at compile time) ---
    /// Every query token must be present among the row's tokens (conjunction).
    AllTokens(Vec<String>),
    /// At least one query token must be present (disjunction).
    AnyTokens(Vec<String>),

    // --- combinators ---
    And(Vec<InlinePredicate>),
    Or(Vec<InlinePredicate>),
    Not(Box<InlinePredicate>),
    MinShould {
        should: Vec<InlinePredicate>,
        min: usize,
    },
}

impl InlinePredicate {
    pub fn eval(&self, value: &ElementValue) -> bool {
        match self {
            InlinePredicate::True | InlinePredicate::Exists => true,
            InlinePredicate::False => false,
            InlinePredicate::Eq(expected) => {
                matches!(value, ElementValue::Raw(v) if tv_eq(v, expected))
            }
            InlinePredicate::InSet(set) => {
                matches!(value, ElementValue::Raw(v) if set.iter().any(|e| tv_eq(v, e)))
            }
            InlinePredicate::Range { lower, upper } => {
                matches!(value, ElementValue::Raw(v) if range_contains(lower, upper, v))
            }
            InlinePredicate::AllTokens(terms) => {
                matches!(value, ElementValue::Text(tokens) if terms.iter().all(|t| tokens.contains(t)))
            }
            InlinePredicate::AnyTokens(terms) => {
                matches!(value, ElementValue::Text(tokens) if terms.iter().any(|t| tokens.contains(t)))
            }
            InlinePredicate::And(clauses) => clauses.iter().all(|c| c.eval(value)),
            InlinePredicate::Or(clauses) => clauses.iter().any(|c| c.eval(value)),
            InlinePredicate::Not(inner) => !inner.eval(value),
            InlinePredicate::MinShould { should, min } => {
                should.iter().filter(|c| c.eval(value)).count() >= *min
            }
        }
    }
}

fn range_contains(
    lower: &Bound<TantivyValue>,
    upper: &Bound<TantivyValue>,
    value: &TantivyValue,
) -> bool {
    use std::cmp::Ordering;
    let lower_ok = match lower {
        Bound::Unbounded => true,
        Bound::Included(b) => matches!(tv_cmp(value, b), Some(Ordering::Greater | Ordering::Equal)),
        Bound::Excluded(b) => matches!(tv_cmp(value, b), Some(Ordering::Greater)),
    };
    lower_ok
        && match upper {
            Bound::Unbounded => true,
            Bound::Included(b) => {
                matches!(tv_cmp(value, b), Some(Ordering::Less | Ordering::Equal))
            }
            Bound::Excluded(b) => matches!(tv_cmp(value, b), Some(Ordering::Less)),
        }
}

/// A numeric view of a value, so comparisons work across the `I64`/`U64`/`F64` variants that
/// different code paths produce (e.g. a JSON-decoded query term is `U64` while an `int4` column
/// value is `I64`).
enum Num {
    Int(i128),
    Float(f64),
}

fn as_num(v: &TantivyValue) -> Option<Num> {
    use crate::postgres::pdb_owned_value::PdbOwnedValue::*;
    match &v.0 {
        I64(n) => Some(Num::Int(*n as i128)),
        U64(n) => Some(Num::Int(*n as i128)),
        F64(f) => Some(Num::Float(*f)),
        _ => None,
    }
}

/// Equality that is tolerant of numeric-variant differences; falls back to exact equality for
/// non-numeric values.
fn tv_eq(a: &TantivyValue, b: &TantivyValue) -> bool {
    match (as_num(a), as_num(b)) {
        (Some(_), Some(_)) => tv_cmp(a, b) == Some(std::cmp::Ordering::Equal),
        _ => a == b,
    }
}

/// Ordering that is tolerant of numeric-variant differences; falls back to `TantivyValue`'s own
/// (same-variant) ordering for non-numeric values.
fn tv_cmp(a: &TantivyValue, b: &TantivyValue) -> Option<std::cmp::Ordering> {
    match (as_num(a), as_num(b)) {
        (Some(Num::Int(x)), Some(Num::Int(y))) => Some(x.cmp(&y)),
        (Some(x), Some(y)) => {
            let xf = match x {
                Num::Int(i) => i as f64,
                Num::Float(f) => f,
            };
            let yf = match y {
                Num::Int(i) => i as f64,
                Num::Float(f) => f,
            };
            xf.partial_cmp(&yf)
        }
        _ => a.partial_cmp(b),
    }
}

/// Compile `query` into an [`InlinePredicate`] over the value of `bound`.
///
/// Returns [`Unsupported`] when the query references any field other than `bound`, or uses a
/// construct that cannot (yet) be evaluated inline. `bound` is `None` when the LHS field could not
/// be resolved, in which case only field-less `All`/`Empty` queries are supported.
pub fn compile(
    query: &SearchQueryInput,
    bound: Option<&BoundField>,
) -> Result<InlinePredicate, Unsupported> {
    match query {
        SearchQueryInput::All => Ok(InlinePredicate::True),
        SearchQueryInput::Empty | SearchQueryInput::Uninitialized => Ok(InlinePredicate::False),

        SearchQueryInput::WithIndex { query, .. }
        | SearchQueryInput::Boost { query, .. }
        | SearchQueryInput::ConstScore { query, .. } => compile(query, bound),

        SearchQueryInput::Boolean {
            must,
            should,
            must_not,
            minimum_should_match,
        } => compile_boolean(must, should, must_not, *minimum_should_match, bound),

        SearchQueryInput::DisjunctionMax { disjuncts, .. } => Ok(InlinePredicate::Or(
            disjuncts
                .iter()
                .map(|d| compile(d, bound))
                .collect::<Result<Vec<_>, _>>()?,
        )),

        SearchQueryInput::FieldedQuery { field, query } => {
            let bound = match bound {
                Some(b) if b.name.root() == field.root() => b,
                _ => {
                    return Err(Unsupported::new(format!(
                        "query targets field '{field}', which is not the value on the left-hand \
                         side of the operator"
                    )))
                }
            };
            compile_fielded(query, bound)
        }

        SearchQueryInput::ScoreFilter { .. } => {
            Err(Unsupported::new("score filters cannot be evaluated inline"))
        }
        SearchQueryInput::MoreLikeThis { .. } => Err(Unsupported::new(
            "more-like-this cannot be evaluated inline",
        )),
        SearchQueryInput::Parse { .. } => Err(Unsupported::new(
            "multi-field parse queries cannot be evaluated inline",
        )),
        SearchQueryInput::TermSet { .. } => Err(Unsupported::new(
            "multi-field term sets cannot be evaluated inline",
        )),
        SearchQueryInput::PostgresExpression { .. } => Err(Unsupported::new(
            "postgres expression queries cannot be evaluated inline",
        )),
        SearchQueryInput::HeapFilter { .. } => Err(Unsupported::new(
            "heap filter queries cannot be evaluated inline",
        )),
    }
}

fn compile_boolean(
    must: &[SearchQueryInput],
    should: &[SearchQueryInput],
    must_not: &[SearchQueryInput],
    minimum_should_match: Option<i64>,
    bound: Option<&BoundField>,
) -> Result<InlinePredicate, Unsupported> {
    let mut clauses = Vec::new();

    for m in must {
        clauses.push(compile(m, bound)?);
    }
    for mn in must_not {
        clauses.push(InlinePredicate::Not(Box::new(compile(mn, bound)?)));
    }

    if !should.is_empty() {
        let compiled_should = should
            .iter()
            .map(|s| compile(s, bound))
            .collect::<Result<Vec<_>, _>>()?;

        // Tantivy semantics: `should` clauses are optional when `must`/`must_not` are present
        // (unless `minimum_should_match` forces some); otherwise at least one must match.
        let has_required = !must.is_empty() || !must_not.is_empty();
        let min = match minimum_should_match {
            Some(m) if m > 0 => (m as usize).min(compiled_should.len()),
            Some(_) => 0,
            None if has_required => 0,
            None => 1,
        };

        if min > 0 {
            clauses.push(InlinePredicate::MinShould {
                should: compiled_should,
                min,
            });
        } else if !has_required {
            clauses.push(InlinePredicate::Or(compiled_should));
        }
    }

    Ok(InlinePredicate::And(clauses))
}

fn compile_fielded(query: &pdb::Query, bound: &BoundField) -> Result<InlinePredicate, Unsupported> {
    match query {
        pdb::Query::All => Ok(InlinePredicate::True),
        pdb::Query::Empty => Ok(InlinePredicate::False),
        pdb::Query::Exists => Ok(InlinePredicate::Exists),

        pdb::Query::ScoreAdjusted { query, .. } => compile_fielded(query, bound),

        pdb::Query::Term { value } => match &bound.kind {
            FieldKind::Raw { .. } => Ok(InlinePredicate::Eq(TantivyValue(value.clone()))),
            FieldKind::Tokenized(analyzer) => {
                let text = value_as_str(value).ok_or_else(|| {
                    Unsupported::new("non-text term against a tokenized field")
                })?;
                Ok(InlinePredicate::AllTokens(analyze(analyzer, text)))
            }
        },

        pdb::Query::TermSet { terms } => match &bound.kind {
            FieldKind::Raw { .. } => Ok(InlinePredicate::InSet(
                terms.iter().map(|v| TantivyValue(v.clone())).collect(),
            )),
            FieldKind::Tokenized(_) => Err(Unsupported::new(
                "term-set against a tokenized field is not supported inline",
            )),
        },

        pdb::Query::RangeTerm { value } => match &bound.kind {
            FieldKind::Raw { .. } => Ok(InlinePredicate::Eq(TantivyValue(value.clone()))),
            FieldKind::Tokenized(_) => {
                Err(Unsupported::new("range term against a tokenized field"))
            }
        },

        pdb::Query::Range {
            lower_bound,
            upper_bound,
        } => match &bound.kind {
            FieldKind::Raw { .. } => Ok(InlinePredicate::Range {
                lower: map_bound(lower_bound),
                upper: map_bound(upper_bound),
            }),
            FieldKind::Tokenized(_) => Err(Unsupported::new("range against a tokenized field")),
        },

        pdb::Query::Match {
            value,
            tokenizer,
            distance,
            prefix,
            conjunction_mode,
            ..
        } => {
            if tokenizer.is_some() || distance.is_some() || prefix == &Some(true) {
                return Err(Unsupported::new(
                    "match with a custom tokenizer, fuzzy distance, or prefix is not supported inline",
                ));
            }
            let analyzer = tokenized(bound, "match")?;
            let terms = analyze(analyzer, value);
            Ok(token_mode(terms, *conjunction_mode))
        }

        pdb::Query::MatchArray {
            tokens,
            distance,
            prefix,
            conjunction_mode,
            ..
        } => {
            if distance.is_some() || prefix == &Some(true) {
                return Err(Unsupported::new(
                    "match-array with fuzzy distance or prefix is not supported inline",
                ));
            }
            let analyzer = tokenized(bound, "match-array")?;
            // Re-analyze the provided tokens so they match index-time normalization.
            let terms: Vec<String> = tokens.iter().flat_map(|t| analyze(analyzer, t)).collect();
            Ok(token_mode(terms, *conjunction_mode))
        }

        pdb::Query::Parse {
            query_string,
            conjunction_mode,
            ..
        }
        | pdb::Query::ParseWithField {
            query_string,
            conjunction_mode,
            ..
        } => match &bound.kind {
            FieldKind::Tokenized(analyzer) => {
                if !is_simple_query_string(query_string) {
                    return Err(Unsupported::new(
                        "parse query with query-parser syntax cannot be evaluated inline",
                    ));
                }
                let terms = analyze(analyzer, query_string);
                Ok(token_mode(terms, *conjunction_mode))
            }
            // On an integer field a query string is a bare number (`'7'`) or a comparison
            // (`'>50'`), matching how the parser lowers it against the numeric column.
            FieldKind::Raw { integer: true } => parse_integer_predicate(query_string).ok_or_else(|| {
                Unsupported::new("only bare-number or comparison query strings are supported on integer fields inline")
            }),
            FieldKind::Raw { integer: false } => Err(Unsupported::new(format!(
                "parse query against a non-tokenized field '{}'",
                bound.name
            ))),
        },

        pdb::Query::FuzzyTerm { .. } => {
            Err(Unsupported::new("fuzzy queries cannot be evaluated inline"))
        }
        pdb::Query::Phrase { .. }
        | pdb::Query::PhraseArray { .. }
        | pdb::Query::PhrasePrefix { .. }
        | pdb::Query::TokenizedPhrase { .. } => {
            Err(Unsupported::new("phrase queries cannot be evaluated inline"))
        }
        pdb::Query::Regex { .. } | pdb::Query::RegexPhrase { .. } => {
            Err(Unsupported::new("regex queries cannot be evaluated inline"))
        }
        pdb::Query::Proximity { .. } => {
            Err(Unsupported::new("proximity queries cannot be evaluated inline"))
        }
        pdb::Query::FastFieldRangeWeight { .. }
        | pdb::Query::RangeContains { .. }
        | pdb::Query::RangeIntersects { .. }
        | pdb::Query::RangeWithin { .. } => {
            Err(Unsupported::new("range-field predicates cannot be evaluated inline"))
        }
        pdb::Query::UnclassifiedString { .. } | pdb::Query::UnclassifiedArray { .. } => {
            Err(Unsupported::new(
                "unclassified query (operator rewrite did not run) cannot be evaluated inline",
            ))
        }
    }
}

fn tokenized<'a>(bound: &'a BoundField, what: &str) -> Result<&'a TextAnalyzer, Unsupported> {
    match &bound.kind {
        FieldKind::Tokenized(analyzer) => Ok(analyzer),
        FieldKind::Raw { .. } => Err(Unsupported::new(format!(
            "{what} query against a non-tokenized field '{}'",
            bound.name
        ))),
    }
}

fn token_mode(terms: Vec<String>, conjunction_mode: Option<bool>) -> InlinePredicate {
    if conjunction_mode == Some(true) {
        InlinePredicate::AllTokens(terms)
    } else {
        InlinePredicate::AnyTokens(terms)
    }
}

/// Parse an integer-field query string into an equality or comparison predicate.
///
/// Handles a bare integer (`"7"` -> `= 7`) and the single-sided comparisons the query parser
/// produces for numeric columns (`">50"`, `">=50"`, `"<50"`, `"<=50"`). Returns `None` for
/// anything else (ranges, boolean syntax, non-integers), which the caller treats as unsupported.
fn parse_integer_predicate(s: &str) -> Option<InlinePredicate> {
    let s = s.trim();
    let i64v = |n: i64| TantivyValue(crate::postgres::pdb_owned_value::PdbOwnedValue::I64(n));

    let (rest, ctor): (&str, fn(TantivyValue) -> InlinePredicate) =
        if let Some(r) = s.strip_prefix(">=") {
            (r, |v| InlinePredicate::Range {
                lower: Bound::Included(v),
                upper: Bound::Unbounded,
            })
        } else if let Some(r) = s.strip_prefix("<=") {
            (r, |v| InlinePredicate::Range {
                lower: Bound::Unbounded,
                upper: Bound::Included(v),
            })
        } else if let Some(r) = s.strip_prefix('>') {
            (r, |v| InlinePredicate::Range {
                lower: Bound::Excluded(v),
                upper: Bound::Unbounded,
            })
        } else if let Some(r) = s.strip_prefix('<') {
            (r, |v| InlinePredicate::Range {
                lower: Bound::Unbounded,
                upper: Bound::Excluded(v),
            })
        } else {
            (s, InlinePredicate::Eq)
        };

    let n: i64 = rest.trim().parse().ok()?;
    Some(ctor(i64v(n)))
}

/// A query string with no query-parser metacharacters -- safe to treat as plain analyzed text.
fn is_simple_query_string(s: &str) -> bool {
    const SPECIAL: &[char] = &[
        ':', '+', '-', '"', '*', '~', '^', '(', ')', '[', ']', '{', '}', '<', '>', '\\', '/', '!',
    ];
    if s.chars().any(|c| SPECIAL.contains(&c)) {
        return false;
    }
    // Boolean operators are whitespace-delimited, uppercase keywords in the tantivy parser.
    !s.split_whitespace()
        .any(|w| matches!(w, "AND" | "OR" | "NOT"))
}

fn value_as_str(value: &crate::postgres::pdb_owned_value::PdbOwnedValue) -> Option<&str> {
    match value {
        crate::postgres::pdb_owned_value::PdbOwnedValue::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

fn map_bound(
    bound: &Bound<crate::postgres::pdb_owned_value::PdbOwnedValue>,
) -> Bound<TantivyValue> {
    match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(v) => Bound::Included(TantivyValue(v.clone())),
        Bound::Excluded(v) => Bound::Excluded(TantivyValue(v.clone())),
    }
}

#[cfg(any(test, feature = "pg_test"))]
mod tests {
    use super::*;
    use crate::postgres::pdb_owned_value::PdbOwnedValue;

    fn raw_i64(n: i64) -> ElementValue {
        ElementValue::Raw(TantivyValue(PdbOwnedValue::I64(n)))
    }
    fn raw_field() -> BoundField {
        BoundField {
            name: FieldName::from("id"),
            kind: FieldKind::Raw { integer: false },
        }
    }

    #[test]
    fn all_and_empty() {
        assert!(compile(&SearchQueryInput::All, None)
            .unwrap()
            .eval(&raw_i64(1)));
        assert!(!compile(&SearchQueryInput::Empty, None)
            .unwrap()
            .eval(&raw_i64(1)));
    }

    #[test]
    fn term_equality_raw() {
        let f = raw_field();
        let q = SearchQueryInput::FieldedQuery {
            field: f.name.clone(),
            query: pdb::Query::Term {
                value: PdbOwnedValue::I64(5),
            },
        };
        let p = compile(&q, Some(&f)).unwrap();
        assert!(p.eval(&raw_i64(5)));
        assert!(!p.eval(&raw_i64(6)));
    }

    #[test]
    fn wrong_field_unsupported() {
        let f = raw_field();
        let q = SearchQueryInput::FieldedQuery {
            field: FieldName::from("body"),
            query: pdb::Query::Exists,
        };
        assert!(compile(&q, Some(&f)).is_err());
    }

    #[test]
    fn half_open_range() {
        let f = raw_field();
        let q = SearchQueryInput::FieldedQuery {
            field: f.name.clone(),
            query: pdb::Query::Range {
                lower_bound: Bound::Included(PdbOwnedValue::I64(10)),
                upper_bound: Bound::Excluded(PdbOwnedValue::I64(20)),
            },
        };
        let p = compile(&q, Some(&f)).unwrap();
        assert!(!p.eval(&raw_i64(9)));
        assert!(p.eval(&raw_i64(10)));
        assert!(p.eval(&raw_i64(19)));
        assert!(!p.eval(&raw_i64(20)));
    }

    #[test]
    fn simple_query_string_detection() {
        assert!(is_simple_query_string("keyboard"));
        assert!(is_simple_query_string("fast keyboard"));
        assert!(!is_simple_query_string("field:value"));
        assert!(!is_simple_query_string("foo AND bar"));
        assert!(!is_simple_query_string("wild*"));
        assert!(!is_simple_query_string(">50"));
    }

    #[test]
    fn token_predicates() {
        let toks: HashSet<String> = ["fast", "brown", "fox"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let e = ElementValue::Text(toks);
        assert!(InlinePredicate::AnyTokens(vec!["fox".into(), "cat".into()]).eval(&e));
        assert!(!InlinePredicate::AnyTokens(vec!["cat".into()]).eval(&e));
        assert!(InlinePredicate::AllTokens(vec!["fast".into(), "fox".into()]).eval(&e));
        assert!(!InlinePredicate::AllTokens(vec!["fast".into(), "cat".into()]).eval(&e));
    }
}
