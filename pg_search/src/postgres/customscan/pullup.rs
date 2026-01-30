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

//! Utilities for "pulling up" values from the index (i.e. using fast fields).
//!
//! This module provides shared logic for determining if and how a PostgreSQL
//! column can be resolved using Tantivy fast fields.

use crate::index::fast_fields_helper::{FastFieldType, WhichFastField};
use crate::postgres::rel::PgSearchRelation;
use crate::schema::FieldSource;
use pgrx::pg_sys;

/// Resolves a PostgreSQL attribute number to a Tantivy fast field, if available.
///
/// This checks if the column corresponding to `attno` in the `heaprel` is available
/// as a fast field in the `index` relation. It handles:
///
/// - System columns (ctid, tableoid)
/// - Regular columns (mapped via name)
/// - Type compatibility checks
/// - Field source verification (ensuring index field comes from the expected heap column)
///
/// Returns `Some(WhichFastField)` if the column can be pulled up from the index,
/// or `None` if it cannot (e.g. not indexed, not a fast field, incompatible type).
pub unsafe fn resolve_fast_field(
    attno: i32,
    tupdesc: &pgrx::PgTupleDesc<'_>,
    index: &PgSearchRelation,
) -> Option<WhichFastField> {
    match attno {
        // any of these mean we can't use fast fields
        pg_sys::MinTransactionIdAttributeNumber
        | pg_sys::MaxTransactionIdAttributeNumber
        | pg_sys::MinCommandIdAttributeNumber
        | pg_sys::MaxCommandIdAttributeNumber => None,

        // these aren't _exactly_ fast fields, but we do have the information
        // readily available during the scan, so we'll pretend
        pg_sys::SelfItemPointerAttributeNumber => Some(WhichFastField::Ctid),

        pg_sys::TableOidAttributeNumber => Some(WhichFastField::TableOid),

        attno => {
            // Handle attno <= 0 - this can happen in materialized views and FULL JOINs
            if attno <= 0 {
                return None;
            }

            // Get attribute info - use if let to handle missing attributes gracefully
            let att = tupdesc.get((attno - 1) as usize)?;
            let schema = index.schema().ok()?;

            if let Some(search_field) = schema.search_field(att.name()) {
                // Check if this is the key field (implicitly fast)
                let key_field_name = schema.key_field_name();
                if att.name() == key_field_name.to_string().as_str() {
                    let ff_type = FastFieldType::from(schema.key_field_type());
                    return Some(WhichFastField::Named(att.name().to_string(), ff_type));
                }

                let categorized_fields = schema.categorized_fields();
                let field_data = categorized_fields
                    .iter()
                    .find(|(sf, _)| sf == &search_field)
                    .map(|(_, data)| data);

                if let Some(data) = field_data {
                    // Ensure that the expression used to index the value exactly matches the
                    // expression used in the target list (which we know is a Var, because
                    // that is the only thing that calls this function with attno > 0).
                    //
                    // Expression indices where target list references original column are not supported.
                    // See: https://github.com/paradedb/paradedb/issues/3978
                    if !matches!(data.source, FieldSource::Heap { attno: source_attno } if source_attno == (attno - 1) as usize)
                    {
                        return None;
                    }

                    if search_field.is_fast() {
                        if let Some(ff_type) =
                            fast_field_type_for_pullup(data.base_oid.value(), data.is_array)
                        {
                            return Some(WhichFastField::Named(att.name().to_string(), ff_type));
                        }
                    }
                }
            }
            None
        }
    }
}

/// Maps a PostgreSQL type OID to a Tantivy `FastFieldType` for pullup execution.
///
/// Returns `Some(FastFieldType)` if the type is supported for fast field execution,
/// `None` otherwise (e.g. arrays, JSON, numeric which require special handling).
pub fn fast_field_type_for_pullup(base_oid: pg_sys::Oid, is_array: bool) -> Option<FastFieldType> {
    if is_array {
        return None;
    }
    match base_oid {
        pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::UUIDOID => Some(FastFieldType::String),
        pg_sys::BOOLOID => Some(FastFieldType::Bool),
        pg_sys::DATEOID
        | pg_sys::TIMEOID
        | pg_sys::TIMESTAMPOID
        | pg_sys::TIMESTAMPTZOID
        | pg_sys::TIMETZOID => Some(FastFieldType::Date),
        pg_sys::FLOAT4OID | pg_sys::FLOAT8OID => Some(FastFieldType::Float64),
        pg_sys::INT2OID | pg_sys::INT4OID | pg_sys::INT8OID => Some(FastFieldType::Int64),
        _ => {
            // This fast field type is supported for pushdown of queries, but not for
            // rendering via fast field execution.
            //
            // JSON/JSONB are excluded because fast fields do not contain the
            // full content of the JSON in a way that we can easily render:
            // rather, the individual fields are exploded out into dynamic
            // columns.
            //
            // NUMERIC is excluded because we do not store the original
            // precision/scale in the index, so we cannot safely reconstruct the
            // value without potentially losing precision. See:
            // https://github.com/paradedb/paradedb/issues/2968
            None
        }
    }
}
