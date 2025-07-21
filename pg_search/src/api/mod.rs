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

// TODO: See https://github.com/pgcentralfoundation/pgrx/pull/2089
#![allow(for_loops_over_fallibles)]

pub mod aggregate;
pub mod builder_fns;
pub mod config;
pub mod operator;
pub mod tokenize;

use pgrx::{
    direct_function_call, pg_cast, pg_sys, InOutFuncs, IntoDatum, PostgresType, StringInfo,
};
pub use rustc_hash::FxHashMap as HashMap;
pub use rustc_hash::FxHashSet as HashSet;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fmt::{Debug, Display, Formatter};
use tantivy::json_utils::split_json_path;

#[macro_export]
macro_rules! nodecast {
    ($type_:ident, $kind:ident, $node:expr) => {{
        let node = $node;
        pgrx::is_a(node.cast(), pgrx::pg_sys::NodeTag::$kind)
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};

    ($type_:ident, $kind:ident, $node:expr, true) => {{
        let node = $node;
        (node.is_null() || pgrx::is_a(node.cast(), pgrx::pg_sys::NodeTag::$kind))
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};
}

// came to life in pg15
pub type Cardinality = f64;

#[cfg(feature = "pg14")]
pub type Varno = pgrx::pg_sys::Index;
#[cfg(not(feature = "pg14"))]
pub type Varno = i32;

#[allow(dead_code)]
pub trait AsBool {
    unsafe fn as_bool(&self) -> Option<bool>;
}

pub trait AsCStr {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr>;
}

#[cfg(feature = "pg14")]
impl AsBool for *mut pgrx::pg_sys::Node {
    unsafe fn as_bool(&self) -> Option<bool> {
        let node = nodecast!(Value, T_Integer, *self)?;
        Some((*node).val.ival != 0)
    }
}

#[cfg(not(feature = "pg14"))]
impl AsBool for *mut pgrx::pg_sys::Node {
    unsafe fn as_bool(&self) -> Option<bool> {
        let node = nodecast!(Boolean, T_Boolean, *self)?;
        Some((*node).boolval)
    }
}

#[cfg(feature = "pg14")]
impl AsCStr for *mut pgrx::pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr> {
        let node = nodecast!(Value, T_String, *self)?;
        Some(std::ffi::CStr::from_ptr((*node).val.str_))
    }
}

#[cfg(not(feature = "pg14"))]
impl AsCStr for *mut pgrx::pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr> {
        let node = nodecast!(String, T_String, *self)?;
        Some(std::ffi::CStr::from_ptr((*node).sval))
    }
}

/// A type used whenever our builder functions require a fieldname.
#[derive(
    Debug, Clone, Ord, Eq, PartialOrd, PartialEq, Hash, Serialize, Deserialize, PostgresType,
)]
#[inoutfuncs]
pub struct FieldName(String);

impl Display for FieldName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for FieldName {
    fn as_ref(&self) -> &str {
        self
    }
}

impl std::ops::Deref for FieldName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for FieldName
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        FieldName(value.into())
    }
}

impl InOutFuncs for FieldName {
    fn input(input: &CStr) -> Self
    where
        Self: Sized,
    {
        FieldName(input.to_str().unwrap().to_owned())
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&self.0);
    }
}

impl From<FieldName> for *mut pg_sys::Const {
    fn from(value: FieldName) -> Self {
        unsafe {
            pg_sys::makeConst(
                fieldname_typoid(),
                -1,
                pg_sys::Oid::INVALID,
                -1,
                value.into_datum().unwrap(),
                false,
                false,
            )
        }
    }
}

impl FieldName {
    pub fn null_const() -> *mut pg_sys::Const {
        unsafe {
            pg_sys::makeConst(
                fieldname_typoid(),
                -1,
                pg_sys::Oid::INVALID,
                -1,
                pg_sys::Datum::null(),
                true,
                false,
            )
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn root(&self) -> String {
        let json_path = split_json_path(self.0.as_str());
        if json_path.len() == 1 {
            self.0.clone()
        } else {
            json_path[0].clone()
        }
    }

    pub fn path(&self) -> Option<String> {
        let json_path = split_json_path(self.0.as_str());
        if json_path.len() == 1 {
            None
        } else {
            Some(json_path[1..].join("."))
        }
    }

    pub fn is_ctid(&self) -> bool {
        self.root() == "ctid"
    }
}

#[pg_cast(implicit)]
fn text_to_fieldname(field: String) -> FieldName {
    FieldName(field)
}

#[allow(unused)]
pub fn fieldname_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"paradedb.FieldName".into_datum()],
        )
        .expect("type `paradedb.FieldName` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `paradedb.FieldName` should exist");
        }
        oid
    }
}
