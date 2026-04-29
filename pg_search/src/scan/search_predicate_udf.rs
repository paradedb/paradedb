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

//! SearchPredicateUDF - A UDF for search predicate evaluation in DataFusion plans.
//!
//! This UDF represents a search predicate (@@@ operator) inside a DataFusion plan.
//! It carries the Tantivy query and defers execution until DataFusion evaluates it.
//!
//! ## Execution Paths
//!
//! **Pushed-down path** (preferred): `PgSearchTableProvider::supports_filters_pushdown`
//! returns `Exact` and DataFusion passes the filter to `scan()`, which folds it into
//! the Tantivy search.
//!
//! **Direct execution path**: When the predicate sits on a join filter (e.g., a
//! cross-table OR like `p.desc @@@ 'x' OR s.info @@@ 'y'`), DataFusion calls
//! `invoke_with_args` at join time. This runs `execute_search` to materialize matching
//! CTIDs and checks each row via binary search.
//!
//! Both paths can be active simultaneously: for a cross-table OR, DataFusion pushes the
//! per-table arms down to individual scans (reducing rows entering the join), while the
//! full expression also remains as a `HashJoinExec` filter evaluated via `execute_search`.
//!
//! ## Future Work: Optimizing the Direct Execution Path
//!
//! The direct execution path materializes all matching CTIDs upfront. For query patterns
//! that consistently hit this path, we could improve performance with plan rewrites:
//!
//! - **Rewrite as a join**: A DataFusion optimizer rule could detect an unpushed
//!   `SearchPredicateUDF` and convert it into a join against a virtual table that
//!   executes the Tantivy search (similar to correlated subquery decorrelation).
//!
//! - **Semi-join injection**: For existence checks (e.g., `WHERE EXISTS (... @@@ ...)`),
//!   the optimizer could inject a semi-join node.
//!
//! - **Materialized CTE**: Eagerly execute the search into a temporary result set and
//!   reference it as a CTE, avoiding repeated evaluation.

use std::any::Any;
use std::sync::Arc;

use arrow_array::builder::BooleanBuilder;
use arrow_array::{Array, UInt64Array};
use arrow_schema::DataType;
use datafusion::common::{Result, ScalarValue};
use datafusion::logical_expr::{
    ColumnarValue, Expr, ScalarUDF, ScalarUDFImpl, Signature, Volatility,
};
use pgrx::pg_sys;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tantivy::index::SegmentId;

use crate::query::SearchQueryInput;

/// The name of our search predicate UDF - used for recognition in filter pushdown
pub const SEARCH_PREDICATE_UDF_NAME: &str = "pdb_search_predicate";

/// User Defined Function that marks a search predicate for lazy evaluation.
///
/// This UDF carries the search query information and is designed to be pushed down
/// to `PgSearchTableProvider` via DataFusion's filter pushdown mechanism.
///
/// Arguments:
/// - ctid_col: UInt64 - the CTID column to filter
/// - index_oid: UInt32 - OID of the BM25 index
/// - heap_oid: UInt32 - OID of the heap table
/// - query_json: Utf8 - JSON serialization of SearchQueryInput
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPredicateUDF {
    /// OID of the BM25 index
    pub index_oid: pg_sys::Oid,
    /// OID of the heap table
    pub heap_oid: pg_sys::Oid,
    /// The search query as JSON (stored as string for Hash/Eq derivation)
    query_json: String,
    /// Human-readable representation of the original PostgreSQL expression (for EXPLAIN output).
    display_string: String,
    /// When true, `execute_search` produces packed DocAddresses.
    /// When false, it produces real ctids with visibility checking.
    #[serde(default)]
    deferred_visibility: bool,
    /// Position of the source this UDF targets in the join's `sources()` ordering.
    /// Used by the deserialization codec to inject the correct canonical segment
    /// IDs in self-join scenarios where multiple scans share the same
    /// `index_oid` but have different segment sets (partitioned vs replicated).
    #[serde(default)]
    plan_position: Option<usize>,
    /// Canonical segment IDs injected during deserialization for execution-time
    /// UDF evaluation. Skipped during serialization because they are backend-local.
    #[serde(skip)]
    canonical_segment_ids: Option<crate::api::HashSet<SegmentId>>,
    #[serde(skip, default = "SearchPredicateUDF::make_signature")]
    signature: Signature,
}

// Manual impls exclude runtime-only fields (signature)
impl PartialEq for SearchPredicateUDF {
    fn eq(&self, other: &Self) -> bool {
        self.index_oid == other.index_oid
            && self.heap_oid == other.heap_oid
            && self.query_json == other.query_json
            && self.display_string == other.display_string
            && self.deferred_visibility == other.deferred_visibility
            && self.plan_position == other.plan_position
    }
}

impl Eq for SearchPredicateUDF {}

impl std::hash::Hash for SearchPredicateUDF {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index_oid.hash(state);
        self.heap_oid.hash(state);
        self.query_json.hash(state);
        self.display_string.hash(state);
        self.deferred_visibility.hash(state);
        self.plan_position.hash(state);
    }
}

impl SearchPredicateUDF {
    #[allow(dead_code)]
    pub fn new(
        index_oid: pg_sys::Oid,
        heap_oid: pg_sys::Oid,
        query: SearchQueryInput,
        display_string: String,
    ) -> Self {
        Self::with_deferred_visibility(index_oid, heap_oid, query, display_string, false, None)
    }

    pub fn with_deferred_visibility(
        index_oid: pg_sys::Oid,
        heap_oid: pg_sys::Oid,
        query: SearchQueryInput,
        display_string: String,
        deferred_visibility: bool,
        plan_position: Option<usize>,
    ) -> Self {
        let query_json =
            serde_json::to_string(&query).expect("SearchQueryInput should be serializable");
        Self {
            index_oid,
            heap_oid,
            query_json,
            display_string,
            deferred_visibility,
            plan_position,
            canonical_segment_ids: None,
            signature: Self::make_signature(),
        }
    }

    /// Get a human-readable display string for EXPLAIN output.
    /// This was eagerly computed during planning via `deparse_expr`.
    pub fn display(&self) -> String {
        // Strip dynamic OID values for deterministic output
        strip_oids(&self.display_string)
    }

    fn make_signature() -> Signature {
        // Takes ctid column (UInt64)
        Signature::exact(vec![DataType::UInt64], Volatility::Immutable)
    }

    /// Get the search query by deserializing from JSON
    pub fn query(&self) -> SearchQueryInput {
        serde_json::from_str(&self.query_json).expect("query_json should be valid SearchQueryInput")
    }

    pub(crate) fn plan_position(&self) -> Option<usize> {
        self.plan_position
    }

    pub(crate) fn set_canonical_segment_ids(&mut self, ids: crate::api::HashSet<SegmentId>) {
        self.canonical_segment_ids = Some(ids);
    }

    /// Create a DataFusion expression that calls this UDF
    pub fn into_expr(self, ctid_col: Expr) -> Expr {
        let udf = ScalarUDF::new_from_impl(self);
        udf.call(vec![ctid_col])
    }

    /// Try to extract SearchPredicateUDF from a DataFusion Expr
    pub fn try_from_expr(expr: &Expr) -> Option<&Self> {
        match expr {
            Expr::ScalarFunction(func) => {
                if func.func.name() == SEARCH_PREDICATE_UDF_NAME {
                    func.func
                        .inner()
                        .as_any()
                        .downcast_ref::<SearchPredicateUDF>()
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for SearchPredicateUDF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl ScalarUDFImpl for SearchPredicateUDF {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        SEARCH_PREDICATE_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke_with_args(
        &self,
        args: datafusion::logical_expr::ScalarFunctionArgs,
    ) -> Result<ColumnarValue> {
        // Direct execution path: DataFusion did not push this filter down, so we execute
        // the Tantivy search here and match CTIDs against the input batch.
        // This happens for cross-table predicates (e.g., OR conditions spanning multiple
        // tables) that cannot be pushed to individual table scans.
        // See module-level docs for future optimization strategies.

        let arg = &args.args[0];
        let matching_ctids = self.execute_search()?;

        match arg {
            ColumnarValue::Array(array) => {
                let ctids = array
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("Expected UInt64Array for ctid");
                let mut builder = BooleanBuilder::with_capacity(ctids.len());
                // Binary search per row: batch CTIDs are not necessarily sorted after
                // a join (the join strategy may reorder rows), so a merge-join cursor
                // would skip valid matches. Binary search is O(n log m) and correct
                // regardless of input order.
                for i in 0..ctids.len() {
                    if ctids.is_null(i) {
                        builder.append_null();
                    } else {
                        let ctid = ctids.value(i);
                        builder.append_value(matching_ctids.binary_search(&ctid).is_ok());
                    }
                }
                Ok(ColumnarValue::Array(Arc::new(builder.finish())))
            }
            ColumnarValue::Scalar(scalar) => match scalar {
                ScalarValue::UInt64(Some(ctid)) => {
                    let is_present = matching_ctids.binary_search(ctid).is_ok();
                    Ok(ColumnarValue::Scalar(ScalarValue::Boolean(Some(
                        is_present,
                    ))))
                }
                _ => Ok(ColumnarValue::Scalar(ScalarValue::Boolean(None))),
            },
        }
    }
}

impl SearchPredicateUDF {
    /// Execute the search and return sorted matching values.
    ///
    /// This is the direct execution path used when the filter is not pushed down
    /// (e.g., cross-table OR predicates). It materializes all matching values upfront
    /// for binary-search filtering against incoming batches.
    ///
    /// When `deferred_visibility` is false, returns real ctids with visibility checking.
    /// When `deferred_visibility` is true, returns packed DocAddresses (segment_ord << 32 | doc_id)
    /// without visibility checking — the caller (VisibilityFilterExec) will handle visibility later.
    fn execute_search(&self) -> Result<Vec<u64>> {
        use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
        use crate::index::mvcc::MvccSatisfies;
        use crate::index::reader::index::SearchIndexReader;
        use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
        use crate::postgres::rel::PgSearchRelation;
        use crate::scan::Scanner;

        let index_rel = PgSearchRelation::open(self.index_oid);
        let heap_rel = PgSearchRelation::open(self.heap_oid);
        let query = self.query();

        // Use canonical segment IDs when available to ensure the same segment
        // set and ordering as the plan's table providers. This is critical for
        // deferred visibility where packed DocAddresses must be comparable.
        let mvcc_style = if let Some(ids) = self.canonical_segment_ids.clone() {
            MvccSatisfies::ParallelWorker(ids)
        } else {
            MvccSatisfies::Snapshot
        };

        let needs_tokenizer_manager = query.needs_tokenizer();
        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query,
            false, // score_needed
            mvcc_style,
            None,
            None,
            needs_tokenizer_manager,
        )
        .map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!("Failed to open reader: {e}"))
        })?;

        let search_results = reader.search();

        let fields = if self.deferred_visibility {
            // Deferred visibility: emit packed DocAddresses without heap access.
            // The incoming batch also has packed DocAddresses because the UDF runs
            // below VisibilityFilterExec (inner join case).
            let ctid_alias = format!("ctid_udf_{}", self.heap_oid.to_u32());
            vec![WhichFastField::DeferredCtid(ctid_alias)]
        } else {
            vec![WhichFastField::Ctid]
        };

        let ffhelper = FFHelper::with_fields(&reader, &fields);
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let mut visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);
        let mut scanner = Scanner::new(search_results, None, fields, self.heap_oid.into());

        let mut values = Vec::new();
        while let Some(batch) = scanner.next(&ffhelper, &mut visibility, None) {
            if let Some(Some(col)) = batch.fields.first() {
                let array = col
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("UDF ctid/DeferredCtid should be UInt64Array");
                values.extend(array.values());
            }
        }
        values.sort_unstable();
        values.dedup();
        Ok(values)
    }
}

/// Strip dynamic OID values from expression strings for deterministic output.
/// Removes patterns like `"oid":12345,` from JSON in the expression.
fn strip_oids(s: &str) -> String {
    // Match `"oid":NUMBER,` pattern
    let re = Regex::new(r#""oid":\d+,"#).unwrap();
    re.replace_all(s, "").to_string()
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use datafusion::logical_expr::col;
    use pgrx::prelude::*;

    #[pg_test]
    fn test_udf_name() {
        let udf = SearchPredicateUDF::new(
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            SearchQueryInput::All,
            "test display".to_string(),
        );
        assert_eq!(udf.name(), SEARCH_PREDICATE_UDF_NAME);
    }

    #[pg_test]
    fn test_into_expr() {
        let udf = SearchPredicateUDF::new(
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            SearchQueryInput::All,
            "test display".to_string(),
        );
        let expr = udf.into_expr(col("ctid"));

        // Verify it's a scalar function
        assert!(matches!(expr, Expr::ScalarFunction(_)));
    }

    #[pg_test]
    fn test_try_from_expr() {
        let udf = SearchPredicateUDF::new(
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            SearchQueryInput::All,
            "test display".to_string(),
        );
        let expr = udf.clone().into_expr(col("ctid"));

        let extracted = SearchPredicateUDF::try_from_expr(&expr);
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().query(), SearchQueryInput::All);
    }

    #[pg_test]
    fn test_try_from_expr_not_our_udf() {
        // Regular column expression - should return None
        let expr = col("some_column");
        assert!(SearchPredicateUDF::try_from_expr(&expr).is_none());
    }
}
