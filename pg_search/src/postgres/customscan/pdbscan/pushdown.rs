// Copyright (c) 2023-2025 Retake, Inc.
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

use crate::api::index::{fieldname_typoid, FieldName};
use crate::api::operator::{attname_from_var, parse_with_field_procoid, searchqueryinput_typoid};
use crate::nodecast;
use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use crate::schema::{SearchFieldName, SearchIndexSchema};
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList};
use rustc_hash::FxHashMap;
use std::ffi::CString;
use std::sync::OnceLock;

macro_rules! pushdown {
    ($attname:expr, $opexpr:expr, $parse_operator:expr, $rhs:ident) => {
        let funcexpr = make_opexpr($attname, $opexpr, $parse_operator, $rhs);

        if !is_complex(funcexpr.cast()) {
            return Some(Qual::PushdownExpr { funcexpr });
        } else {
            return Some(Qual::Expr {
                node: funcexpr.cast(),
                expr_state: std::ptr::null_mut(),
            });
        }
    };
}

type PostgresOperatorOid = pg_sys::Oid;
type TantivyOperator = &'static str;

unsafe fn initialize_equality_operator_lookup() -> FxHashMap<PostgresOperatorOid, TantivyOperator> {
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
        // boolean
        ["boolean", "boolean"],
    ];

    let mut lookup = FxHashMap::default();
    for o in OPERATORS {
        for [l, r] in TYPE_PAIRS {
            lookup.insert(opoid(&format!("{o}({l},{r})")), o);
            if l != r {
                // types can be reversed too
                lookup.insert(opoid(&format!("{o}({r},{l})")), o);
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
pub unsafe fn try_pushdown(
    root: *mut pg_sys::PlannerInfo,
    opexpr: *mut pg_sys::OpExpr,
    schema: &SearchIndexSchema
) -> Option<Qual> {
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
    let rhs = args.get_ptr(1)?;

    let (typeoid, attname) = attname_from_var(root, var);
    let attname = attname?;

    let _search_field = schema.get_search_field(&SearchFieldName(attname.clone()))?;

    static EQUALITY_OPERATOR_LOOKUP: OnceLock<FxHashMap<pg_sys::Oid, &str>> = OnceLock::new();
    match EQUALITY_OPERATOR_LOOKUP.get_or_init(|| initialize_equality_operator_lookup()).get(&(*opexpr).opno) {
        Some(pgsearch_operator) => { pushdown!(&attname, opexpr, pgsearch_operator, rhs); },
        None => {
            // TODO:  support other types of OpExprs
            None
        }
    }
}

unsafe fn opoid(signature: &str) -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(
        pg_sys::regoperatorin,
        &[CString::new(signature).into_datum()],
    )
    .expect("should be able to lookup operator signature")
}

unsafe fn textanycat_procid() -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(pg_sys::regprocin, &[c"pg_catalog.textanycat".into_datum()])
        .expect("could not lookup the `pg_catalog.textanycat` function")
}

unsafe fn make_opexpr(
    attname: &str,
    orig_opexor: *mut pg_sys::OpExpr,
    parse_operator: &str,
    value: *mut pg_sys::Node,
) -> *mut pg_sys::FuncExpr {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*paradedb_funcexpr).funcid = parse_with_field_procoid();
    (*paradedb_funcexpr).funcresulttype = searchqueryinput_typoid();
    (*paradedb_funcexpr).funcretset = false;
    (*paradedb_funcexpr).funcvariadic = false;
    (*paradedb_funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*paradedb_funcexpr).funccollid = pg_sys::InvalidOid;
    (*paradedb_funcexpr).inputcollid = (*orig_opexor).inputcollid;
    (*paradedb_funcexpr).location = (*orig_opexor).location;
    (*paradedb_funcexpr).args = {
        // the caller gives us a `parse_operator` like "=" or ">", or "<=" and we want to concatenate
        // that (what will become) text literal with whatever the original user's `value` expression
        // was.  That happens through injecting a call to Postgres' `textanycat()` function, which
        // we build here
        let textconcat_func = pg_sys::makeFuncExpr(
            textanycat_procid(),
            pg_sys::TEXTOID,
            {
                let mut args = PgList::<pg_sys::Node>::new();
                args.push(make_string_const(parse_operator, (*orig_opexor).location).cast());
                args.push(value);
                args.into_pg()
            },
            pg_sys::DEFAULT_COLLATION_OID,
            pg_sys::DEFAULT_COLLATION_OID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
        );

        let mut args = PgList::<pg_sys::Node>::new();
        args.push(
            pg_sys::makeConst(
                fieldname_typoid(),
                -1,
                pg_sys::InvalidOid,
                -1,
                FieldName::from(attname).into_datum().unwrap(),
                false,
                false,
            )
            .cast(),
        );
        args.push(textconcat_func.cast());
        args.push(pg_sys::makeBoolConst(false, false)); // "lenient" argument
        args.push(pg_sys::makeBoolConst(false, false)); // "conjunction_mode" argument
        args.into_pg()
    };

    paradedb_funcexpr
}

fn is_complex(root: *mut pg_sys::Node) -> bool {
    unsafe extern "C" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
        nodecast!(Var, T_Var, node).is_some()
            || nodecast!(Param, T_Param, node).is_some()
            || pg_sys::contain_volatile_functions(node)
            || pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    if root.is_null() {
        return false;
    }

    unsafe {
        nodecast!(Var, T_Var, root).is_some()
            || nodecast!(Param, T_Param, root).is_some()
            || pg_sys::contain_volatile_functions(root)
            || pg_sys::expression_tree_walker(root, Some(walker), std::ptr::null_mut())
    }
}

unsafe fn make_string_const(str: &str, location: i32) -> *mut pg_sys::Const {
    let const_: *mut pg_sys::Const = pg_sys::palloc0(size_of::<pg_sys::Const>()).cast();

    (*const_).xpr.type_ = pg_sys::NodeTag::T_Const;
    (*const_).constvalue = str.into_datum().unwrap();
    (*const_).constisnull = false;
    (*const_).constcollid = pg_sys::DEFAULT_COLLATION_OID;
    (*const_).constbyval = false;
    (*const_).consttype = pg_sys::TEXTOID;
    (*const_).constlen = -1;
    (*const_).consttypmod = -1;
    (*const_).location = location;

    const_
}
