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

//! Shared encoding for the scalar UDFs `pg_search` puts into DataFusion plans.
//!
//! Both the logical codec ([`crate::scan::codec`]) and the physical codec
//! ([`crate::scan::physical_codec`]) have to move `SearchPredicateUDF` and
//! `PgExprUdf` across a serialization boundary, in exactly the same way. The two
//! codecs differ only in what they do with a UDF that is *not* one of ours, so
//! these helpers report "not mine" rather than deciding, and each codec applies
//! its own fallback.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::logical_expr::ScalarUDF;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::pg_expr_udf::{PG_EXPR_UDF_PREFIX, PgExprUdf};
use crate::scan::search_predicate_udf::SearchPredicateUDF;

/// Whether `name` identifies a scalar UDF that `pg_search` owns the encoding for.
///
/// This is the single definition of that predicate; the decode/encode helpers
/// below and the composed physical codec all agree by construction.
pub(crate) fn is_pg_search_udf(name: &str) -> bool {
    name == "pdb_search_predicate" || name.starts_with(PG_EXPR_UDF_PREFIX)
}

/// Decode a `pg_search` scalar UDF.
///
/// Returns `Ok(None)` when `name` does not belong to `pg_search`, leaving the
/// caller to decide whether that is an error or should fall through to the
/// session registry.
///
/// `index_segment_ids` supplies the canonical segment IDs to re-attach to a
/// `SearchPredicateUDF`; pass an empty slice when the caller has none.
pub(crate) fn try_decode_pg_search_udf(
    name: &str,
    buf: &[u8],
    index_segment_ids: &[HashSet<SegmentId>],
) -> Result<Option<Arc<ScalarUDF>>> {
    if name == "pdb_search_predicate" {
        let mut udf: SearchPredicateUDF = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize SearchPredicateUDF: {e}"))
        })?;
        if let Some(plan_position) = udf.plan_position()
            && !index_segment_ids.is_empty()
        {
            let ids = index_segment_ids
                .get(plan_position)
                .cloned()
                .ok_or_else(|| {
                    DataFusionError::Internal(format!(
                        "missing canonical segment IDs for plan_position {plan_position}"
                    ))
                })?;
            udf.set_canonical_segment_ids(ids);
        }
        return Ok(Some(Arc::new(ScalarUDF::new_from_impl(udf))));
    }

    if name.starts_with(PG_EXPR_UDF_PREFIX) {
        let mut udf: PgExprUdf = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize PgExprUdf: {e}"))
        })?;
        udf.fixup_after_deserialize();
        return Ok(Some(Arc::new(ScalarUDF::new_from_impl(udf))));
    }

    Ok(None)
}

/// Encode a `pg_search` scalar UDF into `buf`.
///
/// Returns `Ok(false)` when `node` does not belong to `pg_search`, in which case
/// nothing has been written to `buf` and the caller decides the fallback.
pub(crate) fn try_encode_pg_search_udf(node: &ScalarUDF, buf: &mut Vec<u8>) -> Result<bool> {
    let name = node.name();

    if name == "pdb_search_predicate" {
        let udf = node
            .inner()
            .downcast_ref::<SearchPredicateUDF>()
            .ok_or_else(|| DataFusionError::Internal("UDF is not a SearchPredicateUDF".into()))?;
        let bytes = serde_json::to_vec(udf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to serialize SearchPredicateUDF: {e}"))
        })?;
        buf.extend_from_slice(&bytes);
        return Ok(true);
    }

    if name.starts_with(PG_EXPR_UDF_PREFIX) {
        let udf = node
            .inner()
            .downcast_ref::<PgExprUdf>()
            .ok_or_else(|| DataFusionError::Internal("UDF is not a PgExprUdf".into()))?;
        let bytes = serde_json::to_vec(udf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to serialize PgExprUdf: {e}"))
        })?;
        buf.extend_from_slice(&bytes);
        return Ok(true);
    }

    Ok(false)
}
