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

//! Composite type handling for BM25 indexes
//!
//! This module provides functionality to:
//! - Detect composite types in index definitions
//! - Validate composite types for indexing
//! - Extract field metadata from composite types
//! - Unpack composite values during indexing

use crate::api::HashMap;
use pgrx::{heap_tuple::PgHeapTuple, pg_sys, PgTupleDesc};

/// Metadata for a field within a composite type
#[derive(Debug, Clone)]
pub struct CompositeFieldInfo {
    /// Field name from TYPE definition
    pub field_name: String,

    /// Position within the composite (0-indexed)
    pub field_index: usize,

    /// PostgreSQL type OID
    pub type_oid: pg_sys::Oid,

    /// Type modifier (contains tokenizer config)
    pub typmod: i32,

    /// Whether this attribute was dropped (attisdropped).
    /// Indexed composites can't normally have dropped attrs (ALTER TYPE DROP ATTRIBUTE
    /// is blocked by dependencies), but we keep this to mirror the tupdesc and preserve
    /// positional mapping if it ever appears.
    pub is_dropped: bool,
}

/// Errors that can occur during composite type handling
#[derive(Debug, thiserror::Error)]
pub enum CompositeError {
    #[error("Anonymous ROW expressions are not supported for BM25 indexes. Create a named composite type: CREATE TYPE my_type AS (...); then use ROW(...)::my_type")]
    AnonymousRowNotSupported,

    #[error("Domain over composite type is not supported for BM25 indexes. Use the base composite type directly instead of the domain.")]
    DomainOverCompositeNotSupported,

    #[error("Type OID {0} is not a composite type")]
    NotACompositeType(pg_sys::Oid),

    #[error("Failed to lookup tuple descriptor for type OID {0}")]
    TupleDescLookupFailed(pg_sys::Oid),

    #[error("Nested composite types are not supported for BM25 indexes.")]
    NestedCompositeNotSupported,
}

/// Check if a type OID is a composite type (not RECORD, not domain)
///
/// # Safety
/// Caller must ensure type_oid is valid
pub unsafe fn is_composite_type(type_oid: pg_sys::Oid) -> bool {
    pg_sys::get_typtype(type_oid) as u8 == pg_sys::TYPTYPE_COMPOSITE
}

/// Check if type is anonymous RECORD (must be rejected)
///
/// Anonymous ROW expressions like ROW(1, 2, 3) without a type cast
/// have type RECORDOID and are not supported because we cannot
/// introspect their structure reliably.
pub fn is_anonymous_record(type_oid: pg_sys::Oid) -> bool {
    type_oid == pg_sys::RECORDOID
}

/// Check if type is a domain type
///
/// # Safety
/// Caller must ensure type_oid is valid
pub unsafe fn is_domain_type(type_oid: pg_sys::Oid) -> bool {
    pg_sys::get_typtype(type_oid) as u8 == pg_sys::TYPTYPE_DOMAIN
}

/// Check if a domain wraps a composite type
///
/// # Safety
/// Caller must ensure type_oid is valid
pub unsafe fn is_domain_over_composite(type_oid: pg_sys::Oid) -> bool {
    if !is_domain_type(type_oid) {
        return false;
    }
    let base_oid = pg_sys::getBaseType(type_oid);
    is_composite_type(base_oid)
}

/// Extract field information from a named composite type.
///
/// This function introspects a composite type's structure and returns
/// metadata for each field. It uses PostgreSQL's lookup_rowtype_tupdesc()
/// to get the type's tuple descriptor.
///
/// # Arguments
/// * `type_oid` - The OID of the composite type
///
/// # Returns
/// * `Ok(Vec<CompositeFieldInfo>)` - Field metadata for all fields
/// * `Err(CompositeError)` - If type is invalid or not a composite
///
/// # Errors
/// Returns error if:
/// - Type is RECORDOID (anonymous ROW)
/// - Type is a domain over composite
/// - Type is not a composite type
/// - Tuple descriptor lookup fails
///
/// # Safety
/// Caller must ensure type_oid is valid and that we're in a PostgreSQL context.
pub unsafe fn get_composite_type_fields(
    type_oid: pg_sys::Oid,
) -> Result<Vec<CompositeFieldInfo>, CompositeError> {
    if is_anonymous_record(type_oid) {
        return Err(CompositeError::AnonymousRowNotSupported);
    }

    if is_domain_over_composite(type_oid) {
        return Err(CompositeError::DomainOverCompositeNotSupported);
    }

    if !is_composite_type(type_oid) {
        return Err(CompositeError::NotACompositeType(type_oid));
    }

    // For named composites, typmod is always -1 (schema in catalog)
    let tupdesc = pg_sys::lookup_rowtype_tupdesc(type_oid, -1);
    if tupdesc.is_null() {
        return Err(CompositeError::TupleDescLookupFailed(type_oid));
    }
    let pg_tupdesc = PgTupleDesc::from_pg(tupdesc);
    let natts = pg_tupdesc.len();
    let mut fields = Vec::with_capacity(natts);

    for i in 0..natts {
        // PgTupleDesc provides proper access to attributes
        let att = pg_tupdesc.get(i).expect("attribute index should be valid");

        fields.push(CompositeFieldInfo {
            field_name: att.name().to_string(),
            field_index: i,
            type_oid: att.type_oid().value(),
            typmod: att.type_mod(),
            is_dropped: att.is_dropped(),
        });
    }

    Ok(fields)
}

/// Check if a composite type contains nested composite fields.
///
/// Nested composites are not supported because they would require
/// recursive unpacking and complex field naming schemes.
///
/// # Safety
/// Caller must ensure type_oid is valid.
pub unsafe fn has_nested_composite(type_oid: pg_sys::Oid) -> bool {
    let fields = match get_composite_type_fields(type_oid) {
        Ok(f) => f,
        Err(_) => return false,
    };

    fields
        .iter()
        .filter(|f| !f.is_dropped)
        .any(|f| is_composite_type(f.type_oid))
}

/// Get validated composite fields for use in BM25 index.
///
/// This is the main entry point called during index creation.
/// Returns field info after validating for unsupported configurations.
///
/// # Arguments
/// * `type_oid` - The OID of the composite type
///
/// # Returns
/// * `Ok(Vec<CompositeFieldInfo>)` - Valid fields ready for indexing
/// * `Err(CompositeError)` - If validation fails
///
/// # Errors
/// Returns error if:
/// - Type is anonymous ROW, domain over composite, or not composite
/// - Any field is itself a composite (nested)
///
/// # Safety
/// Caller must ensure type_oid is valid and we're in PostgreSQL context.
pub unsafe fn get_composite_fields_for_index(
    type_oid: pg_sys::Oid,
) -> Result<Vec<CompositeFieldInfo>, CompositeError> {
    // Step 1: Get fields (this validates type is composite, not RECORD, not domain)
    let fields = get_composite_type_fields(type_oid)?;

    // Step 2: Check for nested composites
    if has_nested_composite(type_oid) {
        return Err(CompositeError::NestedCompositeNotSupported);
    }

    Ok(fields)
}

/// Pre-unpacked composite values for a row.
///
/// All composites are unpacked upfront when this struct is created,
/// eliminating on-demand caching complexity.
///
/// # Example
/// ```sql
/// CREATE INDEX idx ON t USING bm25 (id, ROW(a,b,c)::my_type);
/// ```
/// The composite at values[1] is unpacked once during construction.
/// Fields "a", "b", "c" are then retrieved via simple lookups.
///
/// # Lifetime
/// Created once per row, dropped after row is indexed.
#[derive(Default)]
pub struct CompositeSlotValues {
    /// Unpacked values: slot_index → [(Datum, is_null), ...]
    unpacked: HashMap<usize, Vec<(pg_sys::Datum, bool)>>,
}

impl CompositeSlotValues {
    /// Create with all composites unpacked upfront.
    ///
    /// # Arguments
    /// * `composites` - Iterator of (slot_index, datum, is_null, type_oid) for each composite
    ///
    /// # Safety
    /// Caller must ensure all datums and type_oids are valid.
    pub unsafe fn from_composites<I>(composites: I) -> Self
    where
        I: IntoIterator<Item = (usize, pg_sys::Datum, bool, pg_sys::Oid)>,
    {
        let mut unpacked = HashMap::default();

        for (slot_index, datum, is_null, type_oid) in composites {
            if unpacked.contains_key(&slot_index) {
                continue;
            }
            let fields = Self::unpack_datum(datum, is_null, type_oid);
            unpacked.insert(slot_index, fields);
        }

        Self { unpacked }
    }

    /// Get a field from a pre-unpacked composite.
    ///
    /// # Panics
    /// Panics if the slot was not unpacked or field_idx is out of bounds.
    pub fn get(&self, slot_index: usize, field_idx: usize) -> (pg_sys::Datum, bool) {
        self.unpacked[&slot_index][field_idx]
    }

    /// Unpack a single composite datum into its field values.
    unsafe fn unpack_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        type_oid: pg_sys::Oid,
    ) -> Vec<(pg_sys::Datum, bool)> {
        let tupdesc = pg_sys::lookup_rowtype_tupdesc(type_oid, -1);
        let pg_tupdesc = PgTupleDesc::from_pg(tupdesc);
        let natts = pg_tupdesc.len();

        if is_null {
            return vec![(pg_sys::Datum::from(0), true); natts];
        }

        let heap_tuple = PgHeapTuple::from_composite_datum(datum);
        let htup_data = heap_tuple.into_pg();
        let mut values = vec![pg_sys::Datum::from(0); natts];
        let mut nulls_raw = vec![false; natts];

        pg_sys::heap_deform_tuple(
            htup_data,
            pg_tupdesc.as_ptr(),
            values.as_mut_ptr(),
            nulls_raw.as_mut_ptr(),
        );

        values.into_iter().zip(nulls_raw).collect()
    }
}
