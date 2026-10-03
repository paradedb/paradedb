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

use crate::api::FieldName;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::{ConstNode, TantivyValueError};
use crate::postgres::utils::ToPalloc;
use crate::query::pdb_query::pdb;
use pgrx::{PgList, pg_sys};
use std::ffi::CStr;
use std::ops::Bound;
use std::ptr::null_mut;
use tantivy::SegmentReader;
use tantivy::query::{
    BooleanQuery, BoostQuery, ConstScoreQuery, DisjunctionMaxQuery, Occur, Query, QueryEstimate,
    TermQuery, TermSetQuery,
};

/// Build the equivalent SQL predicate for PostgreSQL's selectivity estimator.
pub(super) unsafe fn non_text_clause(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    index: &PgSearchRelation,
    field: &FieldName,
    query: &pdb::Query,
) -> Option<*mut pg_sys::Node> {
    if root.is_null() {
        return None;
    }
    if field.path().is_some() {
        todo!("estimate JSON paths using expression statistics");
    }
    let schema = index.schema().ok()?;
    let fields = schema.categorized_fields();
    let (search_field, data) = fields.iter().find(|(f, _)| f.field_name() == field)?;
    if data.is_array {
        todo!("estimate array elements using array statistics");
    }
    if data.is_json {
        todo!("estimate JSON filters using expression statistics");
    }
    // A range over text terms is not a range over whole column values.
    if matches!(query, pdb::Query::Range { .. })
        && matches!(
            search_field.field_type(),
            crate::schema::SearchFieldType::Text(..)
                | crate::schema::SearchFieldType::Tokenized(..)
        )
    {
        return None;
    }
    let attno = data
        .source
        .heap_attno(index)
        .unwrap_or_else(|| todo!("estimate computed fields using expression statistics"));
    let heap = index.heap_relation()?;
    let rte = PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable)
        .get_ptr(rti.checked_sub(1)? as usize)?;
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION || (*rte).relid != heap.oid() {
        return None;
    }
    let desc = heap.tuple_desc();
    let attr = desc.get(attno)?;
    let oid = attr.atttypid;
    let var = pg_sys::makeVar(
        rti as _,
        (attno + 1) as _,
        oid,
        attr.atttypmod,
        attr.attcollation,
        0,
    );
    let mut clauses = PgList::<pg_sys::Node>::new();
    let exists = || {
        pg_sys::NullTest {
            xpr: pg_sys::Expr {
                type_: pg_sys::NodeTag::T_NullTest,
            },
            arg: var.cast(),
            nulltesttype: pg_sys::NullTestType::IS_NOT_NULL,
            argisrow: false,
            location: -1,
        }
        .palloc()
        .cast()
    };
    match query {
        pdb::Query::Exists => clauses.push(exists()),
        pdb::Query::Term { value } => {
            let value: ConstNode = (value, oid).try_into().ok()?;
            clauses.push(Comparison::try_from((&*var, c"=", value)).ok()?.into());
        }
        pdb::Query::Range {
            lower_bound,
            upper_bound,
        } => {
            for (bound, inclusive, exclusive) in
                [(lower_bound, c">=", c">"), (upper_bound, c"<=", c"<")]
            {
                let (value, name) = match bound {
                    Bound::Included(value) => (value, inclusive),
                    Bound::Excluded(value) => (value, exclusive),
                    Bound::Unbounded => continue,
                };
                let value: ConstNode = (value, oid).try_into().ok()?;
                clauses.push(Comparison::try_from((&*var, name, value)).ok()?.into());
            }
            if clauses.is_empty() {
                clauses.push(exists());
            }
        }
        pdb::Query::RangeTerm { value } => {
            let subtype = pg_sys::get_range_subtype(oid);
            if subtype == pg_sys::InvalidOid {
                return None;
            }
            clauses.push(
                Comparison {
                    var: &*var,
                    op: pg_sys::Oid::from(pg_sys::OID_RANGE_CONTAINS_ELEM_OP),
                    value: (value, subtype).try_into().ok()?,
                }
                .into(),
            );
        }
        pdb::Query::RangeContains {
            lower_bound,
            upper_bound,
        }
        | pdb::Query::RangeIntersects {
            lower_bound,
            upper_bound,
        }
        | pdb::Query::RangeWithin {
            lower_bound,
            upper_bound,
        } => {
            let bounds = RangeBounds {
                lower: lower_bound,
                upper: upper_bound,
            };
            let value = match oid {
                pg_sys::INT4RANGEOID => pgrx::Range::<i32>::try_from(bounds).ok()?.into(),
                pg_sys::INT8RANGEOID => pgrx::Range::<i64>::try_from(bounds).ok()?.into(),
                pg_sys::NUMRANGEOID => pgrx::Range::<pgrx::AnyNumeric>::try_from(bounds)
                    .ok()?
                    .into(),
                pg_sys::DATERANGEOID => pgrx::Range::<pgrx::datum::Date>::try_from(bounds)
                    .ok()?
                    .into(),
                pg_sys::TSRANGEOID => pgrx::Range::<pgrx::datum::Timestamp>::try_from(bounds)
                    .ok()?
                    .into(),
                pg_sys::TSTZRANGEOID => {
                    pgrx::Range::<pgrx::datum::TimestampWithTimeZone>::try_from(bounds)
                        .ok()?
                        .into()
                }
                _ => unreachable!("range predicates require a supported range-column type"),
            };
            let op = match query {
                pdb::Query::RangeContains { .. } => pg_sys::OID_RANGE_CONTAINS_OP,
                pdb::Query::RangeIntersects { .. } => pg_sys::OID_RANGE_OVERLAP_OP,
                pdb::Query::RangeWithin { .. } => pg_sys::OID_RANGE_CONTAINED_OP,
                _ => unreachable!(),
            };
            clauses.push(
                Comparison {
                    var: &*var,
                    op: pg_sys::Oid::from(op),
                    value,
                }
                .into(),
            );
        }
        pdb::Query::All | pdb::Query::Empty | pdb::Query::ScoreAdjusted { .. } => {
            unreachable!("the caller handles constants and unwraps score adjustments")
        }
        pdb::Query::MoreLikeThis { .. }
        | pdb::Query::FuzzyTerm { .. }
        | pdb::Query::Match { .. }
        | pdb::Query::MatchArray { .. }
        | pdb::Query::Phrase { .. }
        | pdb::Query::PhraseArray { .. }
        | pdb::Query::PhrasePrefix { .. }
        | pdb::Query::TokenizedPhrase { .. }
        | pdb::Query::Regex { .. }
        | pdb::Query::RegexPhrase { .. } => unreachable!("text queries use Tantivy's statistics"),
        pdb::Query::Parse { .. } | pdb::Query::ParseWithField { .. } => {
            todo!("decompose parsed queries to estimate their non-text leaves")
        }
        pdb::Query::TermSet { .. } => unreachable!("the caller expands term sets"),
        pdb::Query::FastFieldRangeWeight { .. } => {
            unreachable!("the caller converts fast-field bounds to a range")
        }
        pdb::Query::Proximity { .. } => todo!("implement a text estimator for proximity"),
        pdb::Query::UnclassifiedString { .. } | pdb::Query::UnclassifiedArray { .. } => {
            unreachable!("operator support functions must classify queries before estimation")
        }
    }
    Some(pg_sys::make_ands_explicit(clauses.into_pg()).cast())
}

struct Comparison<'a> {
    var: &'a pg_sys::Var,
    op: pg_sys::Oid,
    value: ConstNode,
}

impl<'a> TryFrom<(&'a pg_sys::Var, &CStr, ConstNode)> for Comparison<'a> {
    type Error = ();

    fn try_from(
        (var, name, value): (&'a pg_sys::Var, &CStr, ConstNode),
    ) -> Result<Self, Self::Error> {
        unsafe {
            let mut names = PgList::<pg_sys::Node>::new();
            for name in [c"pg_catalog", name] {
                names.push(pg_sys::makeString(pg_sys::pstrdup(name.as_ptr())).cast());
            }
            let konst: *mut pg_sys::Const = (&value).into();
            let op =
                pg_sys::compatible_oper_opid(names.as_ptr(), var.vartype, (*konst).consttype, true);
            if op == pg_sys::InvalidOid {
                return Err(());
            }
            Ok(Self { var, op, value })
        }
    }
}

impl From<Comparison<'_>> for *mut pg_sys::Node {
    fn from(comparison: Comparison<'_>) -> Self {
        let value: *mut pg_sys::Const = comparison.value.into();
        unsafe {
            pg_sys::make_opclause(
                comparison.op,
                pg_sys::BOOLOID,
                false,
                std::ptr::from_ref(comparison.var).cast_mut().cast(),
                value.cast(),
                pg_sys::InvalidOid,
                comparison.var.varcollid,
            )
            .cast()
        }
    }
}

struct RangeBounds<'a> {
    lower: &'a Bound<PdbOwnedValue>,
    upper: &'a Bound<PdbOwnedValue>,
}

impl<T: pgrx::datum::RangeSubType> TryFrom<RangeBounds<'_>> for pgrx::Range<T> {
    type Error = TantivyValueError;

    fn try_from(bounds: RangeBounds<'_>) -> Result<Self, Self::Error> {
        let bound =
            |bound: &Bound<PdbOwnedValue>| -> Result<pgrx::datum::RangeBound<T>, Self::Error> {
                let (value, inclusive) = match bound {
                    Bound::Unbounded => return Ok(pgrx::datum::RangeBound::Infinite),
                    Bound::Included(value) => (value, true),
                    Bound::Excluded(value) => (value, false),
                };
                let value: ConstNode = (value, T::type_oid()).try_into()?;
                let value: *mut pg_sys::Const = value.into();
                let value = unsafe { T::from_datum((*value).constvalue, false) }
                    .ok_or(TantivyValueError::DatumDeref)?;
                Ok(if inclusive {
                    pgrx::datum::RangeBound::Inclusive(value)
                } else {
                    pgrx::datum::RangeBound::Exclusive(value)
                })
            };
        Ok(Self::new(bound(bounds.lower)?, bound(bounds.upper)?))
    }
}

pub(super) struct Selectivity(pub f64);

impl From<Selectivity> for *mut pg_sys::Node {
    fn from(selectivity: Selectivity) -> Self {
        // PostgreSQL reads norm_selec before inspecting the clause. These nodes are only estimated.
        pg_sys::RestrictInfo {
            type_: pg_sys::NodeTag::T_RestrictInfo,
            clause: unsafe { pg_sys::makeBoolConst(true, false).cast() },
            norm_selec: selectivity.0,
            outer_selec: selectivity.0,
            ..Default::default()
        }
        .palloc()
        .cast()
    }
}

impl Selectivity {
    pub(super) unsafe fn estimate(
        clause: *mut pg_sys::Node,
        planner: Option<(*mut pg_sys::PlannerInfo, pg_sys::Index)>,
    ) -> f64 {
        // Without a planner, the tree contains only cached selectivities and Boolean nodes.
        let (root, rti) = planner.unwrap_or((null_mut(), 0));
        pg_sys::clause_selectivity(
            root,
            clause,
            rti as _,
            pg_sys::JoinType::JOIN_INNER,
            null_mut(),
        )
        .clamp(0.0, 1.0)
    }
}

#[derive(Default)]
pub(super) struct BooleanClause {
    pub must: Vec<*mut pg_sys::Node>,
    pub should: Vec<*mut pg_sys::Node>,
    pub must_not: Vec<*mut pg_sys::Node>,
    pub minimum_should_match: usize,
}

impl BooleanClause {
    pub(super) unsafe fn into_clause(
        mut self,
        planner: Option<(*mut pg_sys::PlannerInfo, pg_sys::Index)>,
    ) -> *mut pg_sys::Node {
        if self.must.is_empty() && self.should.is_empty()
            || self.minimum_should_match > self.should.len()
        {
            return Selectivity(0.0).into();
        }
        // With no MUST clauses, Tantivy requires a SHOULD match even when the minimum is zero.
        let required = self
            .minimum_should_match
            .max(usize::from(self.must.is_empty()));
        if required == self.should.len() {
            self.must.append(&mut self.should);
        } else if required == 1 {
            let mut should = PgList::<pg_sys::Expr>::new();
            for clause in self.should {
                should.push(clause.cast());
            }
            self.must
                .push(pg_sys::make_orclause(should.into_pg()).cast());
        } else if required > 1 {
            // P(at least k matches), assuming independent children; never enumerate combinations.
            let mut at_least = vec![0.0; required + 1];
            at_least[0] = 1.0;
            for clause in self.should {
                let p = Selectivity::estimate(clause, planner);
                for k in (1..=required).rev() {
                    at_least[k] = p * at_least[k - 1] + (1.0 - p) * at_least[k];
                }
            }
            self.must.push(Selectivity(at_least[required]).into());
        }
        let mut clauses = PgList::<pg_sys::Node>::new();
        for clause in self.must {
            // Keep range bounds together so PostgreSQL can recognize a two-sided range.
            for conjunct in
                PgList::<pg_sys::Node>::from_pg(pg_sys::make_ands_implicit(clause.cast()))
                    .iter_ptr()
            {
                clauses.push(conjunct);
            }
        }
        for clause in self.must_not {
            clauses.push(pg_sys::make_notclause(clause.cast()).cast());
        }
        pg_sys::make_ands_explicit(clauses.into_pg()).cast()
    }
}

pub(super) fn query_estimate(mut query: &dyn Query) -> Box<dyn QueryEstimate + '_> {
    while let Some(boxed) = query.downcast_ref::<Box<dyn Query>>() {
        query = boxed.as_ref();
    }
    if let Some(query) = query.downcast_ref::<BooleanQuery>() {
        Box::new(BooleanQueryEstimate(query))
    } else if let Some(query) = query.downcast_ref::<DisjunctionMaxQuery>() {
        Box::new(DisjunctionMaxQueryEstimate(query))
    } else if let Some(query) = query.downcast_ref::<TermSetQuery>() {
        Box::new(TermSetQueryEstimate(query))
    } else if let Some(query) = query.downcast_ref::<BoostQuery>() {
        Box::new(BoostQueryEstimate(query))
    } else if let Some(query) = query.downcast_ref::<ConstScoreQuery>() {
        Box::new(ConstScoreQueryEstimate(query))
    } else {
        Box::new(TantivyQueryEstimate(query))
    }
}

struct BooleanQueryEstimate<'a>(&'a BooleanQuery);

impl QueryEstimate for BooleanQueryEstimate<'_> {
    /// Let PostgreSQL combine the child estimates, respecting which clauses must match.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        BooleanClause {
            minimum_should_match: self.0.get_minimum_number_should_match(),
            ..Default::default()
        }
        .estimate_docs(
            reader,
            self.0.clauses().iter().map(|(occur, query)| {
                (*occur, query_estimate(query.as_ref()).estimate_docs(reader))
            }),
        )
    }
}

struct DisjunctionMaxQueryEstimate<'a>(&'a DisjunctionMaxQuery);

impl QueryEstimate for DisjunctionMaxQueryEstimate<'_> {
    /// Estimate an OR of the children; the scoring rule does not change which documents match.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        BooleanClause::default().estimate_docs(
            reader,
            self.0.disjuncts().iter().map(|query| {
                (
                    Occur::Should,
                    query_estimate(query.as_ref()).estimate_docs(reader),
                )
            }),
        )
    }
}

struct TermSetQueryEstimate<'a>(&'a TermSetQuery);

impl QueryEstimate for TermSetQueryEstimate<'_> {
    /// Look up each term's document count and let PostgreSQL estimate their OR.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        BooleanClause::default().estimate_docs(
            reader,
            self.0.terms().map(|term| {
                let query = TermQuery::new(term.clone(), tantivy::schema::IndexRecordOption::Basic);
                (Occur::Should, query.estimate_docs(reader))
            }),
        )
    }
}

struct BoostQueryEstimate<'a>(&'a BoostQuery);

impl QueryEstimate for BoostQueryEstimate<'_> {
    /// Boost changes scores, so use the child query's estimate.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        query_estimate(self.0.query()).estimate_docs(reader)
    }
}

struct ConstScoreQueryEstimate<'a>(&'a ConstScoreQuery);

impl QueryEstimate for ConstScoreQueryEstimate<'_> {
    /// A constant score changes no matches, so use the child query's estimate.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        query_estimate(self.0.query()).estimate_docs(reader)
    }
}

struct TantivyQueryEstimate<'a>(&'a dyn Query);

impl QueryEstimate for TantivyQueryEstimate<'_> {
    /// Use Tantivy's estimate for queries it already handles.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        self.0.estimate_docs(reader)
    }
}

impl BooleanClause {
    fn estimate_docs(
        mut self,
        reader: &SegmentReader,
        children: impl Iterator<Item = (Occur, tantivy::Result<Option<(u32, u64)>>)>,
    ) -> tantivy::Result<Option<(u32, u64)>> {
        if reader.max_doc() == 0 {
            return Ok(Some((0, 0)));
        }
        let mut work = 0u64;
        for (occur, estimate) in children {
            let Some((count, cost)) = estimate? else {
                return Ok(None);
            };
            work = work.saturating_add(cost);
            let clause =
                Selectivity(f64::from(count.min(reader.max_doc())) / f64::from(reader.max_doc()))
                    .into();
            match occur {
                Occur::Must => self.must.push(clause),
                Occur::Should => self.should.push(clause),
                Occur::MustNot => self.must_not.push(clause),
            }
        }
        let selectivity = unsafe { Selectivity::estimate(self.into_clause(None), None) };
        Ok(Some((
            (selectivity * f64::from(reader.max_doc())).ceil() as u32,
            work,
        )))
    }
}
