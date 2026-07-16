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

use pgrx::{pg_sys, FromDatum, IntoDatum};
use std::ffi::CStr;
use std::sync::OnceLock;

/// Pins a syscache entry for the lifetime of the guard; the pin is
/// released on drop, on every exit path including panics.
/// Note: holds a raw pointer and is therefore !Send — required, since
/// the catcache refcount is a plain non-atomic decrement on
/// backend-private memory.
struct SysCacheEntry {
    cache: pg_sys::SysCacheIdentifier::Type,
    tuple: pg_sys::HeapTuple,
}

impl SysCacheEntry {
    unsafe fn search1(cache: pg_sys::SysCacheIdentifier::Type, key: pg_sys::Datum) -> Option<Self> {
        let tuple = pg_sys::SearchSysCache1(cache as _, key);
        (!tuple.is_null()).then_some(Self { cache, tuple })
    }

    /// Note that borrowed values MUST be copied out before the guard drops
    /// Ex: attr::<&CStr>() hands back a borrow into the pinned tuple, but the lifetime isn't tied to the guard, so Rust's compiler won't catch use after free
    unsafe fn attr<T: FromDatum>(&self, attno: u32) -> Option<T> {
        let mut is_null = false;
        let datum = pg_sys::SysCacheGetAttr(self.cache as _, self.tuple, attno as _, &mut is_null);
        T::from_datum(datum, is_null)
    }
}

impl Drop for SysCacheEntry {
    fn drop(&mut self) {
        unsafe {
            pg_sys::ReleaseSysCache(self.tuple);
        }
    }
}

/// Helper function to lookup a namespace's [`pg_sys::Oid`] (SQL schema) by name
pub fn lookup_namespace(namespace: &CStr) -> Option<pg_sys::Oid> {
    unsafe {
        let namespace_oid = pg_sys::get_namespace_oid(namespace.as_ptr(), true);
        (namespace_oid != pg_sys::InvalidOid).then_some(namespace_oid)
    }
}

/// Helper function to lookup a type's assigned `typcategory` attribute from `pg_catalog.pg_type`
pub fn lookup_type_category(typoid: pg_sys::Oid) -> Option<u8> {
    unsafe {
        let entry =
            SysCacheEntry::search1(pg_sys::SysCacheIdentifier::TYPEOID, typoid.into_datum()?)?;
        let category = entry.attr::<i8>(pg_sys::Anum_pg_type_typcategory)?;
        Some(category as u8)
    }
}

/// Helper function to lookup a type's assigned `typcategory` attribute from `pg_catalog.pg_type`
pub fn lookup_type_name(typoid: pg_sys::Oid) -> Option<String> {
    unsafe {
        let entry =
            SysCacheEntry::search1(pg_sys::SysCacheIdentifier::TYPEOID, typoid.into_datum()?)?;
        entry
            .attr::<&CStr>(pg_sys::Anum_pg_type_typname)
            .map(|s| s.to_string_lossy().to_string())
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

/// Helper function to lookup a function's name by its OID as a CStr
pub fn lookup_func_name_cstr<'a>(funcid: pg_sys::Oid) -> Option<&'a std::ffi::CStr> {
    unsafe {
        let name_ptr = pg_sys::get_func_name(funcid);
        if !name_ptr.is_null() {
            Some(std::ffi::CStr::from_ptr(name_ptr))
        } else {
            None
        }
    }
}

/// Helper function to lookup a function's name by its OID
pub fn lookup_func_name(funcid: pg_sys::Oid) -> Option<String> {
    lookup_func_name_cstr(funcid).and_then(|cstr| cstr.to_str().ok().map(|s| s.to_string()))
}

/// Helper function to lookup a namespace's name by its OID
pub fn lookup_namespace_name(namespace_oid: pg_sys::Oid) -> Option<String> {
    unsafe {
        let name_ptr = pg_sys::get_namespace_name(namespace_oid);
        if !name_ptr.is_null() {
            std::ffi::CStr::from_ptr(name_ptr)
                .to_str()
                .ok()
                .map(|s| s.to_string())
        } else {
            None
        }
    }
}

/// Helper function to lookup a function's fully qualified name (schema.name) by its OID
pub fn lookup_fully_qualified_func_name(funcid: pg_sys::Oid) -> Option<String> {
    let name = lookup_func_name(funcid)?;
    let namespace_oid = unsafe { pg_sys::get_func_namespace(funcid) };
    if namespace_oid != pg_sys::InvalidOid {
        if let Some(namespace) = lookup_namespace_name(namespace_oid) {
            if namespace != "pg_catalog" {
                return Some(format!("{}.{}", namespace, name));
            }
        }
    }
    Some(name)
}

/// Returns `true` if `oid` is the OID of the `citext` type.
pub fn is_citext_oid(oid: pg_sys::Oid) -> bool {
    static CITEXT_OID: OnceLock<pg_sys::Oid> = OnceLock::new();
    let cid = *CITEXT_OID
        .get_or_init(|| lookup_typoid(c"public", c"citext").unwrap_or(pg_sys::Oid::INVALID));
    cid != pg_sys::Oid::INVALID && oid == cid
}

/// Helper function to lookup a function's [`pg_sys::Oid`] by name, argument types, and namespace
pub fn lookup_procoid(
    namespace: &CStr,
    funcname: &CStr,
    argtypes: &[pg_sys::Oid],
) -> Option<pg_sys::Oid> {
    unsafe {
        let argvec = pg_sys::buildoidvector(argtypes.as_ptr(), argtypes.len() as _);
        let procoid = pg_sys::GetSysCacheOid(
            pg_sys::SysCacheIdentifier::PROCNAMEARGSNSP as _,
            pg_sys::Anum_pg_proc_oid as _,
            pg_sys::Datum::from(funcname.as_ptr()),
            pg_sys::Datum::from(argvec),
            lookup_namespace(namespace).into_datum()?,
            pg_sys::Datum::null(),
        );
        pg_sys::pfree(argvec.cast());
        (procoid != pg_sys::InvalidOid).then_some(procoid)
    }
}

/// Returns `true` if `oid` is the OID of the `ltree` type.
pub fn is_ltree_oid(oid: pg_sys::Oid) -> bool {
    static LTREE_OID: OnceLock<pg_sys::Oid> = OnceLock::new();
    let lid = *LTREE_OID
        .get_or_init(|| lookup_typoid(c"public", c"ltree").unwrap_or(pg_sys::Oid::INVALID));
    lid != pg_sys::Oid::INVALID && oid == lid
}

/// Converts a facet-encoded string (null-byte–separated path as stored by Tantivy's fast-field
/// reader) back to a dot-separated ltree path string.
/// Tantivy's stored-fields reader returns `OwnedValue::Facet`, but the fast-field (columnar)
/// reader returns the raw internal representation as `OwnedValue::Str` with null-byte separators
/// and a leading null byte (e.g. `\0Top\0Science\0Biology`).
// This helper handles both cases by
/// stripping the leading null and replacing interior null bytes with dots.
pub fn facet_encoded_str_to_ltree_text(s: &str) -> String {
    s.trim_start_matches('\0').replace('\0', ".")
}

pub enum CollationProvider {
    #[cfg(any(feature = "pg17", feature = "pg18"))]
    Builtin,
    Icu,
    Libc,
    Unknown(#[expect(dead_code)] u8),
}

impl From<u8> for CollationProvider {
    fn from(c: u8) -> Self {
        match c {
            #[cfg(any(feature = "pg17", feature = "pg18"))]
            pg_sys::COLLPROVIDER_BUILTIN => Self::Builtin,
            pg_sys::COLLPROVIDER_ICU => Self::Icu,
            pg_sys::COLLPROVIDER_LIBC => Self::Libc,
            other => Self::Unknown(other),
        }
    }
}

pub struct CollationLocale {
    pub provider: CollationProvider,
    /// libc LC_COLLATE-style locale name. Reliably present only for libc
    /// providers (collcollate is NULL for ICU/builtin rows in pg_collation).
    pub name: Option<String>,
}

/// Helper function to lookup the database's `datcollate` and `datlocprovider` settings from `pg_database`
pub fn lookup_database_collation_locale() -> Option<CollationLocale> {
    unsafe {
        let entry = SysCacheEntry::search1(
            pg_sys::SysCacheIdentifier::DATABASEOID,
            pg_sys::MyDatabaseId.into_datum()?,
        )?;

        let datcollate = entry.attr::<String>(pg_sys::Anum_pg_database_datcollate)?;
        let datlocprovider = entry.attr::<i8>(pg_sys::Anum_pg_database_datlocprovider)?;
        Some(CollationLocale {
            provider: CollationProvider::from(datlocprovider as u8),
            name: Some(datcollate),
        })
    }
}

/// Helper function to lookup the `collcollate` and `collprovider` fields for a collation object in `pg_collation`
/// Note that while `collprovider` is always present in `pg_collation`, `collcollate` may be NULL: <https://www.postgresql.org/docs/current/catalog-pg-collation.html>
pub fn lookup_collation_locale(collation: pg_sys::Oid) -> Option<CollationLocale> {
    unsafe {
        let entry =
            SysCacheEntry::search1(pg_sys::SysCacheIdentifier::COLLOID, collation.into_datum()?)?;

        let collcollate = entry.attr::<String>(pg_sys::Anum_pg_collation_collcollate);
        let collprovider = entry.attr::<i8>(pg_sys::Anum_pg_collation_collprovider)?;
        Some(CollationLocale {
            provider: CollationProvider::from(collprovider as u8),
            name: collcollate,
        })
    }
}
