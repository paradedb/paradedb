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

use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub attno: pg_sys::AttrNumber,
    pub field: WhichFastField,
}

/// Represents the estimated number of rows for a query.
/// `Unknown` is used when the table hasn't been ANALYZEd (reltuples = -1 or 0).
///
/// Sorting: `Unknown` is considered larger than any `Known` estimate.
/// This ensures that when sorting sources by estimate (descending) for partitioning,
/// unknown/large tables are prioritized for partitioning over known small tables.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum RowEstimate {
    /// Known row estimate
    Known(u64),
    /// Unknown - table hasn't been analyzed
    #[default]
    Unknown,
}

impl PartialOrd for RowEstimate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RowEstimate {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (RowEstimate::Known(a), RowEstimate::Known(b)) => a.cmp(b),
            (RowEstimate::Known(_), RowEstimate::Unknown) => Ordering::Less,
            (RowEstimate::Unknown, RowEstimate::Known(_)) => Ordering::Greater,
            (RowEstimate::Unknown, RowEstimate::Unknown) => Ordering::Equal,
        }
    }
}

impl RowEstimate {
    pub fn value(&self) -> u64 {
        match self {
            RowEstimate::Known(v) => *v,
            RowEstimate::Unknown => 0,
        }
    }

    /// The estimate as a float, or `None` when the table hasn't been ANALYZEd. Unlike
    /// [`value`](Self::value), which collapses `Unknown` to `0`, this keeps "no estimate" distinct.
    pub fn known_rows(self) -> Option<f64> {
        match self {
            RowEstimate::Known(rows) => Some(rows as f64),
            RowEstimate::Unknown => None,
        }
    }

    pub fn from_reltuples(reltuples: Option<f64>) -> Self {
        match reltuples {
            Some(r) if r.is_normal() && !r.is_sign_negative() => RowEstimate::Known(r as u64),
            _ => RowEstimate::Unknown,
        }
    }

    /// The row estimate to provide to the DataFusion physical planner.
    ///
    /// It intentionally remains undivided by the number of parallel processes (for MPP) so
    /// DataFusion's optimizer receives the true data scale, preventing it from mistakenly choosing
    /// a broadcast join (CollectLeft) for partitioned parallel plans, leaving the slicing of the
    /// workload up to `datafusion-distributed`.
    pub fn as_planner_estimate(&self) -> u64 {
        match self {
            RowEstimate::Known(n) => *n,
            RowEstimate::Unknown => 1000, // conservative fallback
        }
    }
}

/// Information about a scan of a ParadeDB table.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ScanInfo {
    /// The range table index for this scan's base relation.
    pub heap_rti: pg_sys::Index,
    /// The OID of the heap table.
    pub heaprelid: pg_sys::Oid,
    /// The OID of the BM25 index (if this scan has one).
    pub indexrelid: pg_sys::Oid,
    /// The search query for this scan (extracted from WHERE clause predicates).
    pub query: SearchQueryInput,
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
    pub fields: Vec<FieldInfo>,
    /// The partitioning configuration of the BM25 index, if it was created with `partition_by`.
    pub partition_by: Vec<crate::api::FieldName>,
    /// Estimated number of rows matching the query.
    /// Used to decide which table to partition in parallel joins.
    pub estimate: RowEstimate,
    /// The number of segments in the index.
    pub segment_count: usize,
}

impl ScanInfo {
    pub fn add_field(&mut self, attno: pg_sys::AttrNumber, field: WhichFastField) {
        if !self.fields.iter().any(|f| f.attno == attno) {
            self.fields.push(FieldInfo { attno, field });
        }
    }

    /// Add a field identified by name rather than attno.
    /// Used for JSON sub-fields (e.g., `metadata.category`) which share the
    /// parent column's attno but have distinct Tantivy field names.
    pub fn add_field_by_name(&mut self, attno: pg_sys::AttrNumber, field: WhichFastField) {
        let name = field.name();
        if !self.fields.iter().any(|f| f.field.name() == name) {
            self.fields.push(FieldInfo { attno, field });
        }
    }
}
