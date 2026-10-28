// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

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
/// This value confers no authority; callers must explicitly request an operation.
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
    /// The identifier alone permits catalog classification, not an index read.
    pub(crate) fn oid(&self) -> pg_sys::Oid {
        self.0
    }

    pub(crate) fn read(self) -> StandaloneIndexRead {
        let relations = LockedIndexRelations::open(self.0, pg_sys::AccessShareLock as _);
        if let Err(denied) = check_read(&relations.heap) {
            denied.report(&relations.heap);
        }
        StandaloneIndexRead { relations }
    }

    /// Inventory APIs omit entries whose direct-read policy denies access.
    /// Missing relations, wrong kinds and operational failures still raise errors.
    pub(crate) fn is_readable(&self) -> bool {
        let relations = LockedIndexRelations::open(self.0, pg_sys::AccessShareLock as _);
        check_read(&relations.heap).is_ok()
    }

    /// Trusted diagnostics may inspect metadata without table SELECT or RLS.
    pub(crate) fn read_metadata(self) -> IndexMetadataRead {
        let relations = LockedIndexRelations::open(self.0, pg_sys::AccessShareLock as _);
        if let Err(denied) = check_metadata(&relations.heap) {
            denied.report(&relations.heap);
        }
        IndexMetadataRead { relations }
    }

    /// Like metadata reads, but inventory omits entries denied to this caller.
    pub(crate) fn read_metadata_if_permitted(self) -> Option<IndexMetadataRead> {
        let relations = LockedIndexRelations::open(self.0, pg_sys::AccessShareLock as _);
        check_metadata(&relations.heap).ok()?;
        Some(IndexMetadataRead { relations })
    }

    pub(crate) fn maintain(self) -> IndexMaintenance {
        let relations = LockedIndexRelations::open(self.0, pg_sys::RowExclusiveLock as _);
        if !owns_table(&relations.heap) {
            unsafe {
                pg_sys::aclcheck_error(
                    pg_sys::AclResult::ACLCHECK_NOT_OWNER,
                    pg_sys::ObjectType::OBJECT_TABLE,
                    pg_sys::get_rel_name(relations.heap.oid()),
                );
            }
        }
        IndexMaintenance { relations }
    }
}

// Field order also releases our index reference before our heap reference.
struct LockedIndexRelations {
    index: PgSearchRelation,
    heap: PgSearchRelation,
}

impl LockedIndexRelations {
    fn open(index_oid: pg_sys::Oid, index_lock: pg_sys::LOCKMODE) -> Self {
        unsafe {
            // Like amcheck_lock_relation_and_check: resolve without an index
            // lock, lock the table first, then recheck the association.
            let heap_oid = pg_sys::IndexGetRelation(index_oid, true);
            if heap_oid == pg_sys::InvalidOid {
                report_not_an_index(index_oid);
            }
            let heap = PgSearchRelation::with_lock(heap_oid, pg_sys::AccessShareLock as _);
            let index = PgSearchRelation::with_lock(index_oid, index_lock);
            if index.rd_index.is_null() || (*index.rd_index).indrelid != heap_oid {
                pg_sys::ereport!(
                    ERROR,
                    PgSqlErrorCode::ERRCODE_UNDEFINED_TABLE,
                    format!(
                        "could not open the backing table of index \"{}\"",
                        index.name()
                    )
                );
            }
            if !(*index.rd_rel).relam.is_paradedb_am() {
                pg_sys::ereport!(
                    ERROR,
                    PgSqlErrorCode::ERRCODE_WRONG_OBJECT_TYPE,
                    format!("\"{}\" is not a ParadeDB index", index.name())
                );
            }
            Self { index, heap }
        }
    }
}

/// Raises for a relation that is not an index, naming it like PostgreSQL does.
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

/// Both relation locks live until the standalone read has materialized its result.
pub(crate) struct StandaloneIndexRead {
    relations: LockedIndexRelations,
}

impl StandaloneIndexRead {
    pub(crate) fn index(&self) -> &PgSearchRelation {
        &self.relations.index
    }
}

/// Authority for schema and segment statistics, not for document reads.
/// Uses the same lock ownership as a standalone read with a distinct policy.
pub(crate) struct IndexMetadataRead {
    relations: LockedIndexRelations,
}

impl IndexMetadataRead {
    pub(crate) fn index(&self) -> &PgSearchRelation {
        &self.relations.index
    }
}

/// Maintenance authority is distinct from SELECT and from a diagnostic role.
pub(crate) struct IndexMaintenance {
    relations: LockedIndexRelations,
}

impl IndexMaintenance {
    pub(crate) fn index(&self) -> &PgSearchRelation {
        &self.relations.index
    }
}

enum ReadDenied {
    Schema(pg_sys::AclResult::Type),
    Table(pg_sys::AclResult::Type),
    RowSecurity,
}

fn check_metadata(heap: &PgSearchRelation) -> Result<(), ReadDenied> {
    unsafe {
        // Match pgstattuple's trusted diagnostic role. pg_monitor inherits it.
        // Check effective privileges on every call, including inherited grants
        // and revocation. These APIs intentionally expose whole-index counts
        // under RLS, and numeric OIDs need no schema USAGE for this role.
        if pg_sys::has_privs_of_role(
            pg_sys::GetUserId(),
            pg_sys::Oid::from(pg_sys::ROLE_PG_STAT_SCAN_TABLES),
        ) {
            return Ok(());
        }
    }
    check_read(heap)
}

fn check_read(heap: &PgSearchRelation) -> Result<(), ReadDenied> {
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
            return Err(ReadDenied::Schema(schema_acl));
        }
        let table_acl = pg_sys::pg_class_aclcheck(heap.oid(), user, pg_sys::ACL_SELECT as _);
        if table_acl != pg_sys::AclResult::ACLCHECK_OK {
            return Err(ReadDenied::Table(table_acl));
        }
        // noError=true reports RLS_ENABLED even when row_security=off.
        if pg_sys::check_enable_rls(heap.oid(), user, true)
            == pg_sys::CheckEnableRlsResult::RLS_ENABLED as i32
        {
            return Err(ReadDenied::RowSecurity);
        }
        Ok(())
    }
}

impl ReadDenied {
    fn report(self, heap: &PgSearchRelation) -> ! {
        unsafe {
            match self {
                Self::Schema(acl) => pg_sys::aclcheck_error(
                    acl,
                    pg_sys::ObjectType::OBJECT_SCHEMA,
                    pg_sys::get_namespace_name((*heap.rd_rel).relnamespace),
                ),
                Self::Table(acl) => pg_sys::aclcheck_error(
                    acl,
                    pg_sys::ObjectType::OBJECT_TABLE,
                    pg_sys::get_rel_name(heap.oid()),
                ),
                Self::RowSecurity => {
                    pg_sys::ereport!(
                        ERROR,
                        PgSqlErrorCode::ERRCODE_INSUFFICIENT_PRIVILEGE,
                        format!(
                            "direct index reads are not supported when row-level security applies to table {}.{}",
                            pgrx::spi::quote_identifier(heap.namespace()),
                            pgrx::spi::quote_identifier(heap.name()),
                        )
                    );
                }
            }
        }
        unreachable!("aclcheck_error always raises PostgreSQL ERROR")
    }
}
