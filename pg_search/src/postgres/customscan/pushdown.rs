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

use crate::api::operator::{field_name_from_node, searchqueryinput_typoid};
use crate::api::tokenizers::type_is_alias;
use crate::api::{FieldName, HashMap, fieldname_typoid};
use crate::nodecast;
use crate::postgres::catalog::{lookup_procoid, lookup_typoid};
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::opexpr::{
    OpExpr, OperatorAccepts, PostgresOperatorOid, TantivyOperator, TantivyOperatorExt,
    initialize_equality_operator_lookup,
};
use crate::postgres::customscan::qual_inspect::{PlannerContext, Qual, contains_correlated_param};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::var::{VarContext, find_json_path, find_vars};
use crate::schema::SearchField;
use pgrx::pg_sys::NodeTag::T_Const;
use pgrx::{FromDatum, IntoDatum, PgList, direct_function_call, is_a, pg_guard, pg_sys};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct PushdownField {
    field_name: FieldName,
    search_field: Option<SearchField>,
    varno: pg_sys::Index,
}

impl PushdownField {
    /// Given a Postgres [`pg_sys::Var`] and a [`SearchIndexSchema`], try to create a [`PushdownField`].
    /// The purpose of this is to guard against the case where we mistakenly push down a field that's not indexed.
    ///
    /// Returns `Some(PushdownField)` if the field is found in the schema, `None` otherwise.
    /// If `None` is returned, a helpful warning is logged.
    pub unsafe fn try_new(
        root: *mut pg_sys::PlannerInfo,
        var: *mut pg_sys::Node,
        indexrel: &PgSearchRelation,
    ) -> Option<Self> {
        let schema = indexrel.schema().ok()?;
        let mut var = var;

        if let Some(expr) = nodecast!(CoerceViaIO, T_CoerceViaIO, var) {
            var = (*expr).arg.cast();
        }

        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, var) {
            if !type_is_alias((*relabel).resulttype) {
                var = (*relabel).arg.cast();
            }
        }

        let heaprel = indexrel
            .heap_relation()
            .expect("index should have a heap relation");
        if let Some(field_name) =
            field_name_from_node(VarContext::from_planner(root), &heaprel, indexrel, var)
        {
            let search_field = schema.search_field(field_name.root())?;
            // an indexed expression could have more than one var, but we only need to check the first one
            // since they all come from the same relation and we use `varno` to determine if the field is in the same rti
            let vars = find_vars(var);
            if vars.is_empty() {
                return None;
            }
            let varno = (*vars[0]).varno as pg_sys::Index;
            return Some(Self {
                field_name,
                varno,
                search_field: Some(search_field),
            });
        }

        None
    }

    /// Create a new [`PushdownField`] from an attribute name.
    ///
    /// This does not verify if field can be pushed down and is intended to be used for testing.
    pub fn new(attname: &str) -> Self {
        Self {
            field_name: attname.into(),
            varno: 0,
            search_field: None,
        }
    }

    pub fn attname(&self) -> FieldName {
        self.field_name.clone()
    }

    pub fn search_field(&self) -> SearchField {
        self.search_field
            .clone()
            .expect("pushdown field should have a search field")
    }

    pub fn varno(&self) -> pg_sys::Index {
        self.varno
    }
}

macro_rules! pushdown {
    ($attname:expr, $opexpr:expr, $operator:expr, $field:ident, $field_is_array:ident, $root:ident, $indexrel:ident) => {{
        make_opexpr($attname, $opexpr, $operator, $field, $field_is_array).map(|funcexpr| {
            if !is_complex(funcexpr.cast()) {
                Qual::PushdownExpr { funcexpr }
            } else {
                let context = PlannerContext::from_planner($root);
                Qual::Expr {
                    node: funcexpr.cast(),
                    expr_state: std::ptr::null_mut(),
                    expr_desc: deparse_expr(Some(&context), $indexrel, funcexpr.cast()),
                }
            }
        })
    }};
}

static JSONB_EXISTS_OPOID: OnceLock<pg_sys::Oid> = OnceLock::new();

/// Take a Postgres [`pg_sys::OpExpr`] pointer that is **not** of our `@@@` operator and try  to
/// convert it into one that is.
///
/// Returns `Some(Qual)` if we were able to convert it, `None` if not.
#[rustfmt::skip]
pub unsafe fn try_pushdown_inner(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: OpExpr,
    indexrel: &PgSearchRelation,
) -> Option<Qual> {
    let args = opexpr.args();
    let lhs = args.get_ptr(0)?;
    let rhs = args.get_ptr(1)?;

    // If the RHS contains correlated PARAM_EXEC nodes (parameters which depend on an outer
    // relation), we can't push it down because the parameters need runtime evaluation with
    // planstate. Return None to let the caller create a HeapExpr instead.
    //
    // Uncorrelated PARAM_EXEC nodes will result in Qual::Expr and Qual::PostgresExpr nodes, which
    // are evaluated in BeginCustomScan.
    if contains_correlated_param(root, rhs) {
        return None;
    }

    // JSONB ? operator: construct field path from LHS + RHS key
    if opexpr.opno() == *JSONB_EXISTS_OPOID.get_or_init(|| operator_oid("?(jsonb,text)")) {
        return try_pushdown_jsonb_exists(root, rti, lhs, rhs, indexrel);
    }

    // if <field> is an array, 'literal' = ANY(<field>) the value appears on the lhs
    // in all other pushdown scenarios, the value is on the rhs
    let (maybe_field, maybe_value, field_is_array) = if is_a(lhs, T_Const)
        && nodecast!(Var, T_Var, rhs)
            .is_some_and(|var| pg_sys::type_is_array(unsafe { (*var).vartype }))
    {
        (rhs, lhs, true)
    } else {
        (lhs, rhs, false)
    };
    let pushdown = PushdownField::try_new(root, maybe_field, indexrel)?;
    let search_field = pushdown.search_field();

    static EQUALITY_OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();
    match EQUALITY_OPERATOR_LOOKUP.get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::All) }).get(&opexpr.opno()) {
        Some(pgsearch_operator) => {
            // can't push down tokenized text
            if (search_field.is_text() || opexpr.is_text_binary()) && !search_field.is_keyword() {
                return None;
            }

            // tantivy doesn't support JSON range if JSON is not fast
            if search_field.is_json() && !search_field.is_fast() && (*pgsearch_operator).is_range() {
                return None;
            }

            // tantivy doesn't support JSON exists if JSON is not fast, and our `<>` pushdown uses exists
            if search_field.is_json() && !search_field.is_fast() && (*pgsearch_operator).is_neq() {
                return None;
            }

            // the `opexpr` is one we can pushdown
            if pushdown.varno() == rti {
                let pushed_down_qual = pushdown!(
                    &pushdown.attname(),
                    opexpr,
                    pgsearch_operator,
                    maybe_value,
                    field_is_array,
                    root,
                    indexrel
                )?;
                // and it's in this RTI, so we can use it directly
                Some(pushed_down_qual)
            } else {
                // it's not in this RTI, which means it's in some other table due to a join, so
                // we need to indicate an arbitrary external var
                Some(Qual::ExternalVar)
            }
        },
        None => {
            // TODO:  support other types of OpExprs
            None
        }
    }
}

/// Pushdown JSONB `?` (exists) operator to BM25 index.
///
/// Converts `data ? 'key'` to equivalent of `id @@@ paradedb.exists('data.key')`.
/// For nested paths like `data->'nested' ? 'key'`, produces `data.nested.key`.
///
/// Returns `None` if pushdown not possible (field not indexed, not JSON, or not fast).
unsafe fn try_pushdown_jsonb_exists(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
    indexrel: &PgSearchRelation,
) -> Option<Qual> {
    // Extract the key from RHS (must be a non-null text constant)
    let rhs_const = nodecast!(Const, T_Const, rhs).filter(|c| !(**c).constisnull)?;
    let key = String::from_datum((*rhs_const).constvalue, false)?;

    // Build field path: extract JSON path from LHS and append the key
    let mut path = find_json_path(&VarContext::from_planner(root), lhs);
    if path.is_empty() {
        return None;
    }
    path.push(key);

    // Verify the root field is an indexed JSON field with fast=true (required for exists)
    let field_name = FieldName::from(path.join("."));
    let search_field = indexrel.schema().ok()?.search_field(field_name.root())?;
    if !search_field.is_json() || !search_field.is_fast() {
        return None;
    }

    // Check if field belongs to this relation or is from a join
    let varno = (**find_vars(lhs).first()?).varno as pg_sys::Index;
    if varno != rti {
        return Some(Qual::ExternalVar);
    }

    Some(Qual::PushdownIsNotNull {
        field: PushdownField {
            field_name,
            varno,
            search_field: Some(search_field),
        },
    })
}

unsafe fn term_with_operator_procid() -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.term_with_operator(paradedb.fieldname, text, anyelement)".into_datum()],
        )
            .expect("the `paradedb.term_with_operator(paradedb.fieldname, text, anyelement)` function should exist")
}

unsafe fn terms_with_operator_procid() -> Option<pg_sys::Oid> {
    lookup_procoid(
        c"paradedb",
        c"terms_with_operator",
        &[
            lookup_typoid(c"paradedb", c"fieldname")?,
            pg_sys::TEXTOID,
            pg_sys::ANYELEMENTOID,
            pg_sys::BOOLOID,
        ],
    )
}

unsafe fn make_opexpr(
    field: &FieldName,
    orig_opexor: OpExpr,
    operator: &str,
    value: *mut pg_sys::Node,
    field_is_array: bool,
) -> Option<*mut pg_sys::FuncExpr> {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    // if the field is an array, we actually want to do a term query, not a term set query
    // term set queries are for queries where the value is an array, not if the field is an array
    (*paradedb_funcexpr).funcid = if matches!(orig_opexor, OpExpr::Array(_)) && !field_is_array {
        terms_with_operator_procid()?
    } else {
        term_with_operator_procid()
    };
    (*paradedb_funcexpr).funcresulttype = searchqueryinput_typoid();
    (*paradedb_funcexpr).funcretset = false;
    (*paradedb_funcexpr).funcvariadic = false;
    (*paradedb_funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*paradedb_funcexpr).funccollid = pg_sys::InvalidOid;
    (*paradedb_funcexpr).inputcollid = orig_opexor.inputcollid();
    (*paradedb_funcexpr).location = orig_opexor.location();
    (*paradedb_funcexpr).args = {
        let fieldname = pg_sys::makeConst(
            fieldname_typoid(),
            -1,
            pg_sys::InvalidOid,
            -1,
            field.clone().into_datum().unwrap(),
            false,
            false,
        );
        let operator = pg_sys::makeConst(
            pg_sys::TEXTOID,
            -1,
            pg_sys::DEFAULT_COLLATION_OID,
            -1,
            operator.into_datum().unwrap(),
            false,
            false,
        );

        let mut args = PgList::<pg_sys::Node>::new();
        args.push(fieldname.cast());
        args.push(operator.cast());
        args.push(value.cast());

        if matches!(orig_opexor, OpExpr::Array(_)) {
            let conjunction_mode = !orig_opexor.use_or().unwrap(); // invert meaning for `conjunction` (which would be AND)
            let use_or = pg_sys::makeBoolConst(conjunction_mode, false);
            args.push(use_or.cast());
        }

        args.into_pg()
    };

    Some(paradedb_funcexpr)
}

pub unsafe fn is_complex(root: *mut pg_sys::Node) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        _data: *mut core::ffi::c_void,
    ) -> bool {
        nodecast!(Var, T_Var, node).is_some()
            || nodecast!(Param, T_Param, node).is_some()
            || pg_sys::contain_volatile_functions(node)
            || pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    if root.is_null() {
        return false;
    }

    walker(root, std::ptr::null_mut())
}
