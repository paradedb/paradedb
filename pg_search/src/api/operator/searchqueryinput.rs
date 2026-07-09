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
use super::inline_eval::{self, BoundField, ElementValue, FieldKind, InlinePredicate};
use super::{anyelement_query_input_opoid, request_simplify};
use crate::api::operator::{estimate_selectivity, find_var_relation, ReturnedNodePointer};
use crate::api::HashMap;
use crate::gucs::per_tuple_cost;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::types::TantivyValue;
use crate::query::pdb_query::resolve_search_tokenizer;
use crate::query::SearchQueryInput;
use crate::schema::SearchFieldType;
use crate::{nodecast, PARAMETERIZED_SELECTIVITY, UNKNOWN_SELECTIVITY};
use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
};
use pgrx::{
    pg_extern, pg_func_extra, pg_getarg_datum_raw, pg_getarg_type, pg_sys, FromDatum, Internal,
    PgList, PgOid, PgRelation,
};
use std::ptr::NonNull;

/// SQL API for allowing the user to specify the index to query.
///
/// This is useful (required, even) in cases where a query must be planned a sequential scan.
///
/// An example might be a query like this, that reads "find everything from `t` where the `body` field
/// contains a term from the `keywords` field.
///
/// ```sql
/// SELECT * FROM t WHERE key_field @@@ paradedb.term('body', keywords);
/// ```
///
/// In order for pg_search to execute this, we need to know the index to use, so it would need to be written
/// as:
///
/// ```sql
/// SELECT * FROM t WHERE key_field @@@ paradedb.with_index('bm25_idxt', paradedb.term('body', keywords));
/// ```
#[pg_extern(immutable, parallel_safe)]
pub fn with_index(index: PgRelation, query: SearchQueryInput) -> SearchQueryInput {
    SearchQueryInput::WithIndex {
        oid: index.oid(),
        query: Box::new(query),
    }
}

/// Per-call-site state for [`search_with_query_input`].
///
/// The left-hand-side field (the field whose value the operator receives) and its Postgres
/// type are fixed for a given plan node, so we resolve them once. The query (right-hand side)
/// may vary per row when it references a column, so compiled predicates are memoized per
/// distinct query datum.
struct Cache {
    /// The Postgres type OID of the left-hand-side value.
    element_oid: PgOid,
    /// The field the LHS value belongs to (plus how to interpret it), resolved from the operator's
    /// expression node. `None` if it could not be resolved (e.g. the LHS is not a plain indexed
    /// column), which makes every query unsupported.
    bound: Option<BoundField>,
    /// Memoized compiled predicate (or the reason it is unsupported) per query datum.
    by_query: HashMap<Vec<u8>, Result<InlinePredicate, String>>,
}

/// Allows us to have a UDF with an argument of type `anyelement` but not do any pgrx-related
/// datum conversion
pub struct FakeAnyElement;

/// Allows us to have a UDF with an argument of type `SearchQueryInput` but not do any pgrx-related
/// datum conversion
pub struct FakeSearchQueryInput;

unsafe impl<'fcx> ArgAbi<'fcx> for FakeAnyElement {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

// Unlike `FakeSearchQueryInput` below, this does not borrow another type's resolution
// because `anyelement` is a Postgres pseudo-type with no graph entry to depend on.
unsafe impl SqlTranslatable for FakeAnyElement {
    const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(FakeAnyElement);
    const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        Ok(SqlMappingRef::literal("anyelement"));
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

unsafe impl<'fcx> ArgAbi<'fcx> for FakeSearchQueryInput {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

unsafe impl SqlTranslatable for FakeSearchQueryInput {
    // This is intentionally borrowing `SearchQueryInput`'s `TYPE_IDENT`: current
    // pgrx tolerates two Rust types sharing that identifier, and we rely on that
    // observed behavior so the SQL graph still emits `CREATE TYPE SearchQueryInput`
    // before any function that consumes this fake wrapper.
    const TYPE_IDENT: &'static str = <SearchQueryInput as SqlTranslatable>::TYPE_IDENT;
    const TYPE_ORIGIN: TypeOrigin = <SearchQueryInput as SqlTranslatable>::TYPE_ORIGIN;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        <SearchQueryInput as SqlTranslatable>::ARGUMENT_SQL;
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

/// Resolve the field whose value appears on the left-hand side of the operator (and how to
/// interpret it), by inspecting the operator's expression node (`flinfo->fn_expr`).
///
/// Returns `None` when the LHS is not a plain indexed column we can resolve, or the query is not
/// scoped to an index — in which case every query is treated as unsupported.
unsafe fn resolve_bound_field(
    fcinfo: pg_sys::FunctionCallInfo,
    query: &SearchQueryInput,
) -> Option<BoundField> {
    let flinfo = (*fcinfo).flinfo;
    if flinfo.is_null() || (*flinfo).fn_expr.is_null() {
        return None;
    }
    let fn_expr = (*flinfo).fn_expr;

    // As an operator qual the node is an `OpExpr`; a direct function call is a `FuncExpr`.
    // Both expose their argument list the same way; the LHS is the first argument.
    let args = if let Some(op) = nodecast!(OpExpr, T_OpExpr, fn_expr) {
        PgList::<pg_sys::Node>::from_pg((*op).args)
    } else if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, fn_expr) {
        PgList::<pg_sys::Node>::from_pg((*func).args)
    } else {
        return None;
    };
    let lhs = normalize_lhs_attr(args.get_ptr(0)?);

    let index_oid = query.index_oid()?;
    let index_relation =
        PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
    let heap_relation = index_relation.heap_relation()?;

    let field_name = super::field_name_from_node(
        crate::postgres::var::VarContext::from_exec(heap_relation.oid()),
        &heap_relation,
        &index_relation,
        lhs,
    )?;

    let schema = index_relation.schema().ok()?;
    let search_field = schema.search_field(field_name.root())?;

    // Only genuine text columns are analyzed: their datum is a Rust `String` we can tokenize. Other
    // `is_str()` field types (uuid, inet, hex-encoded numeric bytes, composites, ...) are stored as
    // strings internally but their *column* value is not text, so they are compared exactly instead.
    // Resolving the analyzer needs a `Searcher`, so we open a reader once (no search is performed, so
    // this stays O(1) in memory).
    let kind = match search_field.field_type() {
        SearchFieldType::Text(_) | SearchFieldType::Tokenized(..) => {
            let reader = SearchIndexReader::open(
                &index_relation,
                query.clone(),
                false,
                MvccSatisfies::Snapshot,
            )
            .ok()?;
            let analyzer =
                resolve_search_tokenizer(&search_field, &schema, reader.searcher()).ok()?;
            FieldKind::Tokenized(analyzer)
        }
        ft => FieldKind::Raw {
            integer: matches!(ft, SearchFieldType::I64(_)),
        },
    };

    Some(BoundField {
        name: field_name,
        kind,
    })
}

/// If `node` is a `Var` that `setrefs` translated (e.g. a join filter above a Custom Scan, where
/// `varattno` becomes a position in the child's projected target list), rewrite a copy so its
/// `varattno` is the original heap attribute number preserved in `varattnosyn`. This lets field
/// resolution work off the real column rather than a tuple slot position.
unsafe fn normalize_lhs_attr(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if let Some(var) = nodecast!(Var, T_Var, node) {
        if (*var).varattnosyn > 0 && (*var).varattnosyn != (*var).varattno {
            let copy = pg_sys::copyObjectImpl(var.cast()).cast::<pg_sys::Var>();
            (*copy).varattno = (*copy).varattnosyn;
            return copy.cast();
        }
    }
    node
}

/// Per-row evaluation of a search operator (`@@@`, `&&&`, `|||`, `###`, `===`) when it could
/// not be pushed down to the BM25 index.
///
/// The query is evaluated *inline* against `element` — the value of the field on the operator's
/// left-hand side for the current row — rather than searching the index and materializing every
/// matching key. This is only possible when every predicate targets that same field; otherwise
/// (or for constructs we cannot evaluate inline) we raise an error rather than silently falling
/// back to an unbounded materialization.
#[allow(unused_variables)]
#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input(
    element: FakeAnyElement,
    query: FakeSearchQueryInput,
    fcinfo: pg_sys::FunctionCallInfo,
) -> Option<bool> {
    assert!(
        unsafe { (*(*fcinfo).flinfo).fn_strict },
        "paradedb.search_with_query_input must be STRICT"
    );

    // Because this function is STRICT, neither argument is ever SQL NULL.
    let query_datum = unsafe { pg_getarg_datum_raw(fcinfo, 1) };

    // Per-call-site state: the LHS type and bound field are fixed for this plan node, so resolve
    // them once (using the first query datum to locate the index).
    let mut cache = unsafe {
        pg_func_extra(fcinfo, || {
            let element_oid = PgOid::from_untagged(pg_getarg_type(fcinfo, 0));
            let first_query = SearchQueryInput::from_datum(query_datum, false)
                .expect("the query argument cannot be NULL");
            let bound = resolve_bound_field(fcinfo, &first_query);
            Cache {
                element_oid,
                bound,
                by_query: HashMap::default(),
            }
        })
    };
    // Split the borrows so `by_query` (mutated) and `bound` (read) can be used together.
    let Cache {
        element_oid,
        bound,
        by_query,
    } = &mut *cache;
    let element_oid = *element_oid;
    let bound = bound.as_ref();

    // Compile the query into a predicate over the bound field's value, memoized per distinct
    // query datum (the query can vary per row when it references a column).
    let key = unsafe {
        let varlena = query_datum.cast_mut_ptr::<pg_sys::varlena>();
        pgrx::varlena_to_byte_slice(varlena).to_vec()
    };
    let compiled = by_query.entry(key).or_insert_with(|| {
        let search_query_input = unsafe {
            SearchQueryInput::from_datum(query_datum, false)
                .expect("the query argument cannot be NULL")
        };
        inline_eval::compile(&search_query_input, bound).map_err(|e| e.0)
    });

    match compiled {
        Ok(predicate) => {
            let element = build_element_value(fcinfo, element_oid, bound);
            Some(predicate.eval(&element))
        }
        Err(reason) => {
            pgrx::error!(
                "pg_search: this search predicate cannot be evaluated as a per-row filter: {reason}. \
                 It requires the BM25 index scan; ensure `paradedb.enable_custom_scan` is on and the \
                 query can use the index, or rewrite it so the searched field is on the left-hand side \
                 of the operator."
            )
        }
    }
}

/// Build the current row's bound-field value for evaluation. For a tokenized text field the value
/// is analyzed into its token set (matching index-time analysis); otherwise it is a raw scalar.
fn build_element_value(
    fcinfo: pg_sys::FunctionCallInfo,
    element_oid: PgOid,
    bound: Option<&BoundField>,
) -> ElementValue {
    let element = unsafe { pg_getarg_datum_raw(fcinfo, 0) };
    match bound.map(|b| &b.kind) {
        Some(FieldKind::Tokenized(analyzer)) => {
            let text = unsafe {
                String::from_datum(element, false)
                    .expect("tokenized bound field value should be text")
            };
            ElementValue::Text(inline_eval::analyze(analyzer, &text).into_iter().collect())
        }
        _ => {
            let value = unsafe {
                TantivyValue::try_from_datum(element, element_oid).expect("no value present")
            };
            ElementValue::Raw(value)
        }
    }
}

#[pg_extern(immutable, parallel_safe)]
pub unsafe fn query_input_support(arg: Internal) -> ReturnedNodePointer {
    let datum = match arg.unwrap() {
        Some(d) => d,
        None => return ReturnedNodePointer(None),
    };

    if let Some(node) = query_input_support_request_simplify(datum) {
        return node;
    }

    if let Some(node) = search_query_input_request_cost(datum) {
        return node;
    }

    ReturnedNodePointer(None)
}

fn query_input_support_request_simplify(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        request_simplify(
            arg.cast_mut_ptr::<pg_sys::Node>(),
            //
            // if either of these closures are called that represents a logic error
            // in `request_simplify`'s implementation
            //
            // in this case, we know that the rhs of the expression has a type of `SearchQueryInput`
            // so there's no need for additional rewriting
            //
            |_, _, _| {
                unreachable!(
                    "query_input_support_request_simplify should never be called for Const rewriting"
                )
            },
            |_, _, _| {
                unreachable!("query_input_support_request_simplify should never be called for rhs expression rewriting")
            },
        )
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    fn inner_query_input(
        planner_info: Internal, // <pg_sys::PlannerInfo>,
        args: Internal,         // <pg_sys::List>,
    ) -> Option<f64> {
        unsafe {
            let info = planner_info.unwrap()?.cast_mut_ptr::<pg_sys::PlannerInfo>();
            let args =
                PgList::<pg_sys::Node>::from_pg(args.unwrap()?.cast_mut_ptr::<pg_sys::List>());

            let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
            let rhs = args.get_ptr(1)?;

            match (*rhs).type_ {
                pg_sys::NodeTag::T_Const => {
                    let const_ = rhs.cast::<pg_sys::Const>();
                    let (heaprelid, _, _) = find_var_relation(var, info);
                    let indexrel = rel_get_bm25_index(heaprelid)?.1;
                    let search_query_input =
                        SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)?;
                    estimate_selectivity(&indexrel, search_query_input)
                }
                pg_sys::NodeTag::T_Param => Some(PARAMETERIZED_SELECTIVITY),
                _ => None,
            }
        }
    }

    assert!(operator_oid == anyelement_query_input_opoid());

    let mut selectivity = inner_query_input(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}

fn search_query_input_request_cost(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        let src = nodecast!(
            SupportRequestCost,
            T_SupportRequestCost,
            arg.cast_mut_ptr::<pg_sys::Node>()
        )?;
        // our `search_with_*` functions are *incredibly* expensive.  So much so that
        // we really don't ever want Postgres to prefer them.
        //
        // The higher the `per_tuple` cost is here, the better.
        //
        // it can cost a lot to startup the `@@@` operator outside of an IndexScan because
        // ultimately we have to hash all the resulting ctids in memory.  For lack of a better
        // value, we say it costs as much as the `GUCS.per_tuple_cost()`.  This is an arbitrary
        // number that we've documented as needing to be big.
        (*src).startup = per_tuple_cost();

        // similarly, use the same GUC here.  Postgres will then add this into its per-tuple
        // cost evaluations for whatever scan it's considering using for the `@@@` operator.
        // our IAM provides more intelligent costs for the IndexScan situation.
        (*src).per_tuple = per_tuple_cost();

        Some(ReturnedNodePointer(NonNull::new(src.cast())))
    }
}
