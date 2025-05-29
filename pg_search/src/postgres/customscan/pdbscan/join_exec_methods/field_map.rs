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

//! Field classification system for optimizing field access patterns
//!
//! This module determines which fields can be loaded from fast fields (Tantivy index)
//! versus those that require expensive heap access.

use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::pdbscan::get_rel_name;
use pgrx::{pg_sys, warning, PgRelation, PgTupleDesc};
use std::collections::{HashMap, HashSet};

/// Loading strategy for a field
#[derive(Debug, Clone, PartialEq)]
pub enum FieldLoadingStrategy {
    /// Field available from fast fields (Tantivy index)
    FastField(WhichFastField),
    /// Field stored in Tantivy but not as fast field
    TantivyStored(String),
    /// Field requires heap access
    HeapAccess,
}

/// Information about a single field
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub attnum: pg_sys::AttrNumber,
    pub attname: String,
    pub type_oid: pg_sys::Oid,
    pub loading_strategy: FieldLoadingStrategy,
}

/// Maps fields for a single table
#[derive(Debug, Clone)]
pub struct TableFieldMap {
    pub relid: pg_sys::Oid,
    pub relname: String,
    pub fields: HashMap<pg_sys::AttrNumber, FieldInfo>,
    pub fast_field_count: usize,
    pub non_fast_field_count: usize,
}

impl TableFieldMap {
    /// Create a new field map for a relation
    pub unsafe fn new(
        relid: pg_sys::Oid,
        fast_fields: &HashSet<WhichFastField>,
    ) -> Result<Self, String> {
        let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        if heaprel.is_null() {
            return Err(format!("Failed to open relation {}", relid));
        }

        let relname = get_rel_name(relid);
        let tuple_desc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);

        let mut fields = HashMap::new();
        let mut fast_field_count = 0;
        let mut non_fast_field_count = 0;

        // Analyze each column
        for i in 0..tuple_desc.len() {
            if let Some(attribute) = tuple_desc.get(i) {
                let attnum = (i + 1) as pg_sys::AttrNumber;
                let attname = attribute.name().to_string();
                let type_oid = attribute.type_oid().value();

                // Determine loading strategy
                let loading_strategy = determine_loading_strategy(&attname, attnum, fast_fields);

                match &loading_strategy {
                    FieldLoadingStrategy::FastField(_) => fast_field_count += 1,
                    _ => non_fast_field_count += 1,
                }

                fields.insert(
                    attnum,
                    FieldInfo {
                        attnum,
                        attname,
                        type_oid,
                        loading_strategy,
                    },
                );
            }
        }

        pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);

        Ok(Self {
            relid,
            relname,
            fields,
            fast_field_count,
            non_fast_field_count,
        })
    }

    /// Get fields that require heap access
    pub fn get_non_fast_fields(&self) -> Vec<&FieldInfo> {
        self.fields
            .values()
            .filter(|field| !matches!(field.loading_strategy, FieldLoadingStrategy::FastField(_)))
            .collect()
    }

    /// Get fields available from fast fields
    pub fn get_fast_fields(&self) -> Vec<&FieldInfo> {
        self.fields
            .values()
            .filter(|field| matches!(field.loading_strategy, FieldLoadingStrategy::FastField(_)))
            .collect()
    }

    /// Check if a field is a fast field
    pub fn is_fast_field(&self, attnum: pg_sys::AttrNumber) -> bool {
        self.fields
            .get(&attnum)
            .map(|field| matches!(field.loading_strategy, FieldLoadingStrategy::FastField(_)))
            .unwrap_or(false)
    }

    /// Get loading strategy for a field
    pub fn get_loading_strategy(
        &self,
        attnum: pg_sys::AttrNumber,
    ) -> Option<&FieldLoadingStrategy> {
        self.fields
            .get(&attnum)
            .map(|field| &field.loading_strategy)
    }
}

/// Maps fields across multiple tables in a join
#[derive(Debug, Clone)]
pub struct MultiTableFieldMap {
    pub tables: HashMap<pg_sys::Oid, TableFieldMap>,
    pub total_fast_fields: usize,
    pub total_non_fast_fields: usize,
}

impl MultiTableFieldMap {
    /// Create a new multi-table field map
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            total_fast_fields: 0,
            total_non_fast_fields: 0,
        }
    }

    /// Add a table to the map
    pub fn add_table(&mut self, table_map: TableFieldMap) {
        self.total_fast_fields += table_map.fast_field_count;
        self.total_non_fast_fields += table_map.non_fast_field_count;
        self.tables.insert(table_map.relid, table_map);
    }

    /// Get field map for a specific table
    pub fn get_table(&self, relid: pg_sys::Oid) -> Option<&TableFieldMap> {
        self.tables.get(&relid)
    }

    /// Get all non-fast fields across all tables
    pub fn get_all_non_fast_fields(&self) -> Vec<(pg_sys::Oid, &FieldInfo)> {
        let mut fields = Vec::new();
        for (relid, table_map) in &self.tables {
            for field in table_map.get_non_fast_fields() {
                fields.push((*relid, field));
            }
        }
        fields
    }

    /// Check if we should use lazy loading based on field distribution
    pub fn should_use_lazy_loading(&self, limit: Option<f64>) -> bool {
        // Use lazy loading if:
        // 1. We have non-fast fields
        // 2. We have a LIMIT clause
        // 3. The ratio of non-fast to total fields is significant

        if self.total_non_fast_fields == 0 {
            return false;
        }

        if limit.is_none() {
            // Without LIMIT, lazy loading may not provide benefit
            return false;
        }

        let total_fields = self.total_fast_fields + self.total_non_fast_fields;
        let non_fast_ratio = self.total_non_fast_fields as f64 / total_fields as f64;

        // Use lazy loading if more than 30% of fields are non-fast
        non_fast_ratio > 0.3
    }

    /// Log field distribution statistics
    pub fn log_statistics(&self) {
        warning!(
            "ParadeDB: Field distribution - {} fast fields, {} non-fast fields across {} tables",
            self.total_fast_fields,
            self.total_non_fast_fields,
            self.tables.len()
        );

        for (relid, table_map) in &self.tables {
            warning!(
                "ParadeDB: Table {} - {} fast, {} non-fast fields",
                table_map.relname,
                table_map.fast_field_count,
                table_map.non_fast_field_count
            );
        }
    }
}

/// Determine the loading strategy for a field
fn determine_loading_strategy(
    attname: &str,
    attnum: pg_sys::AttrNumber,
    fast_fields: &HashSet<WhichFastField>,
) -> FieldLoadingStrategy {
    // Check if it's a fast field
    for ff in fast_fields {
        match ff {
            WhichFastField::Ctid => {
                if attname == "ctid" {
                    return FieldLoadingStrategy::FastField(WhichFastField::Ctid);
                }
            }
            WhichFastField::TableOid => {
                if attname == "tableoid" {
                    return FieldLoadingStrategy::FastField(WhichFastField::TableOid);
                }
            }
            WhichFastField::Score => {
                if attname.contains("score") {
                    return FieldLoadingStrategy::FastField(WhichFastField::Score);
                }
            }
            WhichFastField::Data { column_name, .. } => {
                if attname == column_name {
                    return FieldLoadingStrategy::FastField(ff.clone());
                }
            }
        }
    }

    // Check if it might be stored in Tantivy (future optimization)
    if is_text_field(attname) {
        // For now, text fields that aren't fast fields need heap access
        // In the future, we could check if they're stored in Tantivy
        FieldLoadingStrategy::HeapAccess
    } else {
        // All other fields require heap access
        FieldLoadingStrategy::HeapAccess
    }
}

/// Check if a field is likely a text field based on name patterns
fn is_text_field(attname: &str) -> bool {
    let text_patterns = ["title", "content", "description", "text", "body", "name"];
    text_patterns
        .iter()
        .any(|pattern| attname.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_classification() {
        let fast_fields = HashSet::from([
            WhichFastField::Ctid,
            WhichFastField::Data {
                column_name: "id".to_string(),
                is_json: false,
            },
        ]);

        // Test fast field detection
        assert!(matches!(
            determine_loading_strategy("ctid", 1, &fast_fields),
            FieldLoadingStrategy::FastField(WhichFastField::Ctid)
        ));

        // Test non-fast field detection
        assert!(matches!(
            determine_loading_strategy("title", 2, &fast_fields),
            FieldLoadingStrategy::HeapAccess
        ));
    }
}
