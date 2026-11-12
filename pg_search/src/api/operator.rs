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
#![allow(unsafe_op_in_unsafe_fn)]

pub use planner::api::operator::{
    ReturnedNodePointer, SearchPredicate, anyelement_query_input_opoid, anyelement_search_opoids,
    boost_typoid, const_typoid, estimate_selectivity, find_node_relation, fuzzy_typoid,
    is_search_with_query_input_exec_procoid, pdb_proximityclause_typoid, pdb_query_typoid,
    search_with_query_input_exec_procoids, searchqueryinput_typoid, slop_typoid,
    tantivy_field_name_from_node, with_index_procoid,
};

pub(crate) use crate::postgres::var::find_var_relation;

use crate::api::FieldName;
use crate::api::operator::boost::{BoostType, boost_to_boost};
use crate::api::operator::fuzzy::{FuzzyType, fuzzy_to_fuzzy};
use crate::api::operator::rewrite::SearchOperator;
use crate::api::operator::slop::{SlopType, slop_to_slop};
use crate::api::tokenizers::{type_can_be_tokenized, type_is_alias, type_is_tokenizer};
use crate::gucs::per_tuple_cost;
use crate::nodecast;
use crate::postgres::catalog::{is_citext_oid, lookup_type_name};
use crate::postgres::customscan::opexpr::expr_matches_node;
use crate::postgres::index::is_partitioned_index;
use crate::postgres::node::NodeExt;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::sequentialscan::MaybeInlineRow;
use crate::postgres::utils::ToPalloc;
#[cfg(feature = "pg18")]
use crate::postgres::var::resolve_rte_group_var;
use crate::postgres::var::system_column_var;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::pdb;
use crate::query::proximity::ProximityClause;
use pgrx::pg_sys::panic::ErrorReport;
use pgrx::*;

macro_rules! operator_support {
    ($vis:vis fn $name:ident, $operator:ident) => {
        #[pgrx::pg_extern(immutable, parallel_safe)]
        $vis fn $name(arg: pgrx::Internal) -> ReturnedNodePointer {
            use crate::api::operator::ReturnedNodePointerSupport;
            unsafe {
                ReturnedNodePointer::for_support_operator(arg, super::SearchOperator::$operator)
            }
        }
    };
}

mod andandand;
mod atatat;
pub(crate) mod boost;
pub(crate) mod const_score;
mod eqeqeq;
mod expand;
pub(crate) mod fuzzy;
mod hashhashhash;
mod ororor;
mod proximity;
mod rewrite;
mod searchqueryinput;
pub(crate) mod slop;

enum RHSValue {
    Text(String),
    TextArray(Vec<String>),
    PdbQuery(pdb::Query),
    ProximityClause(ProximityClause),
}

pub(crate) enum SimplifyRhs {
    SearchQueryInput,
    Rewrite(SearchOperator),
}

pub(crate) trait ReturnedNodePointerSupport {
    unsafe fn for_support_operator(arg: Internal, operator: SearchOperator) -> Self;
    unsafe fn for_support_simplify(arg: *mut pg_sys::Node, rhs_rewrite: SimplifyRhs) -> Self;
    unsafe fn for_support_index_condition(
        request: *mut pg_sys::SupportRequestIndexCondition,
    ) -> Self;
    unsafe fn for_support_selectivity(request: *mut pg_sys::SupportRequestSelectivity) -> Self;
    unsafe fn for_support_cost(request: *mut pg_sys::SupportRequestCost) -> Self;
}

impl ReturnedNodePointerSupport for ReturnedNodePointer {
    unsafe fn for_support_operator(arg: Internal, operator: SearchOperator) -> Self {
        Self::for_support_simplify(
            arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>(),
            SimplifyRhs::Rewrite(operator),
        )
    }

    unsafe fn for_support_simplify(arg: *mut pg_sys::Node, rhs_rewrite: SimplifyRhs) -> Self {
        // Other support request types are handled by their corresponding methods.
        let Some(request) = nodecast!(SupportRequestSimplify, arg) else {
            return Self::unsupported();
        };

        // Index and field resolution require planner context.
        if (*request).root.is_null() {
            return Self::unsupported();
        }

        let input_args = PgList::<pg_sys::Node>::from_pg((*(*request).fcall).args);
        // A malformed search call has no predicate to simplify.
        let (Some(lhs), Some(rhs)) = (input_args.get_ptr(0), input_args.get_ptr(1)) else {
            return Self::unsupported();
        };

        let original_lhs = input_args
            .iter_ptr()
            .last()
            .and_then(|node| nodecast!(RowExpr, node))
            .and_then(|row| record_field(row, c"original_lhs"));
        let lhs = original_lhs.unwrap_or(lhs);
        let Some(base_var) = lhs.find_node::<pg_sys::Var>() else {
            return Self::unsupported();
        };
        // Retain the field binding while EXISTS conversion or subquery pushdown can
        // change the original LHS's relation context.
        let derived = find_node_relation(lhs, (*request).root).2.is_some();
        let keep_original_lhs = ((*(*(*request).root).parse).hasSubLinks || derived)
            && (original_lhs.is_some()
                || matches!(rhs_rewrite, SimplifyRhs::Rewrite(_))
                    && get_expr_result_type(rhs) != searchqueryinput_typoid());

        // The LHS must identify a ParadeDB index, optionally with a specific field.
        let Some((indexrel, field)) = tantivy_field_name_from_node((*request).root, lhs) else {
            return Self::unsupported();
        };

        let inferred_field = original_lhs.and_then(|_| field.clone());
        let search_query_input_typoid = searchqueryinput_typoid();
        let mut selectivity_now = None;
        let rhs = match rhs_rewrite {
            SimplifyRhs::SearchQueryInput => {
                // The canonical search function only accepts an already typed RHS.
                if get_expr_result_type(rhs) != search_query_input_typoid {
                    return Self::unsupported();
                }
                rhs
            }
            SimplifyRhs::Rewrite(operator) => {
                let rhs = match (&field, operator) {
                    (Some(field), SearchOperator::Conjunction | SearchOperator::Disjunction) => {
                        let root = (*request).root;
                        match expand::replace_expand_subquery(root, rhs, &indexrel, field) {
                            Some(replaced) => replaced,
                            None => {
                                selectivity_now = expand::selectivity_now(
                                    root, operator, lhs, rhs, &indexrel, field,
                                );
                                rhs
                            }
                        }
                    }
                    _ => rhs,
                };
                rewrite_rhs_to_search_query_input(
                    operator,
                    search_query_input_typoid,
                    lhs,
                    rhs,
                    field,
                )
            }
        };

        assert_eq!(
            get_expr_result_type(rhs),
            search_query_input_typoid,
            "rhs must represent a SearchQueryInput"
        );

        #[cfg(feature = "pg18")]
        let base_var = resolve_lhs_var_for_group((*request).root, base_var);

        let original_lhs = lhs;
        let SearchLhs {
            node: lhs,
            supports_heap_fallback,
        } = if derived {
            SearchLhs {
                node: lhs,
                supports_heap_fallback: true,
            }
        } else {
            SearchLhs::from_index(&indexrel, base_var, lhs)
        };
        let Some(rhs) = wrap_with_index(&indexrel, rhs, inferred_field) else {
            return Self::unsupported();
        };
        if !supports_heap_fallback {
            // Preserve index planning; the scalar operator rejects heap execution.
            return Self::from_node(
                SearchPredicate {
                    lhs,
                    rhs,
                    location: (*(*request).fcall).location,
                }
                .into_opexpr()
                .cast(),
            );
        }
        let ctid = (!derived).then(|| {
            system_column_var(
                base_var,
                pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
                pg_sys::TIDOID,
            )
        });

        let mut args = PgList::<pg_sys::Node>::new();
        args.push(lhs);
        args.push(rhs);
        args.push(ctid.map_or_else(
            || {
                pg_sys::makeConst(
                    pg_sys::TIDOID,
                    -1,
                    pg_sys::Oid::INVALID,
                    size_of::<pg_sys::ItemPointerData>() as _,
                    pg_sys::ItemPointerData::default().into_datum().unwrap(),
                    false,
                    false,
                )
                .cast()
            },
            |ctid| ctid.cast(),
        ));

        let inline_row = MaybeInlineRow::new((*request).root, base_var, ctid, &indexrel);
        if let Some(row) = inline_row.as_ptr() {
            args.push(row);
        }

        let mut fields = PgList::<pg_sys::Node>::new();
        let mut names = PgList::<pg_sys::Node>::new();
        if keep_original_lhs {
            fields.push(original_lhs);
            names.push(pg_sys::makeString(pg_sys::pstrdup(c"original_lhs".as_ptr())).cast());
        }
        // A predicate that stays above the Append keeps the parent partitioned index, whose
        // storage does not exist (#4643). Ship the row's `tableoid` so execution can find
        // the partition's own leaf index. The relkind gate keeps every other plan unchanged.
        if !derived && is_partitioned_index(indexrel.oid()) {
            let tableoid = system_column_var(
                base_var,
                pg_sys::TableOidAttributeNumber as pg_sys::AttrNumber,
                pg_sys::OIDOID,
            );
            fields.push(tableoid.cast());
            names.push(pg_sys::makeString(pg_sys::pstrdup(c"tableoid".as_ptr())).cast());
        }
        // The estimate of a subquery that has no value until the query runs.
        if let Some(selectivity) = selectivity_now.and_then(IntoDatum::into_datum) {
            fields.push(
                pg_sys::makeConst(
                    pg_sys::FLOAT8OID,
                    -1,
                    pg_sys::Oid::INVALID,
                    size_of::<f64>() as _,
                    selectivity,
                    false,
                    true,
                )
                .cast(),
            );
            names.push(pg_sys::makeString(pg_sys::pstrdup(c"selectivity".as_ptr())).cast());
        }
        // A non-null record preserves strictness even when the original LHS is NULL.
        args.push(
            pg_sys::RowExpr {
                xpr: pg_sys::Expr {
                    type_: pg_sys::NodeTag::T_RowExpr,
                },
                args: fields.into_pg(),
                row_typeid: pg_sys::RECORDOID,
                row_format: pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
                colnames: names.into_pg(),
                location: -1,
            }
            .palloc()
            .cast(),
        );

        // Index tuple descriptors do not preserve the heap column's NOT NULL flag.
        // Use a strict helper only when the chosen anchor is a NOT NULL heap column.
        let heap_attno = nodecast!(Var, lhs).map_or(0, |var| (*var).varattno);
        let anchor_is_not_null = !derived
            && heap_attno > 0
            && indexrel.heap_relation().is_some_and(|heaprel| {
                heaprel
                    .tuple_desc()
                    .get(heap_attno as usize - 1)
                    .is_some_and(|attribute| attribute.attnotnull)
            });

        let function = pg_sys::makeFuncExpr(
            inline_row.procoid(anchor_is_not_null),
            pg_sys::BOOLOID,
            args.into_pg(),
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
        );
        (*function).location = (*(*request).fcall).location;

        Self::from_node(function.cast())
    }

    unsafe fn for_support_index_condition(
        request: *mut pg_sys::SupportRequestIndexCondition,
    ) -> Self {
        unsafe {
            if !is_search_with_query_input_exec_procoid((*request).funcid)
                || (*request).indexarg != 0
                || !pg_sys::op_in_opfamily(anyelement_query_input_opoid(), (*request).opfamily)
            {
                return Self::unsupported();
            }

            let Some(predicate) = SearchPredicate::from_node((*request).node) else {
                return Self::unsupported();
            };
            if !pg_sys::is_pseudo_constant_for_index(
                (*request).root,
                predicate.rhs,
                (*request).index,
            ) {
                return Self::unsupported();
            }

            let scalar_predicate = predicate.into_opexpr();
            let mut conditions = PgList::<pg_sys::Node>::new();
            conditions.push(scalar_predicate.cast());
            (*request).lossy = false;
            Self::from_node(conditions.into_pg().cast())
        }
    }

    unsafe fn for_support_selectivity(request: *mut pg_sys::SupportRequestSelectivity) -> Self {
        unsafe {
            if !is_search_with_query_input_exec_procoid((*request).funcid) {
                return Self::unsupported();
            }

            (*request).selectivity =
                searchqueryinput::query_input_selectivity((*request).root, (*request).args);
            Self::from_node(request.cast())
        }
    }

    unsafe fn for_support_cost(request: *mut pg_sys::SupportRequestCost) -> Self {
        unsafe {
            let root = (*request).root;
            let node = (*request).node;
            // A projected match can't steer path choice and only costs a set lookup per row.
            if !root.is_null()
                && (*root)
                    .processed_tlist
                    .cast::<pg_sys::Node>()
                    .any(|n| std::ptr::eq(n, node))
            {
                (*request).startup = 0.0;
                (*request).per_tuple = pg_sys::cpu_operator_cost;
            } else {
                // Heap execution materializes matching CTIDs, so keep its function cost high enough
                // that PostgreSQL prefers the index AM whenever one is available.
                (*request).startup = per_tuple_cost();
                (*request).per_tuple = per_tuple_cost();
            }
            Self::from_node(request.cast())
        }
    }
}

unsafe fn get_expr_result_type(expr: *mut pg_sys::Node) -> pg_sys::Oid {
    let mut typoid = pg_sys::Oid::INVALID;
    let mut tupdesc = pg_sys::TupleDesc::default();
    pg_sys::get_expr_result_type(expr, &mut typoid, &mut tupdesc);
    if !tupdesc.is_null() {
        pg_sys::FreeTupleDesc(tupdesc);
    }
    typoid
}

struct SearchLhs {
    node: *mut pg_sys::Node,
    supports_heap_fallback: bool,
}

impl SearchLhs {
    unsafe fn from_index(
        indexrel: &PgSearchRelation,
        base_var: *mut pg_sys::Var,
        original_lhs: *mut pg_sys::Node,
    ) -> Self {
        let index_info = unsafe { *indexrel.index_info() };
        let tupdesc = indexrel.tuple_desc();
        // Recheck expressions need a returnable anchor for index-only scans.
        let index_attribute = (0..tupdesc.len())
            .find(|&attno| pg_sys::index_can_return(indexrel.as_ptr(), attno as i32 + 1))
            .or_else(|| (0..tupdesc.len()).find(|&attno| index_info.ii_IndexAttrNumbers[attno] > 0))
            .unwrap_or(0);
        let heap_attno = index_info.ii_IndexAttrNumbers[index_attribute];

        // Zero identifies an indexed expression rather than a heap column.
        if heap_attno == 0 {
            let expressions = indexrel.index_expressions();
            let is_partial = !pg_sys::RelationGetIndexPredicate(indexrel.as_ptr()).is_null();
            let expression = if is_partial {
                let Some(expression) = expressions
                    .iter_ptr()
                    .find(|&expression| expr_matches_node(original_lhs, expression, type_is_alias))
                else {
                    return Self {
                        node: original_lhs,
                        supports_heap_fallback: false,
                    };
                };
                expression
            } else {
                expressions
                    .get_ptr(0)
                    .expect("first index attribute should have an expression")
            };
            let expression = pg_sys::copyObjectImpl(expression.cast()).cast::<pg_sys::Node>();
            for var in expression.collect_nodes::<pg_sys::Var>() {
                (*var).varno = (*base_var).varno;
                (*var).varnosyn = (*base_var).varnosyn;
                (*var).varlevelsup = (*base_var).varlevelsup;
                #[cfg(not(feature = "pg15"))]
                {
                    (*var).varnullingrels = pg_sys::bms_copy((*base_var).varnullingrels);
                }
                #[cfg(feature = "pg18")]
                {
                    (*var).varreturningtype = (*base_var).varreturningtype;
                }
            }
            return Self {
                node: expression,
                supports_heap_fallback: !is_partial,
            };
        }

        let att = tupdesc
            .get(index_attribute)
            .expect("`USING paradedb` index must have at least one attribute");

        let var = pg_sys::copyObjectImpl(base_var.cast()).cast::<pg_sys::Var>();

        (*var).varattno = index_info.ii_IndexAttrNumbers[index_attribute];
        (*var).varattnosyn = (*var).varattno;

        (*var).vartype = att.atttypid;
        (*var).vartypmod = att.atttypmod;
        (*var).varcollid = att.attcollation;

        Self {
            node: var.cast(),
            supports_heap_fallback: true,
        }
    }
}

#[cfg(feature = "pg18")]
#[inline]
unsafe fn resolve_lhs_var_for_group(
    root: *mut pg_sys::PlannerInfo,
    var: *mut pg_sys::Var,
) -> *mut pg_sys::Var {
    let varno = (*var).varno as pg_sys::Index;
    let rtable = (*(*root).parse).rtable;

    // Bounds check: varno is 1-indexed and must be within the rtable
    let rtable_size = if !rtable.is_null() {
        PgList::<pg_sys::RangeTblEntry>::from_pg(rtable).len()
    } else {
        0
    };
    if varno == 0 || varno as usize > rtable_size {
        return var;
    }

    let rte = pgrx::rt_fetch!(varno, rtable).get();
    if (*rte).rtekind == pg_sys::RTEKind::RTE_GROUP {
        // PG18 introduces RTE_GROUP for GROUP BY expressions. When a Var references
        // RTE_GROUP, it points to a synthetic range table entry rather than the base
        // relation. We must resolve these Vars back to their originating column so
        // that operator rewriting can correctly identify the indexed field.
        if let Some(group_var) = resolve_rte_group_var(rte, (*var).varattno) {
            return group_var;
        }
    }

    var
}

/// The field of the trailing record argument carrying `name`, e.g. the preserved
/// original LHS or the partition identity (#4643). The names distinguish the fields, so
/// one is never misread as another.
unsafe fn record_field(
    row: *mut pg_sys::RowExpr,
    name: &core::ffi::CStr,
) -> Option<*mut pg_sys::Node> {
    unsafe {
        let names = PgList::<pg_sys::String>::from_pg((*row).colnames);
        let position = names.iter_ptr().position(|colname| {
            !(*colname).sval.is_null() && core::ffi::CStr::from_ptr((*colname).sval) == name
        })?;
        PgList::<pg_sys::Node>::from_pg((*row).args).get_ptr(position)
    }
}

unsafe fn wrap_with_index(
    indexrel: &PgSearchRelation,
    rhs: *mut pg_sys::Node,
    inferred_field: Option<FieldName>,
) -> Option<*mut pg_sys::Node> {
    if let Some(rhs_const) = nodecast!(Const, rhs) {
        // Const nodes are always of type SearchQueryInput, so we can instantiate a new Const version
        let query =
            SearchQueryInput::from_datum((*rhs_const).constvalue, (*rhs_const).constisnull)?;
        let mut query = match query {
            SearchQueryInput::WithIndex { query, .. } => query,
            query => Box::new(query),
        };
        if let Some(field) = inferred_field {
            query.visit(&mut |query| {
                if let SearchQueryInput::FieldedQuery { field: bound, .. } = query {
                    *bound = field.clone();
                }
            });
        }
        let query = SearchQueryInput::WithIndex {
            oid: indexrel.oid(),
            query,
        };
        let as_const: *mut pg_sys::Const = query.into();
        Some(as_const.cast())
    } else {
        // Replace constant index bindings, but preserve user expressions and their effects.
        let mut rhs = rhs;
        if let Some(function) = nodecast!(FuncExpr, rhs)
            && (*function).funcid == with_index_procoid()
        {
            let args = PgList::<pg_sys::Node>::from_pg((*function).args);
            if let Some(index) = args.get_ptr(0)
                && nodecast!(Const, index).is_some()
            {
                rhs = args
                    .get_ptr(1)
                    .expect("with_index must have a query argument");
            }
        }
        if let Some(field) = inferred_field {
            if nodecast!(Const, rhs).is_some() {
                return wrap_with_index(indexrel, rhs, Some(field));
            }
            let function = nodecast!(FuncExpr, rhs)?;
            let inputs = PgList::<pg_sys::Node>::from_pg((*function).args);
            let bound_field = nodecast!(Const, inputs.get_ptr(0)?)?;
            if (*bound_field).consttype != crate::api::fieldname_typoid() {
                return None;
            }
            let function = pg_sys::copyObjectImpl(function.cast()).cast::<pg_sys::FuncExpr>();
            let mut inputs = PgList::<pg_sys::Node>::from_pg((*function).args);
            inputs.replace_ptr(0, field.into_const().cast());
            (*function).args = inputs.into_pg();
            rhs = function.cast();
        }
        // otherwise we need to wrap the rhs in a `FuncExpr` that calls `paradedb.with_index()`
        let mut args = PgList::<pg_sys::Node>::new();
        args.push(
            pg_sys::makeConst(
                pg_sys::REGCLASSOID,
                -1,
                pg_sys::Oid::INVALID,
                size_of::<pg_sys::Oid>() as _,
                indexrel.oid().into_datum().unwrap(),
                false,
                true,
            )
            .cast(),
        );
        args.push(rhs);

        Some(
            pg_sys::makeFuncExpr(
                with_index_procoid(),
                searchqueryinput_typoid(),
                args.into_pg(),
                pg_sys::Oid::INVALID,
                pg_sys::Oid::INVALID,
                pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
            )
            .cast(),
        )
    }
}

unsafe fn rewrite_rhs_to_search_query_input(
    operator: SearchOperator,
    search_query_input_typoid: pg_sys::Oid,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
    field: Option<FieldName>,
) -> *mut pg_sys::Node {
    let rhs: *mut pg_sys::Node = if get_expr_result_type(rhs) == search_query_input_typoid {
        // the rhs is already of type SearchQueryInput, so we can use it directly
        rhs
    } else if let Some(const_) = nodecast!(Const, rhs) {
        let rhs_value = match (*const_).consttype {
            // these are used for the @@@, &&&, |||, ###, and === operators
            pg_sys::TEXTOID | pg_sys::VARCHAROID => RHSValue::Text(
                String::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text value must not be NULL"),
            ),
            other if is_citext_oid(other) => RHSValue::Text(
                String::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text value must not be NULL"),
            ),
            pg_sys::TEXTARRAYOID | pg_sys::VARCHARARRAYOID => RHSValue::TextArray(
                Vec::<String>::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs text array value must not be NULL"),
            ),

            other if other == pdb_query_typoid() => RHSValue::PdbQuery(
                pdb::Query::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fielded query input value must not be NULL"),
            ),

            other if other == boost_typoid() => {
                let boost = BoostType::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs boost value must not be NULL");
                let boost = boost_to_boost(boost, (*const_).consttypmod, true);
                RHSValue::PdbQuery(boost.into())
            }

            other if other == fuzzy_typoid() => {
                let fuzzy = FuzzyType::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fuzzy value must not be NULL");
                let fuzzy = fuzzy_to_fuzzy(fuzzy, (*const_).consttypmod, true);
                RHSValue::PdbQuery(fuzzy.into())
            }

            other if other == slop_typoid() => {
                let slop = SlopType::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs slop value must not be NULL");
                let slop = slop_to_slop(slop, (*const_).consttypmod, true);
                RHSValue::PdbQuery(slop.into())
            }

            other if other == pdb_proximityclause_typoid() => {
                let prox = ProximityClause::from_datum((*const_).constvalue, (*const_).constisnull)
                    .expect("rhs fielded proximity clause must not be NULL");
                RHSValue::ProximityClause(prox)
            }

            other => panic!("operator does not support rhs type {other}"),
        };

        let query: *mut pg_sys::Const = operator.rewrite_const(lhs, field, rhs_value).into();
        query.cast()
    } else {
        // the rhs is a complex expression that needs to be evaluated at runtime
        // but its return type is not SearchQueryInput, so we need to rewrite it
        operator.rewrite_exec(field, lhs, rhs).cast()
    };
    rhs
}

/// Returns `true` if the given OID is a text-like scalar type (text, varchar, or citext).
fn is_text_like(oid: pg_sys::Oid) -> bool {
    oid == pg_sys::TEXTOID || oid == pg_sys::VARCHAROID || is_citext_oid(oid)
}

/// Returns `true` if the given OID is a text-like array type (text[] or varchar[]).
fn is_text_array_like(oid: pg_sys::Oid) -> bool {
    oid == pg_sys::TEXTARRAYOID || oid == pg_sys::VARCHARARRAYOID
}

/// Build a runtime [`pg_sys::FuncExpr`] for a text-or-text-array RHS.
///
/// Checks the RHS expression type and dispatches to the appropriate builder function.
/// `text_fn` and `array_fn` are `regprocedure` strings for the scalar and array variants.
unsafe fn build_text_funcexpr(
    field: FieldName,
    rhs: *mut pg_sys::Node,
    operator_name: &str,
    text_fn: &std::ffi::CStr,
    array_fn: &std::ffi::CStr,
) -> *mut pg_sys::FuncExpr {
    let expr_type = get_expr_result_type(rhs);
    let is_array = is_text_array_like(expr_type);
    if !(is_text_like(expr_type) || is_array) {
        panic!(
            "The right-hand side of the `{operator_name}` operator must be a text or text array value"
        );
    }

    let sig = if is_array { array_fn } else { text_fn };
    let funcid = direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[sig.into_datum()])
        .unwrap_or_else(|| panic!("`{}` should exist", sig.to_str().unwrap()));

    let mut args = PgList::<pg_sys::Node>::new();
    args.push(field.into_const().cast());
    args.push(rhs.cast());

    pg_sys::makeFuncExpr(
        funcid,
        searchqueryinput_typoid(),
        args.into_pg(),
        pg_sys::Oid::INVALID,
        pg_sys::Oid::INVALID,
        pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
    )
}

/// A ParadeDB `pdb.*` type that participates as an operator RHS, either directly
/// (`pdb.query`) or via an implicit cast to `pdb.query`. Operator runtime rewrite paths use
/// it to decide whether an RHS whose type is a `pdb.*` composite needs the runtime pdb.query
/// dispatch rather than the text/text[] dispatch, and which cast lifts it there.
#[derive(Clone, Copy)]
pub(crate) enum PdbQueryRhs {
    Query,
    Fuzzy,
    Boost,
    Slop,
    Const,
}

impl PdbQueryRhs {
    pub(crate) fn from_oid(oid: pg_sys::Oid) -> Option<Self> {
        if oid == pdb_query_typoid() {
            Some(Self::Query)
        } else if oid == fuzzy_typoid() {
            Some(Self::Fuzzy)
        } else if oid == boost_typoid() {
            Some(Self::Boost)
        } else if oid == slop_typoid() {
            Some(Self::Slop)
        } else if oid == const_typoid() {
            Some(Self::Const)
        } else {
            None
        }
    }

    /// The `*_to_query` cast that lifts this type to `pdb.query`, or `None` when the value
    /// already is one.
    fn cast_sig(self) -> Option<&'static std::ffi::CStr> {
        match self {
            Self::Query => None,
            Self::Fuzzy => Some(c"paradedb.fuzzy_to_query(pdb.fuzzy)"),
            Self::Boost => Some(c"paradedb.boost_to_query(pdb.boost)"),
            Self::Slop => Some(c"paradedb.slop_to_query(pdb.slop)"),
            Self::Const => Some(c"paradedb.const_to_query(pdb.const)"),
        }
    }
}

/// Build a [`pg_sys::FuncExpr`] that calls `outer_sig(fieldname, pdb.query)` on an RHS whose
/// declared type is the given [`PdbQueryRhs`].
pub(crate) unsafe fn build_pdb_query_funcexpr(
    field: FieldName,
    rhs: *mut pg_sys::Node,
    rhs_kind: PdbQueryRhs,
    outer_sig: &std::ffi::CStr,
) -> *mut pg_sys::FuncExpr {
    let coerced_rhs: *mut pg_sys::Node = if let Some(cast_sig) = rhs_kind.cast_sig() {
        let cast_funcid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[cast_sig.into_datum()])
                .unwrap_or_else(|| panic!("`{}` should exist", cast_sig.to_str().unwrap()));

        let mut cast_args = PgList::<pg_sys::Node>::new();
        cast_args.push(rhs);
        pg_sys::makeFuncExpr(
            cast_funcid,
            pdb_query_typoid(),
            cast_args.into_pg(),
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_IMPLICIT_CAST,
        )
        .cast()
    } else {
        rhs
    };

    let outer_funcid =
        direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[outer_sig.into_datum()])
            .unwrap_or_else(|| panic!("`{}` should exist", outer_sig.to_str().unwrap()));

    let mut args = PgList::<pg_sys::Node>::new();
    args.push(field.into_const().cast());
    args.push(coerced_rhs);

    pg_sys::makeFuncExpr(
        outer_funcid,
        searchqueryinput_typoid(),
        args.into_pg(),
        pg_sys::Oid::INVALID,
        pg_sys::Oid::INVALID,
        pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
    )
}

#[track_caller]
#[inline]
unsafe fn validate_lhs_type_as_text_compatible(lhs: *mut pg_sys::Node, operator_name: &str) {
    let typoid = pg_sys::exprType(lhs);
    if !type_can_be_tokenized(typoid) && !type_is_tokenizer(typoid) {
        let typname = lookup_type_name(typoid).unwrap_or_else(|| String::from("<unknown type>"));
        ErrorReport::new(
            PgSqlErrorCode::ERRCODE_SYNTAX_ERROR,
            format!("type `{typname}` is not compatible with the `{operator_name}` operator"),
            function_name!(),
        )
        .report(PgLogLevel::ERROR);
    }
}

extension_sql!(
    r#"
ALTER FUNCTION paradedb.search_with_query_input SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_strict SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row SUPPORT paradedb.query_input_support;
ALTER FUNCTION paradedb.search_with_query_input_ctid_or_row_strict SUPPORT paradedb.query_input_support;

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_query_input,
    LEFTARG = anyelement,
    RIGHTARG = paradedb.searchqueryinput,
    RESTRICT = query_input_restrict
);

CREATE OPERATOR CLASS anyelement_paradedb_ops DEFAULT FOR TYPE anyelement USING paradedb AS
    OPERATOR 1 pg_catalog.@@@(anyelement, text),                         /* for querying with a tantivy-compatible text query */
    OPERATOR 2 pg_catalog.@@@(anyelement, paradedb.searchqueryinput),    /* for querying with a paradedb.searchqueryinput structure */
    STORAGE anyelement;

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, text),                         /* for querying with a tantivy-compatible text query */
    OPERATOR 2 pg_catalog.@@@(anyelement, paradedb.searchqueryinput),    /* for querying with a paradedb.searchqueryinput structure */
    STORAGE anyelement;

CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;

CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;

CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING bm25 AS
    STORAGE public.vector;

CREATE OPERATOR CLASS public.vector_l2_ops DEFAULT FOR TYPE public.vector USING paradedb AS
    STORAGE public.vector;

CREATE OPERATOR CLASS public.vector_cosine_ops FOR TYPE public.vector USING paradedb AS
    STORAGE public.vector;

CREATE OPERATOR CLASS public.vector_ip_ops FOR TYPE public.vector USING paradedb AS
    STORAGE public.vector;
"#,
    name = "bm25_ops_anyelement_operator",
    requires = [
        // for using plain text on the rhs
        atatat::search_with_parse,
        // for using SearchQueryInput on the rhs
        sequentialscan::search_with_query_input,
        sequentialscan::search_with_query_input_ctid,
        sequentialscan::search_with_query_input_ctid_strict,
        sequentialscan::search_with_query_input_ctid_or_row,
        sequentialscan::search_with_query_input_ctid_or_row_strict,
        sequentialscan::ctid_is_valid,
        sequentialscan::xmin_is_visible,
        searchqueryinput::query_input_restrict,
        searchqueryinput::query_input_support,
    ]
);

mod f16_typmod {
    // we clamp the user-provided typmod bounds to this so that we're sure they'll fit after being
    // converted to an f16 without accuracy loss on integer values
    pub(in crate::api::operator) const TYPMOD_BOUNDS: (f32, f32) = (-2048.0, 2048.0);

    /// Serialize an f32 to a non-negative i32 by first converting to f16.
    /// Panics if val is NaN/Inf or out of f16's representable range.
    pub fn serialize_f32_to_i32(val: f32) -> i32 {
        assert!(
            val.is_finite() && val >= TYPMOD_BOUNDS.0 && val <= TYPMOD_BOUNDS.1,
            "only 16 bit floats in the range [{}..{}] are supported",
            TYPMOD_BOUNDS.0,
            TYPMOD_BOUNDS.1
        );
        let half: half::f16 = half::f16::from_f32(val);
        let bits: u16 = half.to_bits();
        bits as i32 // in [0, 0xFFFF], always >= 0
    }

    /// Deserialize the i32 back to a f32 via f16.
    /// Panics if encoded is outside [0, 65535].
    pub fn deserialize_i32_to_f32(encoded: i32) -> f32 {
        assert!(
            (0..=u16::MAX as i32).contains(&encoded),
            "invalid typemod `{encoded}`: must be between 0 and {}",
            u16::MAX
        );

        let bits: u16 = encoded as u16;
        let half = half::f16::from_bits(bits);
        half.to_f32()
    }

    #[test]
    fn roundtrip() {
        use proptest::proptest;

        proptest!(|(typmod in TYPMOD_BOUNDS.0 as i32..TYPMOD_BOUNDS.1 as i32)| {
            let encoded = serialize_f32_to_i32(typmod as f32);
            let decoded = deserialize_i32_to_f32(encoded) as i32;
            assert!(typmod == decoded, "typmod={typmod}, decoded={decoded}");
        });
    }
}
