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

use crate::api::{FieldName, MvccVisibility};
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

/// 0-based index of a tag within a single table scan.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TagIndex(pub usize);

/// Index into the global `join_level_predicates` vector.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GlobalPredicateIndex(pub usize);

/// A tagged search predicate on a scan with a dedicated match tag column name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaggedQuery {
    /// The synthetic column name (e.g. `__users_tag_0`).
    pub tag_name: String,
    /// The 0-based tag index within this scan/table.
    pub tag_idx: TagIndex,
    /// The global predicate index in `join_level_predicates`.
    pub predicate_idx: GlobalPredicateIndex,
    /// The Tantivy search query for this tag.
    pub query: Box<SearchQueryInput>,
}

/// Execution mode for a scan, encapsulating its search query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanMode {
    /// Standard scan executing its own query.
    Standard { query: Box<SearchQueryInput> },
    /// Scan participating in predicate tagging (e.g. in a disjunctive search join).
    Tagged {
        /// Base query (e.g. non-search restrictions like `w.year >= 2024`, or `All`).
        base_query: Box<SearchQueryInput>,
        /// The single-table search predicates lifted from this scan, if any.
        local_queries: Vec<TaggedQuery>,
    },
}

impl ScanMode {
    /// Creates a standard scan mode with the given query.
    pub fn standard(query: SearchQueryInput) -> Self {
        Self::Standard {
            query: Box::new(query),
        }
    }

    /// Creates a standard scan mode matching all documents.
    pub fn all() -> Self {
        Self::standard(SearchQueryInput::All)
    }

    /// Creates a tagged scan mode with a base query and tagged search queries.
    pub fn tagged(base_query: SearchQueryInput, local_queries: Vec<TaggedQuery>) -> Self {
        Self::Tagged {
            base_query: Box::new(base_query),
            local_queries,
        }
    }

    /// Returns a new `ScanMode` with the base or standard query replaced.
    pub fn with_base_query(self, base_query: SearchQueryInput) -> Self {
        match self {
            Self::Standard { .. } => Self::Standard {
                query: Box::new(base_query),
            },
            Self::Tagged { local_queries, .. } => Self::Tagged {
                base_query: Box::new(base_query),
                local_queries,
            },
        }
    }

    /// The base or primary search query for this scan mode.
    pub fn query(&self) -> &SearchQueryInput {
        match self {
            Self::Standard { query } => query,
            Self::Tagged { base_query, .. } => base_query,
        }
    }

    /// Whether any query in this scan mode requires a tokenizer.
    pub fn needs_tokenizer(&self) -> bool {
        match self {
            Self::Standard { query } => query.needs_tokenizer(),
            Self::Tagged {
                base_query,
                local_queries,
            } => {
                base_query.needs_tokenizer()
                    || local_queries.iter().any(|tq| tq.query.needs_tokenizer())
            }
        }
    }
}

/// Information about a scan of a ParadeDB table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanInfo {
    /// The range table index for this scan's base relation.
    pub heap_rti: pg_sys::Index,
    /// The OID of the heap table.
    pub heaprelid: pg_sys::Oid,
    /// The OID of the BM25 index (if this scan has one).
    pub indexrelid: pg_sys::Oid,
    /// Whether this scan has a search predicate (uses @@@ operator).
    pub has_search_predicate: bool,
    /// The execution mode and query for this scan.
    pub mode: ScanMode,
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
    pub partition_by: Vec<FieldName>,
    /// Estimated number of rows matching the query.
    /// Used to decide which table to partition in parallel joins.
    pub estimate: RowEstimate,
    /// True when `estimate` is the index's total document count rather than a match count.
    /// The estimator substitutes the total when the query carries a Postgres expression or a
    /// heap filter it cannot evaluate at plan time; consumers that need a match count (the MPP
    /// size gate) discount it by `PARAMETERIZED_SELECTIVITY`, mirroring BaseScan.
    #[serde(default)]
    pub estimate_from_total_docs: bool,
    /// The number of segments in the index.
    pub segment_count: usize,
    /// Whether rows are checked against the heap for snapshot visibility. Only a
    /// `pdb.agg()` query can ask for anything but the default.
    #[serde(default)]
    pub mvcc_visibility: MvccVisibility,
}

impl ScanInfo {
    pub fn new(
        heap_rti: pg_sys::Index,
        heaprelid: pg_sys::Oid,
        indexrelid: pg_sys::Oid,
        mode: ScanMode,
    ) -> Self {
        Self {
            heap_rti,
            heaprelid,
            indexrelid,
            has_search_predicate: false,
            mode,
            alias: None,
            score_needed: false,
            fields: Vec::new(),
            partition_by: Vec::new(),
            estimate: RowEstimate::Unknown,
            estimate_from_total_docs: false,
            segment_count: 0,
            mvcc_visibility: MvccVisibility::default(),
        }
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    pub fn with_search_predicate(mut self, has_search_predicate: bool) -> Self {
        self.has_search_predicate = has_search_predicate;
        self
    }

    pub fn with_score_needed(mut self, score_needed: bool) -> Self {
        self.score_needed = score_needed;
        self
    }
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
