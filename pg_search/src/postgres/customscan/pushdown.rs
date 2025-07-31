// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::operator::searchqueryinput_typoid;
use crate::api::{fieldname_typoid, FieldName, HashMap};
use crate::nodecast;
use crate::postgres::customscan::opexpr::{
    initialize_equality_operator_lookup, OpExpr, OperatorAccepts, PostgresOperatorOid,
    TantivyOperator, TantivyOperatorExt,
};
use crate::postgres::customscan::qual_inspect::Qual;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::schema::SearchField;
use pgrx::{direct_function_call, pg_guard, pg_sys, IntoDatum, PgList};
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
        let (var, field_name) = find_one_var_and_fieldname(VarContext::from_planner(root), var)?;
        let schema = indexrel.schema().ok()?;
        let search_field = schema.search_field(field_name.root())?;
        Some(Self {
            field_name,
            varno: (*var).varno as pg_sys::Index,
            search_field: Some(search_field),
        })
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
    ($attname:expr, $opexpr:expr, $operator:expr, $rhs:ident) => {{
        let funcexpr = make_opexpr($attname, $opexpr, $operator, $rhs);

        if !is_complex(funcexpr.cast()) {
            Qual::PushdownExpr { funcexpr }
        } else {
            Qual::Expr {
                node: funcexpr.cast(),
                expr_state: std::ptr::null_mut(),
            }
        }
    }};
}

/// Take a Postgres [`pg_sys::OpExpr`] pointer that is **not** of our `@@@` operator and try  to
/// convert it into one that is.
///
/// Returns `Some(Qual)` if we were able to convert it, `None` if not.
#[rustfmt::skip]
pub unsafe fn try_pushdown_inner(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: OpExpr,
    indexrel: &PgSearchRelation
) -> Option<Qual> {
    let args = opexpr.args();
    let lhs = args.get_ptr(0)?;
    let rhs = args.get_ptr(1)?;
    let pushdown = PushdownField::try_new(root, lhs, indexrel)?;
    let search_field = pushdown.search_field();

    static EQUALITY_OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();
    match EQUALITY_OPERATOR_LOOKUP.get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::All) }).get(&opexpr.opno()) {
        Some(pgsearch_operator) => {
            // can't push down tokenized text
            if opexpr.is_text() && !search_field.is_keyword() {
                return None;
            }

            // tantivy doesn't support JSON range if JSON is not fast
            if search_field.is_json() && !search_field.is_fast() && (*pgsearch_operator).is_range() {
                return None;
            }

            // tantivy doesn't support JSON exists if JSON is not fast, and our `<>` pushdown uses exists
            if search_field.is_json() && (*pgsearch_operator).is_neq() {
                return None;
            }

            // the `opexpr` is one we can pushdown
            if pushdown.varno() == rti {
                let pushed_down_qual = pushdown!(&pushdown.attname(), opexpr, pgsearch_operator, rhs);
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

unsafe fn term_with_operator_procid() -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.term_with_operator(paradedb.fieldname, text, anyelement)".into_datum()],
        )
            .expect("the `paradedb.term_with_operator(paradedb.fieldname, text, anyelement)` function should exist")
}

unsafe fn terms_with_operator_procid() -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.terms_with_operator(paradedb.fieldname, text, anyelement, bool)".into_datum()],
        )
            .expect("the `paradedb.terms_with_operator(paradedb.fieldname, text, anyelement, bool)` function should exist")
}

unsafe fn make_opexpr(
    field: &FieldName,
    orig_opexor: OpExpr,
    operator: &str,
    value: *mut pg_sys::Node,
) -> *mut pg_sys::FuncExpr {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*paradedb_funcexpr).funcid = match orig_opexor {
        OpExpr::Array(_) => terms_with_operator_procid(),
        OpExpr::Single(_) => term_with_operator_procid(),
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

    paradedb_funcexpr
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
