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

use pgrx::{pg_sys, IntoDatum};
use std::ffi::CStr;

/// Helper function to lookup a namespace's [`pg_sys::Oid`] (SQL schema) by name
pub fn lookup_namespace(namespace: &CStr) -> Option<pg_sys::Oid> {
    unsafe {
        let namespace_oid = pg_sys::get_namespace_oid(namespace.as_ptr(), true);
        (namespace_oid != pg_sys::InvalidOid).then_some(namespace_oid)
    }
}

/// Helper function to lookup a type's [`pg_sys::Oid`] by name and namespace
pub fn lookup_typoid(namespace: &CStr, typename: &CStr) -> Option<pg_sys::Oid> {
    unsafe {
        let typoid = pg_sys::GetSysCacheOid(
            pg_sys::SysCacheIdentifier::TYPENAMENSP as _,
            pg_sys::Anum_pg_type_oid as _,
            pg_sys::Datum::from(typename.as_ptr()),
            lookup_namespace(namespace).into_datum()?,
            pg_sys::Datum::null(),
            pg_sys::Datum::null(),
        );
        (typoid != pg_sys::InvalidOid).then_some(typoid)
    }
}

/// Helper function to lookup a function's [`pg_sys::Oid`] by name, argument types, and namespace
pub fn lookup_procoid(
    namespace: &CStr,
    fucname: &CStr,
    argtypes: &[pg_sys::Oid],
) -> Option<pg_sys::Oid> {
    unsafe {
        let argvec = pg_sys::buildoidvector(argtypes.as_ptr(), argtypes.len() as _);
        let procoid = pg_sys::GetSysCacheOid(
            pg_sys::SysCacheIdentifier::PROCNAMEARGSNSP as _,
            pg_sys::Anum_pg_proc_oid as _,
            pg_sys::Datum::from(fucname.as_ptr()),
            pg_sys::Datum::from(argvec),
            lookup_namespace(namespace).into_datum()?,
            pg_sys::Datum::null(),
        );
        pg_sys::pfree(argvec.cast());
        (procoid != pg_sys::InvalidOid).then_some(procoid)
    }
}
