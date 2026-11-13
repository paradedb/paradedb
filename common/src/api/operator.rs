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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use crate::nodecast;
use crate::postgres::node::NodeExt;
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
};
use pgrx::*;
use std::ptr::NonNull;
use std::sync::OnceLock;

#[derive(Debug)]
#[repr(transparent)]
pub struct ReturnedNodePointer(pub Option<NonNull<pg_sys::Node>>);

impl ReturnedNodePointer {
    pub fn unsupported() -> Self {
        Self(None)
    }

    pub fn from_node(node: *mut pg_sys::Node) -> Self {
        Self(NonNull::new(node))
    }
}

unsafe impl BoxRet for ReturnedNodePointer {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
        self.0
            .map(|nonnull| {
                let datum = pg_sys::Datum::from(nonnull.as_ptr());
                fcinfo.return_raw_datum(datum)
            })
            .unwrap_or_else(Datum::null)
    }
}

unsafe impl SqlTranslatable for ReturnedNodePointer {
    const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(ReturnedNodePointer);
    const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        Ok(SqlMappingRef::literal("internal"));
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> =
        Ok(ReturnsRef::One(SqlMappingRef::literal("internal")));
}

pub fn with_index_procoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.with_index(regclass, paradedb.searchqueryinput)".into_datum()],
        )
        .expect(
            "the `paradedb.with_index(regclass, paradedb.searchqueryinput)` function should exist",
        )
    }
}

pub fn anyelement_query_input_procoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.search_with_query_input(anyelement, paradedb.searchqueryinput)".into_datum()],
        )
            .expect("the `paradedb.search_with_query_input(anyelement, paradedb.searchqueryinput) function should exist")
    }
}

pub fn search_with_query_input_exec_procoids() -> [pg_sys::Oid; 4] {
    static CACHE: OnceLock<[pg_sys::Oid; 4]> = OnceLock::new();
    *CACHE.get_or_init(|| unsafe {
        [
            c"paradedb.search_with_query_input_ctid(anyelement, paradedb.searchqueryinput, tid, record)",
            c"paradedb.search_with_query_input_ctid_strict(anyelement, paradedb.searchqueryinput, tid, record)",
            c"paradedb.search_with_query_input_ctid_or_row(anyelement, paradedb.searchqueryinput, tid, record[], record)",
            c"paradedb.search_with_query_input_ctid_or_row_strict(anyelement, paradedb.searchqueryinput, tid, record[], record)",
        ].map(|signature| {
            direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[signature.into_datum()])
                .unwrap_or_else(|| panic!("`{}` should exist", signature.to_str().unwrap()))
        })
    })
}

pub fn is_search_with_query_input_exec_procoid(procoid: pg_sys::Oid) -> bool {
    search_with_query_input_exec_procoids().contains(&procoid)
}

/// The search operands shared by the scalar operator and its heap-filter forms.
/// CTID and fallback-row arguments are execution details, so custom-scan planning only sees
/// `lhs` and `rhs`.
#[derive(Clone, Copy)]
pub struct SearchPredicate {
    pub lhs: *mut pg_sys::Node,
    pub rhs: *mut pg_sys::Node,
    pub location: pg_sys::int32,
}

impl SearchPredicate {
    pub unsafe fn from_node(node: *mut pg_sys::Node) -> Option<Self> {
        if let Some(operator) = nodecast!(OpExpr, node)
            && (*operator).opno == anyelement_query_input_opoid()
        {
            let args = PgList::<pg_sys::Node>::from_pg((*operator).args);
            return Some(Self {
                lhs: args.get_ptr(0)?,
                rhs: args.get_ptr(1)?,
                location: (*operator).location,
            });
        }

        let function = nodecast!(FuncExpr, node)?;
        if !is_search_with_query_input_exec_procoid((*function).funcid) {
            return None;
        }

        let args = PgList::<pg_sys::Node>::from_pg((*function).args);
        Some(Self {
            lhs: args.get_ptr(0)?,
            rhs: args.get_ptr(1)?,
            location: (*function).location,
        })
    }

    pub unsafe fn into_opexpr(self) -> *mut pg_sys::OpExpr {
        let operator = pg_sys::make_opclause(
            anyelement_query_input_opoid(),
            pg_sys::BOOLOID,
            false,
            self.lhs.cast(),
            self.rhs.cast(),
            pg_sys::Oid::INVALID,
            pg_sys::DEFAULT_COLLATION_OID,
        )
        .cast::<pg_sys::OpExpr>();
        (*operator).opfuncid = anyelement_query_input_procoid();
        (*operator).location = self.location;
        operator
    }
}

pub unsafe fn expr_contains_search_predicate(node: *mut pg_sys::Node) -> bool {
    node.any(|node| SearchPredicate::from_node(node).is_some())
}

pub fn anyelement_query_input_opoid() -> pg_sys::Oid {
    anyelement_search_opoids()[0]
}

pub fn anyelement_search_opoids() -> [pg_sys::Oid; 2] {
    static CACHE: OnceLock<[pg_sys::Oid; 2]> = OnceLock::new();
    *CACHE.get_or_init(|| unsafe {
        [
            direct_function_call::<pg_sys::Oid>(
                pg_sys::regoperatorin,
                &[c"@@@(anyelement, paradedb.searchqueryinput)".into_datum()],
            )
            .expect("the `@@@(anyelement, paradedb.searchqueryinput)` operator should exist"),
            direct_function_call::<pg_sys::Oid>(
                pg_sys::regoperatorin,
                &[c"@@@(anyelement, pdb.query)".into_datum()],
            )
            .expect("the `@@@(anyelement, pdb.query)` operator should exist"),
        ]
    })
}

pub fn is_anyelement_search_opoid(opno: pg_sys::Oid) -> bool {
    if anyelement_search_opoids().contains(&opno) {
        return true;
    }
    is_paradedb_search_operator(opno)
}

pub fn is_paradedb_search_operator(opno: pg_sys::Oid) -> bool {
    let opertup = unsafe {
        pg_sys::SearchSysCache1(
            pg_sys::SysCacheIdentifier::OPEROID as _,
            opno.into_datum().unwrap(),
        )
    };

    if opertup.is_null() {
        return false;
    }

    let operform = unsafe {
        &*pgrx::GETSTRUCT!(opertup)
            .get()
            .cast::<pg_sys::FormData_pg_operator>()
    };
    let opername = pgrx::name_data_to_str(&operform.oprname);
    let oprleft = operform.oprleft;

    let is_ours = match opername {
        "@@@" | "|||" | "&&&" | "===" | "###" => oprleft == pg_sys::ANYELEMENTOID,
        "##" | "##>" => {
            oprleft != pg_sys::POINTOID
                && oprleft != pg_sys::LINEOID
                && oprleft != pg_sys::LSEGOID
                && oprleft != pg_sys::BOXOID
        }
        _ => false,
    };

    unsafe { pg_sys::ReleaseSysCache(opertup) };
    is_ours
}

pub fn operator_name_from_oid(opno: pg_sys::Oid) -> String {
    unsafe {
        let opertup = pg_sys::SearchSysCache1(
            pg_sys::SysCacheIdentifier::OPEROID as _,
            opno.into_datum().unwrap(),
        );
        if opertup.is_null() {
            return "unknown".to_string();
        }
        let operform = pgrx::GETSTRUCT!(opertup)
            .get()
            .cast::<pg_sys::FormData_pg_operator>();
        let name = pgrx::name_data_to_str(&(*operform).oprname).to_string();
        pg_sys::ReleaseSysCache(opertup);
        name
    }
}

pub fn searchqueryinput_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"paradedb.SearchQueryInput".into_datum()],
        )
        .expect("type `paradedb.SearchQueryInput` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `paradedb.SearchQueryInput` should exist");
        }
        oid
    }
}

pub fn pdb_query_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.Query".into_datum()])
                .expect("type `pdb.Query` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.Query` should exist");
        }
        oid
    }
}

pub fn boost_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.boost".into_datum()])
                .expect("type `pdb.boost` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.boost` should exist");
        }
        oid
    }
}

pub fn const_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.const".into_datum()])
                .expect("type `pdb.const` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.const` should exist");
        }
        oid
    }
}

pub fn fuzzy_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.fuzzy".into_datum()])
                .expect("type `pdb.fuzzy` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.fuzzy` should exist");
        }
        oid
    }
}

pub fn slop_typoid() -> pg_sys::Oid {
    unsafe {
        let oid =
            direct_function_call::<pg_sys::Oid>(pg_sys::regtypein, &[c"pdb.slop".into_datum()])
                .expect("type `pdb.slop` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.slop` should exist");
        }
        oid
    }
}

pub fn pdb_proximityclause_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"pdb.ProximityClause".into_datum()],
        )
        .expect("type `pdb.ProximityClause` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `pdb.ProximityClause` should exist");
        }
        oid
    }
}

pub unsafe fn row_expr_from_indexed_expr(
    mut expr: *mut pg_sys::Expr,
) -> Option<*mut pg_sys::RowExpr> {
    loop {
        if let Some(row_expr) = nodecast!(RowExpr, expr) {
            return Some(row_expr);
        }
        if let Some(coerce) = nodecast!(CoerceViaIO, expr) {
            expr = (*coerce).arg.cast();
            continue;
        }
        if let Some(relabel) = nodecast!(RelabelType, expr) {
            expr = (*relabel).arg.cast();
            continue;
        }
        return None;
    }
}
