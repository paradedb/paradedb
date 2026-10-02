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

/// Estimate the equivalent SQL predicate using PostgreSQL's statistics and defaults.
pub(super) unsafe fn non_text_selectivity(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    index: &PgSearchRelation,
    field: &FieldName,
    query: &pdb::Query,
) -> Option<f64> {
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
        pdb::Query::TermSet { .. } => {
            todo!("estimate individual terms and combine them in the caller")
        }
        pdb::Query::FastFieldRangeWeight { .. } => {
            todo!("translate fast-field bounds into PostgreSQL column values")
        }
        pdb::Query::Proximity { .. } => todo!("implement a text estimator for proximity"),
        pdb::Query::UnclassifiedString { .. } | pdb::Query::UnclassifiedArray { .. } => {
            unreachable!("operator support functions must classify queries before estimation")
        }
    }
    Some(pg_sys::clauselist_selectivity(
        root,
        clauses.as_ptr(),
        rti as _,
        pg_sys::JoinType::JOIN_INNER,
        null_mut(),
    ))
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
