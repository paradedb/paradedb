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
use crate::api::operator::row_expr_from_indexed_expr;
use crate::postgres::composite::get_composite_type_fields;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::{ConstNode, TantivyValueError};
use crate::postgres::utils::{FieldSource, ToPalloc, strip_tokenizer_cast};
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
    let schema = index.schema().ok()?;
    let fields = schema.categorized_fields();
    let (search_field, data) = fields
        .iter()
        .find(|(f, _)| f.field_name().as_ref() == field.root())?;
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
    let heap = index.heap_relation()?;
    let rte = PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable)
        .get_ptr(rti.checked_sub(1)? as usize)?;
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION || (*rte).relid != heap.oid() {
        return None;
    }
    let expr = FieldExpression::try_from((&data.source, index, rti))
        .ok()?
        .0;
    if data.is_json {
        let value = JsonPredicate { field, query }.try_into().ok()?;
        let expr = if pg_sys::exprType(expr.cast()) == pg_sys::JSONBOID {
            expr
        } else {
            let mut args = PgList::new();
            args.push(expr);
            pg_sys::makeFuncExpr(
                pg_sys::F_TO_JSONB.into(),
                pg_sys::JSONBOID,
                args.into_pg(),
                pg_sys::InvalidOid,
                pg_sys::InvalidOid,
                pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
            )
            .cast()
        };
        return Some(Comparison::try_from((&*expr, c"@?", value)).ok()?.into());
    }
    let expr_oid = pg_sys::exprType(expr.cast());
    let element_oid = pg_sys::get_base_element_type(expr_oid);
    let is_array = element_oid != pg_sys::InvalidOid;
    let oid = if is_array { element_oid } else { expr_oid };
    if is_array
        && matches!(query, pdb::Query::Range { lower_bound, upper_bound }
        if !matches!(lower_bound, Bound::Unbounded) && !matches!(upper_bound, Bound::Unbounded))
    {
        // Two ANY predicates may match different elements; PostgreSQL has no same-element range statistics.
        return None;
    }
    let mut clauses = PgList::<pg_sys::Node>::new();
    let exists = || {
        if is_array {
            // Null, empty, and all-null arrays have no value; column null statistics cannot distinguish them.
            return None;
        }
        Some(
            pg_sys::NullTest {
                xpr: pg_sys::Expr {
                    type_: pg_sys::NodeTag::T_NullTest,
                },
                arg: expr,
                nulltesttype: pg_sys::NullTestType::IS_NOT_NULL,
                argisrow: false,
                location: -1,
            }
            .palloc()
            .cast(),
        )
    };
    match query {
        pdb::Query::Exists => clauses.push(exists()?),
        pdb::Query::Term { value } => {
            let value: ConstNode = (value, oid).try_into().ok()?;
            clauses.push(Comparison::try_from((&*expr, c"=", value)).ok()?.into());
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
                clauses.push(Comparison::try_from((&*expr, name, value)).ok()?.into());
            }
            if clauses.is_empty() {
                clauses.push(exists()?);
            }
        }
        pdb::Query::RangeTerm { value } => {
            let subtype = pg_sys::get_range_subtype(oid);
            if subtype == pg_sys::InvalidOid {
                return None;
            }
            clauses.push(
                Comparison {
                    expr: &*expr,
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
                    expr: &*expr,
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
        | pdb::Query::RegexPhrase { .. }
        | pdb::Query::Proximity { .. } => unreachable!("text queries use Tantivy's statistics"),
        pdb::Query::Parse { .. } | pdb::Query::ParseWithField { .. } => {
            unreachable!("the caller uses the default selectivity for parsed queries")
        }
        pdb::Query::TermSet { .. } => unreachable!("the caller expands term sets"),
        pdb::Query::FastFieldRangeWeight { .. } => {
            unreachable!("the caller converts fast-field bounds to a range")
        }
        pdb::Query::UnclassifiedString { .. } | pdb::Query::UnclassifiedArray { .. } => {
            unreachable!("operator support functions must classify queries before estimation")
        }
    }
    Some(pg_sys::make_ands_explicit(clauses.into_pg()).cast())
}

struct FieldExpression(*mut pg_sys::Expr);

impl TryFrom<(&FieldSource, &PgSearchRelation, pg_sys::Index)> for FieldExpression {
    type Error = ();

    fn try_from(
        (source, index, rti): (&FieldSource, &PgSearchRelation, pg_sys::Index),
    ) -> Result<Self, Self::Error> {
        unsafe {
            let expr = match *source {
                FieldSource::Heap { attno } => {
                    let heap = index.heap_relation().ok_or(())?;
                    let desc = heap.tuple_desc();
                    let attr = desc.get(attno).ok_or(())?;
                    pg_sys::makeVar(
                        1,
                        (attno + 1) as _,
                        attr.atttypid,
                        attr.atttypmod,
                        attr.attcollation,
                        0,
                    )
                    .cast()
                }
                FieldSource::Expression { att_idx } => {
                    index.index_expressions().get_ptr(att_idx).ok_or(())?
                }
                FieldSource::CompositeField {
                    expression_idx,
                    field_idx,
                    composite_type_oid,
                    ..
                } => {
                    let expr = index
                        .index_expressions()
                        .get_ptr(expression_idx)
                        .ok_or(())?;
                    if let Some(row) = row_expr_from_indexed_expr(expr) {
                        PgList::<pg_sys::Expr>::from_pg((*row).args)
                            .get_ptr(field_idx)
                            .ok_or(())?
                    } else {
                        let fields =
                            get_composite_type_fields(composite_type_oid).map_err(|_| ())?;
                        let field = fields
                            .iter()
                            .find(|f| f.field_index == field_idx)
                            .ok_or(())?;
                        pg_sys::FieldSelect {
                            xpr: pg_sys::Expr {
                                type_: pg_sys::NodeTag::T_FieldSelect,
                            },
                            arg: expr,
                            fieldnum: (field_idx + 1) as _,
                            resulttype: field.type_oid,
                            resulttypmod: field.typmod,
                            resultcollid: pg_sys::get_typcollation(field.type_oid),
                        }
                        .palloc()
                        .cast()
                    }
                }
            };
            // Copy catalog expressions before rebinding their Vars to the query's table.
            let expr = pg_sys::copyObjectImpl(strip_tokenizer_cast(expr.cast()).cast()).cast();
            pg_sys::ChangeVarNodes(expr, 1, rti as _, 0);
            Ok(Self(expr.cast()))
        }
    }
}

struct JsonPredicate<'a> {
    field: &'a FieldName,
    query: &'a pdb::Query,
}

impl TryFrom<JsonPredicate<'_>> for ConstNode {
    type Error = ();

    fn try_from(predicate: JsonPredicate<'_>) -> Result<Self, Self::Error> {
        let literal = |value: &PdbOwnedValue| {
            match value {
                PdbOwnedValue::I64(_) | PdbOwnedValue::U64(_) | PdbOwnedValue::Bool(_) => {}
                PdbOwnedValue::F64(v) if v.is_finite() => {}
                // Text uses Tantivy; typed dates and other values have no equivalent JSON comparison here.
                _ => return Err(()),
            }
            serde_json::to_string(value).map_err(|_| ())
        };
        let mut path = String::from("lax $");
        for key in tantivy::json_utils::split_json_path(predicate.field)
            .into_iter()
            .skip(1)
        {
            path.push('.');
            path.push_str(&serde_json::to_string(&key).map_err(|_| ())?);
        }
        match predicate.query {
            pdb::Query::Term { value } => path.push_str(&format!(" ? (@ == {})", literal(value)?)),
            pdb::Query::Range {
                lower_bound,
                upper_bound,
            } => {
                let mut bounds = Vec::new();
                for (bound, inclusive, exclusive) in
                    [(lower_bound, ">=", ">"), (upper_bound, "<=", "<")]
                {
                    let (value, op) = match bound {
                        Bound::Included(value) => (value, inclusive),
                        Bound::Excluded(value) => (value, exclusive),
                        Bound::Unbounded => continue,
                    };
                    bounds.push(format!("@ {op} {}", literal(value)?));
                }
                if bounds.is_empty() {
                    // Without a bound, the JSON value type to compare is unknown.
                    return Err(());
                }
                path.push_str(&format!(" ? ({})", bounds.join(" && ")));
            }
            pdb::Query::Exists => path
                .push_str(".** ? (@ != null && @.type() != \"array\" && @.type() != \"object\")"),
            _ => return Err(()),
        }
        (&PdbOwnedValue::Str(path), pg_sys::JSONPATHOID)
            .try_into()
            .map_err(|_| ())
    }
}

struct Comparison<'a> {
    expr: &'a pg_sys::Expr,
    op: pg_sys::Oid,
    value: ConstNode,
}

impl<'a> TryFrom<(&'a pg_sys::Expr, &CStr, ConstNode)> for Comparison<'a> {
    type Error = ();

    fn try_from(
        (expr, name, value): (&'a pg_sys::Expr, &CStr, ConstNode),
    ) -> Result<Self, Self::Error> {
        unsafe {
            let mut names = PgList::<pg_sys::Node>::new();
            for name in [c"pg_catalog", name] {
                names.push(pg_sys::makeString(pg_sys::pstrdup(name.as_ptr())).cast());
            }
            let konst: *mut pg_sys::Const = (&value).into();
            let expr_oid = pg_sys::exprType(std::ptr::from_ref(expr).cast_mut().cast());
            let element_oid = pg_sys::get_base_element_type(expr_oid);
            let oid = if element_oid == pg_sys::InvalidOid {
                expr_oid
            } else {
                element_oid
            };
            let op = pg_sys::compatible_oper_opid(names.as_ptr(), oid, (*konst).consttype, true);
            if op == pg_sys::InvalidOid
                || (element_oid != pg_sys::InvalidOid
                    && pg_sys::get_commutator(op) == pg_sys::InvalidOid)
            {
                return Err(());
            }
            Ok(Self { expr, op, value })
        }
    }
}

impl From<Comparison<'_>> for *mut pg_sys::Node {
    fn from(comparison: Comparison<'_>) -> Self {
        let value: *mut pg_sys::Const = comparison.value.into();
        unsafe {
            let expr = std::ptr::from_ref(comparison.expr).cast_mut();
            let collation = pg_sys::exprCollation(expr.cast());
            if pg_sys::get_base_element_type(pg_sys::exprType(expr.cast())) != pg_sys::InvalidOid {
                let op = pg_sys::get_commutator(comparison.op);
                let mut args = PgList::new();
                args.push(value.cast::<pg_sys::Expr>());
                args.push(expr);
                return pg_sys::ScalarArrayOpExpr {
                    xpr: pg_sys::Expr {
                        type_: pg_sys::NodeTag::T_ScalarArrayOpExpr,
                    },
                    opno: op,
                    opfuncid: pg_sys::get_opcode(op),
                    useOr: true,
                    inputcollid: collation,
                    args: args.into_pg(),
                    location: -1,
                    ..Default::default()
                }
                .palloc()
                .cast();
            }
            pg_sys::make_opclause(
                comparison.op,
                pg_sys::BOOLOID,
                false,
                expr,
                value.cast(),
                pg_sys::InvalidOid,
                collation,
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
    /// Intersections use the cheapest required driver; unions traverse every child.
    pub(super) fn estimate_cost(&self, costs: &[(Occur, Option<u64>)]) -> Option<u64> {
        if self.must.is_empty() && self.should.is_empty()
            || self.minimum_should_match > self.should.len()
        {
            return Some(0);
        }
        let required = costs
            .iter()
            .filter(|(occur, _)| *occur == Occur::Must)
            .try_fold(u64::MAX, |cost, (_, child)| Some(cost.min((*child)?)))?;
        if !self.must.is_empty() && self.minimum_should_match == 0 {
            return Some(required);
        }
        let union = costs
            .iter()
            .filter(|(occur, _)| *occur == Occur::Should)
            .try_fold(0u64, |cost, (_, child)| {
                Some(cost.saturating_add((*child)?))
            })?;
        Some(required.min(union))
    }

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
    if let Some(query) = query.downcast_ref::<TermQuery>() {
        Box::new(TermQueryEstimate(query))
    } else if let Some(query) = query.downcast_ref::<BooleanQuery>() {
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

struct TermQueryEstimate<'a>(&'a TermQuery);

impl QueryEstimate for TermQueryEstimate<'_> {
    /// Posting counts estimate traversal work for text and non-text equality alike.
    fn estimate_docs(&self, reader: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
        let term = self.0.term();
        if !reader.schema().get_field_entry(term.field()).is_indexed() {
            // Unindexed fields have no posting counts.
            return Ok(None);
        }
        let count = reader.inverted_index(term.field())?.doc_freq(term)?;
        Ok(Some((count, u64::from(count))))
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
                (
                    Occur::Should,
                    TermQueryEstimate(&query).estimate_docs(reader),
                )
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
        let mut costs = Vec::new();
        for (occur, estimate) in children {
            let (count, cost) = estimate?.unwrap_or_else(|| {
                let matches = f64::from(reader.max_doc()) * crate::UNKNOWN_SELECTIVITY;
                (
                    matches.ceil() as u32,
                    (matches * crate::gucs::expensive_query_cost_factor()).ceil() as u64,
                )
            });
            costs.push((occur, Some(cost)));
            let clause =
                Selectivity(f64::from(count.min(reader.max_doc())) / f64::from(reader.max_doc()))
                    .into();
            match occur {
                Occur::Must => self.must.push(clause),
                Occur::Should => self.should.push(clause),
                Occur::MustNot => self.must_not.push(clause),
            }
        }
        let work = self
            .estimate_cost(&costs)
            .expect("child costs are available");
        let selectivity = unsafe { Selectivity::estimate(self.into_clause(None), None) };
        Ok(Some((
            (selectivity * f64::from(reader.max_doc())).ceil() as u32,
            work,
        )))
    }
}
