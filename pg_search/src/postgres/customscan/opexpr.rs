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

use crate::nodecast;
use crate::postgres::customscan::operator_oid;
use pgrx::{pg_sys, PgList};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug)]
pub(crate) enum OpExpr {
    Array(*mut pg_sys::ScalarArrayOpExpr),
    Single(*mut pg_sys::OpExpr),
}

impl OpExpr {
    pub unsafe fn from_array(node: *mut pg_sys::Node) -> Option<Self> {
        nodecast!(ScalarArrayOpExpr, T_ScalarArrayOpExpr, node).map(OpExpr::Array)
    }

    pub unsafe fn from_single(node: *mut pg_sys::Node) -> Option<Self> {
        nodecast!(OpExpr, T_OpExpr, node).map(OpExpr::Single)
    }

    pub unsafe fn args(&self) -> PgList<pg_sys::Node> {
        match self {
            OpExpr::Array(expr) => PgList::<pg_sys::Node>::from_pg((*(*expr)).args),
            OpExpr::Single(expr) => PgList::<pg_sys::Node>::from_pg((*(*expr)).args),
        }
    }

    pub unsafe fn use_or(&self) -> Option<bool> {
        match self {
            OpExpr::Array(expr) => Some((*(*expr)).useOr),
            OpExpr::Single(_) => None,
        }
    }

    pub unsafe fn opno(&self) -> pg_sys::Oid {
        match self {
            OpExpr::Array(expr) => (*(*expr)).opno,
            OpExpr::Single(expr) => (*(*expr)).opno,
        }
    }

    pub unsafe fn inputcollid(&self) -> pg_sys::Oid {
        match self {
            OpExpr::Array(expr) => (*(*expr)).inputcollid,
            OpExpr::Single(expr) => (*(*expr)).inputcollid,
        }
    }

    pub unsafe fn location(&self) -> pg_sys::int32 {
        match self {
            OpExpr::Array(expr) => (*(*expr)).location,
            OpExpr::Single(expr) => (*(*expr)).location,
        }
    }

    pub fn is_text(&self) -> bool {
        static TEXT_OPERATOR_LOOKUP: OnceLock<HashMap<pg_sys::Oid, bool>> = OnceLock::new();
        let opno = unsafe { self.opno() };

        TEXT_OPERATOR_LOOKUP
            .get_or_init(|| unsafe { initialize_text_operator_lookup() })
            .contains_key(&opno)
    }
}

unsafe fn initialize_text_operator_lookup() -> HashMap<pg_sys::Oid, bool> {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];
    const TYPES: [&str; 2] = ["text", "uuid"];

    let mut lookup = HashMap::default();
    for op in OPERATORS {
        for typ in TYPES {
            lookup.insert(operator_oid(&format!("{op}({typ},{typ})")), true);
        }
    }
    lookup
}
