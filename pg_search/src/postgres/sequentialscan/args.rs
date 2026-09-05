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

use crate::query::SearchQueryInput;
use pgrx::callconv::{Arg, ArgAbi};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
};

/// Allows us to have a UDF with these types as arguments but not do any pgrx-related datum conversion
pub struct FakeAnyElement;
pub struct FakeSearchQueryInput;
pub struct FakeCtid;

unsafe impl<'fcx> ArgAbi<'fcx> for FakeAnyElement {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

// Unlike `FakeSearchQueryInput` below, this does not borrow another type's resolution
// because `anyelement` is a Postgres pseudo-type with no graph entry to depend on.
unsafe impl SqlTranslatable for FakeAnyElement {
    const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(FakeAnyElement);
    const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        Ok(SqlMappingRef::literal("anyelement"));
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

unsafe impl<'fcx> ArgAbi<'fcx> for FakeSearchQueryInput {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

unsafe impl SqlTranslatable for FakeSearchQueryInput {
    // This is intentionally borrowing `SearchQueryInput`'s `TYPE_IDENT`: current
    // pgrx tolerates two Rust types sharing that identifier, and we rely on that
    // observed behavior so the SQL graph still emits `CREATE TYPE SearchQueryInput`
    // before any function that consumes this fake wrapper.
    const TYPE_IDENT: &'static str = <SearchQueryInput as SqlTranslatable>::TYPE_IDENT;
    const TYPE_ORIGIN: TypeOrigin = <SearchQueryInput as SqlTranslatable>::TYPE_ORIGIN;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
        <SearchQueryInput as SqlTranslatable>::ARGUMENT_SQL;
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}

unsafe impl<'fcx> ArgAbi<'fcx> for FakeCtid {
    unsafe fn unbox_arg_unchecked(_arg: Arg<'_, 'fcx>) -> Self {
        Self
    }
}

unsafe impl SqlTranslatable for FakeCtid {
    const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(FakeCtid);
    const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
    const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> = Ok(SqlMappingRef::literal("tid"));
    const RETURN_SQL: Result<ReturnsRef, ReturnsError> = Err(ReturnsError::Datum);
}
