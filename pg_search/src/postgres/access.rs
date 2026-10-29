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

//! Authorization for SQL APIs that access an index outside a policy-aware scan.
//!
//! Do not use these guards for normal SQL scans: PostgreSQL owns their RTE
//! permission identity and row-policy evaluation. Low-level readers also serve
//! build, vacuum and recovery, so authorization belongs at the public boundary.

use crate::postgres::catalog::OidExt;
use crate::postgres::rel::PgSearchRelation;
use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
};
use pgrx::{PgSqlErrorCode, pg_sys};
use std::ffi::CStr;

/// A `regclass` argument without pgrx's implicit index lock.
///
/// `PgRelation::from_datum` locks the index before the function starts. Keeping
/// only the OID lets acquisition follow PostgreSQL's table-before-index order.
pub struct IndexOid(pg_sys::Oid);

impl From<pg_sys::Oid> for IndexOid {
    fn from(oid: pg_sys::Oid) -> Self {
        Self(oid)
    }
}

unsafe impl<'fcx> ArgAbi<'fcx> for IndexOid {
    unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'fcx>) -> Self {
        Self(unsafe { pg_sys::Oid::unbox_arg_unchecked(arg) })
    }
}

unsafe impl SqlTranslatable for IndexOid {
    const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(IndexOid);
    const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        Ok(SqlMappingRef::literal("regclass"));
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

impl IndexOid {
    pub(crate) fn oid(&self) -> pg_sys::Oid {
        self.0
    }

    pub(crate) fn read(self) -> IndexAccess {
        let access = IndexAccess::open(self.0, pg_sys::AccessShareLock as _);
        if let Err(denied) = check_read(access.heap()) {
            denied.report();
        }
        access
    }

    /// Inventory APIs omit entries whose direct-read policy denies access.
    /// Missing relations, wrong kinds and operational failures still raise errors.
    pub(crate) fn is_readable(&self) -> bool {
        let access = IndexAccess::open(self.0, pg_sys::AccessShareLock as _);
        check_read(access.heap()).is_ok()
    }

    /// Trusted diagnostics may inspect metadata without table SELECT or RLS.
    pub(crate) fn read_metadata(self) -> IndexAccess {
        let access = IndexAccess::open(self.0, pg_sys::AccessShareLock as _);
        if let Err(denied) = check_metadata(access.heap()) {
            denied.report();
        }
        access
    }

    /// Like metadata reads, but inventory omits entries denied to this caller.
    pub(crate) fn read_metadata_if_permitted(self) -> Option<IndexAccess> {
        let access = IndexAccess::open(self.0, pg_sys::AccessShareLock as _);
        check_metadata(access.heap()).ok()?;
        Some(access)
    }

    /// Maintenance follows REINDEX: `MAINTAIN` on the backing table from
    /// PostgreSQL 17, ownership before that.
    pub(crate) fn maintain(self) -> IndexAccess {
        let access = IndexAccess::open(self.0, pg_sys::RowExclusiveLock as _);
        if let Err(denied) = check_maintain(access.heap()) {
            denied.report();
        }
        access
    }
}

/// An index opened after its backing table, with both locks held until drop.
/// Derefs to the index; the entry method on `IndexOid` determines the policy.
/// `open` only locks relations and does not authorize reads.
// Fields drop in declaration order: the index closes before the heap.
pub(crate) struct IndexAccess {
    index: PgSearchRelation,
    heap: PgSearchRelation,
}

impl std::ops::Deref for IndexAccess {
    type Target = PgSearchRelation;

    fn deref(&self) -> &PgSearchRelation {
        &self.index
    }
}

impl IndexAccess {
    pub(crate) fn open(index_oid: pg_sys::Oid, index_lock: pg_sys::LOCKMODE) -> Self {
        // Like amcheck_lock_relation_and_check: resolve without an index
        // lock, lock the table first, then recheck the association.
        let heap_oid = unsafe { pg_sys::IndexGetRelation(index_oid, true) };
        if heap_oid == pg_sys::InvalidOid {
            unsafe { report_not_an_index(index_oid) };
        }
        let heap = PgSearchRelation::with_lock(heap_oid, pg_sys::AccessShareLock as _);
        let index = PgSearchRelation::with_lock(index_oid, index_lock);
        if index.rd_index.is_null() || unsafe { (*index.rd_index).indrelid } != heap_oid {
            pg_sys::ereport!(
                ERROR,
                PgSqlErrorCode::ERRCODE_UNDEFINED_TABLE,
                format!(
                    "could not open the backing table of index \"{}\"",
                    index.name()
                )
            );
        }
        if !unsafe { (*index.rd_rel).relam }.is_paradedb_am() {
            pg_sys::ereport!(
                ERROR,
                PgSqlErrorCode::ERRCODE_WRONG_OBJECT_TYPE,
                format!("\"{}\" is not a ParadeDB index", index.name())
            );
        }
        Self { index, heap }
    }

    pub(crate) fn heap(&self) -> &PgSearchRelation {
        &self.heap
    }
}

unsafe fn report_not_an_index(oid: pg_sys::Oid) -> ! {
    let name = unsafe { pg_sys::get_rel_name(oid) };
    if name.is_null() {
        pg_sys::ereport!(
            ERROR,
            PgSqlErrorCode::ERRCODE_UNDEFINED_TABLE,
            format!("relation with OID {} does not exist", oid.to_u32())
        );
    }
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    pg_sys::ereport!(
        ERROR,
        PgSqlErrorCode::ERRCODE_WRONG_OBJECT_TYPE,
        format!("\"{name}\" is not an index")
    );
}

fn check_maintain(heap: &PgSearchRelation) -> Result<(), AccessDenied> {
    // Owners and pg_maintain members hold MAINTAIN implicitly.
    #[cfg(any(feature = "pg17", feature = "pg18"))]
    let acl = unsafe {
        pg_sys::pg_class_aclcheck(heap.oid(), pg_sys::GetUserId(), pg_sys::ACL_MAINTAIN as _)
    };
    #[cfg(any(feature = "pg15", feature = "pg16"))]
    let acl = if owns_table(heap) {
        pg_sys::AclResult::ACLCHECK_OK
    } else {
        pg_sys::AclResult::ACLCHECK_NOT_OWNER
    };
    if acl != pg_sys::AclResult::ACLCHECK_OK {
        return Err(AccessDenied::Table {
            acl,
            relation: heap.oid(),
        });
    }
    Ok(())
}

#[cfg(any(feature = "pg15", feature = "pg16"))]
fn owns_table(heap: &PgSearchRelation) -> bool {
    unsafe {
        #[cfg(feature = "pg15")]
        {
            pg_sys::pg_class_ownercheck(heap.oid(), pg_sys::GetUserId())
        }
        #[cfg(not(feature = "pg15"))]
        {
            pg_sys::object_ownercheck(pg_sys::RelationRelationId, heap.oid(), pg_sys::GetUserId())
        }
    }
}

enum AccessDenied {
    Schema {
        acl: pg_sys::AclResult::Type,
        namespace: pg_sys::Oid,
    },
    Table {
        acl: pg_sys::AclResult::Type,
        relation: pg_sys::Oid,
    },
    RowSecurity {
        schema: String,
        table: String,
    },
}

fn check_metadata(heap: &PgSearchRelation) -> Result<(), AccessDenied> {
    unsafe {
        // Same trusted role as pgstattuple; pg_monitor inherits it. It sees
        // whole-index counts under RLS and needs no schema USAGE.
        if pg_sys::has_privs_of_role(
            pg_sys::GetUserId(),
            pg_sys::Oid::from(pg_sys::ROLE_PG_STAT_SCAN_TABLES),
        ) {
            return Ok(());
        }
    }
    check_read(heap)
}

fn check_read(heap: &PgSearchRelation) -> Result<(), AccessDenied> {
    unsafe {
        // GetUserId rather than the session user, so SET ROLE and SECURITY
        // DEFINER apply.
        let user = pg_sys::GetUserId();
        // OID/regclass arguments do not necessarily perform name resolution,
        // so check schema USAGE explicitly even for numeric arguments.
        let namespace = (*heap.rd_rel).relnamespace;
        #[cfg(feature = "pg15")]
        let schema_acl = pg_sys::pg_namespace_aclcheck(namespace, user, pg_sys::ACL_USAGE);
        #[cfg(not(feature = "pg15"))]
        let schema_acl = pg_sys::object_aclcheck(
            pg_sys::NamespaceRelationId,
            namespace,
            user,
            pg_sys::ACL_USAGE as _,
        );
        if schema_acl != pg_sys::AclResult::ACLCHECK_OK {
            return Err(AccessDenied::Schema {
                acl: schema_acl,
                namespace,
            });
        }
        // Table-level SELECT, not a column grant: query strings and index
        // expressions can read columns other than the ones a caller names.
        let table_acl = pg_sys::pg_class_aclcheck(heap.oid(), user, pg_sys::ACL_SELECT as _);
        if table_acl != pg_sys::AclResult::ACLCHECK_OK {
            return Err(AccessDenied::Table {
                acl: table_acl,
                relation: heap.oid(),
            });
        }
        // These readers cannot apply row policies, so fail closed. noError=true
        // still reports RLS_ENABLED when row_security=off, so the GUC is not a
        // bypass. MVCC visibility is not a substitute for either check.
        if pg_sys::check_enable_rls(heap.oid(), user, true)
            == pg_sys::CheckEnableRlsResult::RLS_ENABLED as i32
        {
            return Err(AccessDenied::RowSecurity {
                schema: heap.namespace().to_owned(),
                table: heap.name().to_owned(),
            });
        }
        Ok(())
    }
}

impl AccessDenied {
    fn report(self) -> ! {
        unsafe {
            match self {
                Self::Schema { acl, namespace } => pg_sys::aclcheck_error(
                    acl,
                    pg_sys::ObjectType::OBJECT_SCHEMA,
                    pg_sys::get_namespace_name(namespace),
                ),
                Self::Table { acl, relation } => pg_sys::aclcheck_error(
                    acl,
                    pg_sys::ObjectType::OBJECT_TABLE,
                    pg_sys::get_rel_name(relation),
                ),
                Self::RowSecurity { schema, table } => {
                    pg_sys::ereport!(
                        ERROR,
                        PgSqlErrorCode::ERRCODE_INSUFFICIENT_PRIVILEGE,
                        format!(
                            "direct index reads are not supported when row-level security applies to table {}.{}",
                            pgrx::spi::quote_identifier(&schema),
                            pgrx::spi::quote_identifier(&table),
                        )
                    );
                }
            }
        }
        unreachable!("aclcheck_error always raises PostgreSQL ERROR")
    }
}
