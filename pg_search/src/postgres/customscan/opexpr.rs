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

use crate::api::HashMap;
use crate::nodecast;
use crate::postgres::customscan::operator_oid;
use pgrx::{pg_sys, PgList};
use std::sync::OnceLock;

pub type PostgresOperatorOid = pg_sys::Oid;
pub type TantivyOperator = &'static str;

pub trait TantivyOperatorExt {
    fn is_range(&self) -> bool;
    #[allow(unused)]
    fn is_eq(&self) -> bool;
    fn is_neq(&self) -> bool;
}

impl TantivyOperatorExt for TantivyOperator {
    fn is_range(&self) -> bool {
        *self == ">" || *self == ">=" || *self == "<" || *self == "<="
    }

    fn is_eq(&self) -> bool {
        *self == "="
    }

    fn is_neq(&self) -> bool {
        *self == "<>"
    }
}

pub const TEXT_TYPE_PAIRS: &[[&str; 2]] = &[["text", "text"], ["uuid", "uuid"]];
pub const NUMERIC_TYPE_PAIRS: &[[&str; 2]] = &[
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
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorAccepts {
    // text, uuid
    Text,
    // text, uuid, int, bool, etc.
    All,
}

pub unsafe fn initialize_equality_operator_lookup(
    accepts: OperatorAccepts,
) -> HashMap<PostgresOperatorOid, TantivyOperator> {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];
    let mut lookup = HashMap::default();

    if accepts == OperatorAccepts::All {
        // tantivy doesn't support range operators on bools, so we can only support the equality operator
        lookup.insert(operator_oid("=(bool,bool)"), "=");
    }

    let type_pairs = match accepts {
        OperatorAccepts::Text => TEXT_TYPE_PAIRS,
        OperatorAccepts::All => &[NUMERIC_TYPE_PAIRS, TEXT_TYPE_PAIRS].concat(),
    };

    for o in OPERATORS {
        for [l, r] in type_pairs {
            lookup.insert(operator_oid(&format!("{o}({l},{r})")), o);
            if l != r {
                // types can be reversed too
                lookup.insert(operator_oid(&format!("{o}({r},{l})")), o);
            }
        }
    }

    lookup
}

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

    pub fn is_text_binary(&self) -> bool {
        static TEXT_OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> =
            OnceLock::new();
        let opno = unsafe { self.opno() };

        TEXT_OPERATOR_LOOKUP
            .get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::Text) })
            .get(&opno)
            .is_some()
    }
}
