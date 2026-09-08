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

use crate::api::FieldName;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::schema::{FieldSource, SearchFieldType};
use pgrx::pg_sys;
use tantivy::columnar::ColumnType;

/// Resolves a PostgreSQL attribute number to a Tantivy fast field, if available.
///
/// This checks if the column corresponding to `attno` in the heap relation is available
/// as a fast field in the ParadeDB index. It handles:
///
/// - System columns (ctid, tableoid)
/// - Regular columns (mapped via name)
/// - Expression-indexed columns (e.g. typmod casts like `(col)::pdb.literal_normalized`)
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
            if attno <= 0 {
                return None;
            }

            let att = tupdesc.get((attno - 1) as usize)?;
            let schema = index.schema().ok()?;

            if let Some(search_field) = schema.search_field(att.name()) {
                let key_field_name = schema.key_field_name();
                if att.name() == key_field_name.to_string().as_str() {
                    return Some(WhichFastField::Named(
                        att.name().to_string(),
                        schema.key_field_type(),
                    ));
                }

                let categorized_fields = schema.categorized_fields();
                let field_data = categorized_fields
                    .iter()
                    .find(|(sf, _)| sf == &search_field)
                    .map(|(_, data)| data);

                if let Some(data) = field_data {
                    // For direct heap columns, the source attno must match.
                    // Expression-indexed columns (FieldSource::Expression) are NOT
                    // handled here — pulling up the raw column value from a transformed
                    // expression (e.g. lower(name)) would return wrong data.
                    // See: https://github.com/paradedb/paradedb/issues/3978
                    //
                    // Expression-indexed columns that are simple tokenizer casts
                    // (e.g. (col)::pdb.literal_normalized) are handled by the
                    // find_matching_fast_field fallback below instead.
                    if matches!(data.source, FieldSource::Heap { attno: source_attno } if source_attno == (attno - 1) as usize)
                        && search_field.is_fast()
                        && let Some(field_type) =
                            field_type_for_pullup(search_field.field_type(), data.is_array)
                    {
                        return Some(WhichFastField::Named(att.name().to_string(), field_type));
                    }
                }
            }

            // Fallback for expression-indexed columns (e.g. typmod casts like
            // `(col)::pdb.literal_normalized`). These have FieldSource::Expression
            // which the check above skips, but find_matching_fast_field handles
            // by stripping the tokenizer cast and matching the underlying Var.
            //
            // We use varno=1 because Postgres stores index expressions with
            // varno=1 regardless of the table's actual range table index in
            // the query. find_matching_fast_field compares via pg_sys::equal(),
            // so both sides must use the same varno to match.
            let dummy_var = pg_sys::makeVar(
                1,
                attno as pg_sys::AttrNumber,
                att.atttypid,
                att.atttypmod,
                att.attcollation,
                0,
            );
            find_matching_fast_field(
                dummy_var as *mut pg_sys::Node,
                &index.index_expressions(),
                index.schema().ok()?,
                1,
            )
        }
    }
}

/// Resolve a fast field by Tantivy field name (e.g., `metadata.category`).
///
/// Used for JSON sub-fields where [`resolve_fast_field`] fails because the attno
/// maps to the parent JSON column rather than the sub-field. Tantivy stores JSON
/// sub-fields as separate fast fields with dotted names.
pub fn resolve_fast_field_by_name(
    field_name: &str,
    index: &PgSearchRelation,
) -> Option<WhichFastField> {
    let schema = index.schema().ok()?;
    let search_field = schema.search_field(field_name)?;
    if search_field.is_fast() {
        Some(WhichFastField::Named(
            field_name.to_string(),
            search_field.field_type(),
        ))
    } else {
        None
    }
}

/// Resolve an index field name, dotted for a JSON sub-field, to the heap column it
/// is read from and the type a columnar scan hands out. `Ok(None)` when the index
/// has no such fast field, `Err` when it has one the scan cannot read, so the
/// caller can say why rather than that the field does not exist. Resolution goes
/// through the index schema because a field name can be an alias of a column.
/// The gates match [`resolve_fast_field`]: an expression-indexed or array column
/// is turned down, and a JSON column is only reachable through a sub-field.
pub fn resolve_index_field_by_name(
    index: &PgSearchRelation,
    field: &str,
) -> Result<Option<(pg_sys::AttrNumber, SearchFieldType)>, String> {
    let Some(schema) = index.schema().ok() else {
        return Ok(None);
    };
    let root = FieldName::from(field).root();
    let categorized = schema.categorized_fields();
    let Some((search_field, data)) = categorized
        .iter()
        .find(|(sf, _)| sf.field_name().as_ref() == root && sf.is_fast())
    else {
        return Ok(None);
    };
    let FieldSource::Heap { attno } = data.source else {
        return Err(format!(
            "Field '{field}' is an expression index, which cannot be read back as a column"
        ));
    };
    if data.is_array {
        return Err(format!(
            "Field '{field}' is an array, which cannot be read as a single column"
        ));
    }
    let field_type = if field == root {
        match field_type_for_pullup(search_field.field_type(), false) {
            Some(field_type) => field_type,
            None if data.is_json => {
                return Err(format!(
                    "Field '{field}' is a JSON column; name a sub-field, as in '{field}.key'"
                ));
            }
            None => return Ok(None),
        }
    } else {
        check_json_sub_field_is_text(index, field)?;
        search_field.field_type()
    };
    Ok(Some((attno as pg_sys::AttrNumber + 1, field_type)))
}

/// A columnar scan reads every JSON sub-field as text, while the index stores
/// each sub-field in the type of its values. A numeric sub-field would come back
/// as numbers and fail the scan's schema at execution, so it is turned down
/// here. Only the column metadata is read; no values are loaded.
fn check_json_sub_field_is_text(index: &PgSearchRelation, field: &str) -> Result<(), String> {
    let reader =
        SearchIndexReader::open(index, SearchQueryInput::All, false, MvccSatisfies::Snapshot)
            .map_err(|e| format!("could not open index to inspect field '{field}': {e}"))?;
    for segment in reader.segment_readers() {
        let handles = segment
            .fast_fields()
            .dynamic_column_handles(field)
            .map_err(|e| format!("could not inspect field '{field}': {e}"))?;
        if let Some(handle) = handles
            .iter()
            .find(|handle| handle.column_type() != ColumnType::Str)
        {
            return Err(format!(
                "Field '{field}' is a JSON sub-field holding {} values; only a text sub-field \
                 can be read as a column",
                handle.column_type()
            ));
        }
    }
    Ok(())
}

/// Look up a 1-based attribute number by column name in a tuple descriptor.
pub fn get_attno_by_name(
    name: &str,
    tupdesc: &pgrx::PgTupleDesc<'_>,
) -> Option<pg_sys::AttrNumber> {
    for (i, att) in tupdesc.iter().enumerate() {
        if att.name() == name {
            return Some((i + 1) as pg_sys::AttrNumber);
        }
    }
    None
}

/// Returns the `SearchFieldType` if it's supported for fast field pullup execution.
///
/// Returns `Some(SearchFieldType)` if the type is supported for fast field execution,
/// `None` otherwise (e.g. arrays, JSON which require special handling).
pub fn field_type_for_pullup(
    field_type: SearchFieldType,
    is_array: bool,
) -> Option<SearchFieldType> {
    if is_array {
        return None;
    }
    match field_type {
        // JSON/JSONB are excluded because fast fields do not contain the
        // full content of the JSON in a way that we can easily render:
        // rather, the individual fields are exploded out into dynamic columns.
        SearchFieldType::Json(_) => None,
        // Range types are not yet supported for pullup
        SearchFieldType::Range(_) => None,
        // All other types can be pulled up directly
        _ => Some(field_type),
    }
}
