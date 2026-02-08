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

//! SearchPredicateUDF - A marker UDF for lazy search predicate evaluation.
//!
//! This UDF acts as a marker in DataFusion expressions to represent a search predicate
//! that should be evaluated by the underlying `PgSearchTableProvider`. Unlike `RowInSetUDF`
//! which pre-computes matching CTIDs, this UDF carries the search query information and
//! defers execution to the `scan()` method via DataFusion's filter pushdown mechanism.
//!
//! Flow:
//! 1. Translator creates `search_predicate(ctid_col, index_oid, heap_oid, query_json)` expression
//! 2. DataFusion calls `supports_filters_pushdown` - we return `Exact` for our UDF
//! 3. DataFusion passes the filter to `scan()`
//! 4. `scan()` extracts the query and adds it to the Tantivy search

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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::RawPtr;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SearchPredicateUDF {
    /// OID of the BM25 index
    pub index_oid: pg_sys::Oid,
    /// OID of the heap table
    pub heap_oid: pg_sys::Oid,
    /// The search query as JSON (stored as string for Hash/Eq derivation)
    query_json: String,
    /// Raw pointer to the original PostgreSQL expression (for lazy deparse).
    /// Only valid within the same query execution.
    expr_ptr: RawPtr<pg_sys::Node>,
    /// Raw pointer to PlannerInfo (for lazy deparse context).
    planner_info_ptr: RawPtr<pg_sys::PlannerInfo>,
    #[serde(skip, default = "SearchPredicateUDF::make_signature")]
    signature: Signature,
}

impl SearchPredicateUDF {
    pub fn new(
        index_oid: pg_sys::Oid,
        heap_oid: pg_sys::Oid,
        query: SearchQueryInput,
        expr_ptr: *mut pg_sys::Node,
        planner_info_ptr: *mut pg_sys::PlannerInfo,
    ) -> Self {
        let query_json =
            serde_json::to_string(&query).expect("SearchQueryInput should be serializable");
        Self {
            index_oid,
            heap_oid,
            query_json,
            expr_ptr: RawPtr::new(expr_ptr),
            planner_info_ptr: RawPtr::new(planner_info_ptr),
            signature: Self::make_signature(),
        }
    }

    /// Get a human-readable display string for EXPLAIN output (computed lazily).
    /// Uses deparse_expr with the stored expression pointer.
    pub fn display(&self) -> String {
        use crate::postgres::customscan::qual_inspect::PlannerContext;
        use crate::postgres::deparse::deparse_expr;

        // expr_ptr should always be available
        assert!(
            !self.expr_ptr.is_null(),
            "SearchPredicateUDF::display() requires expr_ptr to be set"
        );

        let expr = self.expr_ptr.as_ptr();
        let index_rel = PgSearchRelation::open(self.index_oid);

        let context = if !self.planner_info_ptr.is_null() {
            let root = self.planner_info_ptr.as_ptr();
            Some(PlannerContext::from_planner(root))
        } else {
            None
        };

        let deparsed = unsafe { deparse_expr(context.as_ref(), &index_rel, expr) };
        // Strip dynamic OID values for deterministic output
        strip_oids(&deparsed)
    }

    fn make_signature() -> Signature {
        // Takes ctid column (UInt64)
        Signature::exact(vec![DataType::UInt64], Volatility::Immutable)
    }

    /// Get the search query by deserializing from JSON
    pub fn query(&self) -> SearchQueryInput {
        serde_json::from_str(&self.query_json).expect("query_json should be valid SearchQueryInput")
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
        // This UDF is designed to be pushed down to PgSearchTableProvider when possible.
        // If we reach here, it means the filter wasn't pushed down (e.g., cross-table OR
        // predicates) and we need to evaluate it manually.
        //
        // In the pushed-down path, the filter is converted to a Tantivy query
        // and only matching rows are returned from scan().
        //
        // In this fallback path, we execute the search and check CTIDs against the results.
        // This is used for cross-table predicates that cannot be pushed to individual tables.

        let arg = &args.args[0];

        // Execute the search to get matching CTIDs
        let matching_ctids = self.execute_search()?;

        match arg {
            ColumnarValue::Array(array) => {
                let ctids = array
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("Expected UInt64Array for ctid");
                let mut builder = BooleanBuilder::with_capacity(ctids.len());
                for i in 0..ctids.len() {
                    if ctids.is_null(i) {
                        builder.append_null();
                    } else {
                        let ctid = ctids.value(i);
                        // Binary search since matching_ctids is sorted
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
    /// Execute the search and return sorted matching CTIDs.
    /// This is the fallback path when the filter isn't pushed down.
    ///
    /// This is used for cross-table predicates (e.g., OR conditions between tables)
    /// which cannot be pushed down to individual table scans.
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

        let reader = SearchIndexReader::open_with_context(
            &index_rel,
            query,
            false, // score_needed
            MvccSatisfies::Snapshot,
            None,
            None,
        )
        .map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!("Failed to open reader: {e}"))
        })?;

        let search_results = reader.search();
        let fields = vec![WhichFastField::Ctid];
        let ffhelper = FFHelper::with_fields(&reader, &fields);

        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let mut visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let mut scanner = Scanner::new(search_results, None, fields, self.heap_oid.into());

        let mut ctids = Vec::new();
        while let Some(batch) = scanner.next(&ffhelper, &mut visibility) {
            if let Some(Some(col)) = batch.fields.first() {
                let array = col
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("Ctid should be UInt64Array");
                ctids.extend(array.values());
            }
        }

        ctids.sort_unstable();
        ctids.dedup();
        Ok(ctids)
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
    use std::ptr;

    #[pg_test]
    fn test_udf_name() {
        let udf = SearchPredicateUDF::new(
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            SearchQueryInput::All,
            ptr::null_mut(), // no expr_ptr in tests
            ptr::null_mut(), // no planner_info_ptr in tests
        );
        assert_eq!(udf.name(), SEARCH_PREDICATE_UDF_NAME);
    }

    #[pg_test]
    fn test_into_expr() {
        let udf = SearchPredicateUDF::new(
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            SearchQueryInput::All,
            ptr::null_mut(),
            ptr::null_mut(),
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
            ptr::null_mut(),
            ptr::null_mut(),
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
