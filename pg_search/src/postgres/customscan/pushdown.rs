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
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::opexpr::OpExpr;
use crate::postgres::customscan::qual_inspect::Qual;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct PushdownField {
    field_name: FieldName,
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
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        let (var, field) = find_one_var_and_fieldname(VarContext::from_planner(root), var)?;
        schema.search_field(&field).map(|_| Self {
            field_name: field,
            varno: (*var).varno as pg_sys::Index,
        })
    }

    /// Create a new [`PushdownField`] from an attribute name.
    ///
    /// This does not verify if field can be pushed down and is intended to be used for testing.
    pub fn new(field_name: &str) -> Self {
        Self {
            field_name: field_name.into(),
            varno: Default::default(),
        }
    }

    pub fn field_name(&self) -> FieldName {
        self.field_name.clone()
    }

    pub fn search_field(&self, schema: &SearchIndexSchema) -> Option<SearchField> {
        schema.search_field(self.field_name.root())
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

type PostgresOperatorOid = pg_sys::Oid;
type TantivyOperator = &'static str;

unsafe fn initialize_equality_operator_lookup() -> HashMap<PostgresOperatorOid, TantivyOperator> {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];
    const TYPE_PAIRS: &[[&str; 2]] = &[
        // integers
        ["int2", "int2"],
        ["int4", "int4"],
        ["int8", "int8"],
        ["int2", "int4"],
        ["int2", "int8"],
        ["int4", "int8"],
        // floats
        ["float4", "float4"],
        ["float8", "float8"],
        ["float4", "float8"],
        // dates
        ["date", "date"],
        ["time", "time"],
        ["timetz", "timetz"],
        ["timestamp", "timestamp"],
        ["timestamptz", "timestamptz"],
        // text
        ["text", "text"],
        ["uuid", "uuid"],
    ];

    let mut lookup = HashMap::default();

    // tantivy doesn't support range operators on bools, so we can only support the equality operator
    lookup.insert(operator_oid("=(bool,bool)"), "=");

    for o in OPERATORS {
        for [l, r] in TYPE_PAIRS {
            lookup.insert(operator_oid(&format!("{o}({l},{r})")), o);
            if l != r {
                // types can be reversed too
                lookup.insert(operator_oid(&format!("{o}({r},{l})")), o);
            }
        }
    }

    lookup
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
    schema: &SearchIndexSchema
) -> Option<Qual> {
    let args = opexpr.args();
    let lhs = args.get_ptr(0)?;
    let rhs = args.get_ptr(1)?;
    let pushdown = PushdownField::try_new(root, lhs, schema)?;
    let field = pushdown.search_field(schema)?;
    if field.is_text() && !field.is_keyword() {
        return None;
    }

    static EQUALITY_OPERATOR_LOOKUP: OnceLock<HashMap<pg_sys::Oid, &str>> = OnceLock::new();
    match EQUALITY_OPERATOR_LOOKUP.get_or_init(|| initialize_equality_operator_lookup()).get(&opexpr.opno()) {
        Some(pgsearch_operator) => {
            // the `opexpr` is one we can pushdown
            if pushdown.varno() == rti {
                let pushed_down_qual = pushdown!(&pushdown.field_name(), opexpr, pgsearch_operator, rhs);
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
    unsafe extern "C-unwind" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
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
