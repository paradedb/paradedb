// Copyright (c) 2023-2024 Retake, Inc.
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

use pgrx::{is_a, pg_sys};
use std::ffi::{c_void, CStr};

pub mod config;
pub mod index;
pub mod operator;
pub mod search;
pub mod tokenize;

#[track_caller]
#[inline(always)]
pub unsafe fn node<T>(void: *mut c_void, tag: pg_sys::NodeTag) -> Option<*mut T> {
    let node: *mut T = void.cast();
    if !is_a(node.cast(), tag) {
        return None;
    }
    Some(node)
}

#[macro_export]
macro_rules! nodecast {
    ($type_:ident, $kind:ident, $node:expr) => {
        $crate::api::node::<pg_sys::$type_>($node.cast(), pg_sys::NodeTag::$kind)
    };
}

pub trait AsInt {
    unsafe fn as_int(&self) -> Option<i32>;
}

pub trait AsCStr {
    unsafe fn as_c_str(&self) -> Option<&CStr>;
}

#[cfg(any(feature = "pg13", feature = "pg14"))]
impl AsInt for *mut pg_sys::Node {
    unsafe fn as_int(&self) -> Option<i32> {
        let node = nodecast!(Value, T_Integer, *self)?;
        Some((*node).val.ival)
    }
}

#[cfg(not(any(feature = "pg13", feature = "pg14")))]
impl AsInt for *mut pg_sys::Node {
    unsafe fn as_int(&self) -> Option<i32> {
        let node = nodecast!(Integer, T_Integer, *self)?;
        Some((*node).ival)
    }
}

#[cfg(any(feature = "pg13", feature = "pg14"))]
impl AsCStr for *mut pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&CStr> {
        let node = nodecast!(Value, T_String, *self)?;
        Some(CStr::from_ptr((*node).val.str_))
    }
}

#[cfg(not(any(feature = "pg13", feature = "pg14")))]
impl AsCStr for *mut pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&CStr> {
        let node = nodecast!(String, T_String, *self)?;
        Some(CStr::from_ptr((*node).sval))
    }
}
