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

use crate::index::fast_fields_helper::WhichFastField;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub attno: pg_sys::AttrNumber,
    pub field: WhichFastField,
}

/// Information about a scan of a ParadeDB table.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ScanInfo {
    /// The range table index for this scan's base relation.
    pub heap_rti: Option<pg_sys::Index>,
    /// The OID of the heap table.
    pub heaprelid: Option<pg_sys::Oid>,
    /// The OID of the BM25 index (if this scan has one).
    pub indexrelid: Option<pg_sys::Oid>,
    /// The search query for this scan (extracted from WHERE clause predicates).
    /// None if this scan has no BM25 index or no search predicate.
    pub query: Option<SearchQueryInput>,
    /// Whether this scan has a search predicate (uses @@@ operator).
    pub has_search_predicate: bool,
    /// The alias used in the query (e.g., "p" for "products p"), if any.
    pub alias: Option<String>,
    /// Whether scores are needed for this scan's results.
    /// True when ORDER BY paradedb.score() is present for this scan.
    /// Used to optimize FastField executor (skip score computation when not needed).
    pub score_needed: bool,
    /// The fields that need to be extracted from the index.
    /// Populated during planning via `collect_required_fields`.
    #[serde(default)]
    pub fields: Vec<FieldInfo>,
}

impl ScanInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_heap_rti(mut self, rti: pg_sys::Index) -> Self {
        self.heap_rti = Some(rti);
        self
    }

    pub fn with_heaprelid(mut self, oid: pg_sys::Oid) -> Self {
        self.heaprelid = Some(oid);
        self
    }

    pub fn with_indexrelid(mut self, oid: pg_sys::Oid) -> Self {
        self.indexrelid = Some(oid);
        self
    }

    /// Returns true if this scan has a BM25 index.
    pub fn has_bm25_index(&self) -> bool {
        self.indexrelid.is_some()
    }

    pub fn with_query(mut self, query: SearchQueryInput) -> Self {
        self.query = Some(query);
        self.has_search_predicate = true;
        self
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn with_score_needed(mut self, needed: bool) -> Self {
        self.score_needed = needed;
        self
    }

    pub fn add_field(&mut self, attno: pg_sys::AttrNumber, field: WhichFastField) {
        if !self.fields.iter().any(|f| f.attno == attno) {
            self.fields.push(FieldInfo { attno, field });
        }
    }
}
