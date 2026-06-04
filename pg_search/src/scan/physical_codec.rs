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

//! Physical-plan codec for coordinator-dispatched MPP fragments.
//!
//! The leader builds the distributed physical plan once and ships each stage's subplan to the
//! workers; the workers run their fragments without re-planning. DataFusion's `Network*Exec`
//! boundaries are serialized by the fork's [`DistributedCodec`]; this codec handles the
//! `pg_search` custom execs that sit inside a stage. It mirrors the LOGICAL
//! [`crate::scan::codec`] (same UDF/UDAF handling, same per-source segment-ID injection) but
//! travels post-optimization physical nodes instead of a logical plan.

use std::sync::Arc;

use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::functions_aggregate as dfa;
use datafusion::logical_expr::{AggregateUDF, ScalarUDF};
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::DistributedCodec;
use datafusion_proto::physical_plan::{
    AsExecutionPlan, ComposedPhysicalExtensionCodec, PhysicalExtensionCodec,
};
use datafusion_proto::protobuf::PhysicalPlanNode;
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::pg_expr_udf::{PgExprUdf, PG_EXPR_UDF_PREFIX};
use crate::postgres::ParallelScanState;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::search_predicate_udf::SearchPredicateUDF;

/// Byte tag for `PgSearchScanPlan` in the extension payload. Kept even though the composed
/// codec already records which codec decoded a node, so a future second custom exec
/// (`FilterPassthroughExec`, `VisibilityFilterExec`, ...) can share this codec by tag.
const TAG_PG_SEARCH_SCAN: u8 = 2;

/// [`PhysicalExtensionCodec`] for the `pg_search` custom execs, carrying the runtime context a
/// worker needs to rebuild them. Encode is context-free (the leader serializes the recipe);
/// decode injects the worker's own `ParallelScanState` and frozen per-source segment sets.
#[derive(Debug, Default)]
pub struct PgSearchPhysicalExtensionCodec {
    /// Worker's `ParallelScanState`, used to resolve the scan's MVCC segment set and to claim
    /// segments at runtime.
    parallel_state: Option<*mut ParallelScanState>,
    /// Canonical segment ID sets for non-partitioning sources, indexed by position in the
    /// non-partitioning source list. Mirrors the logical codec.
    non_partitioning_segment_ids: Vec<HashSet<SegmentId>>,
    /// Canonical segment ID sets for all join sources, indexed by `plan_position`. Injected into
    /// `SearchPredicateUDF` on decode, same as the logical codec.
    index_segment_ids: Vec<HashSet<SegmentId>>,
}

// Same justification as the logical `PgSearchExtensionCodec`: Postgres extensions run
// single-threaded, so the raw `ParallelScanState` pointer never crosses a real thread boundary.
unsafe impl Send for PgSearchPhysicalExtensionCodec {}
unsafe impl Sync for PgSearchPhysicalExtensionCodec {}

impl PhysicalExtensionCodec for PgSearchPhysicalExtensionCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        _inputs: &[Arc<dyn ExecutionPlan>],
        _ctx: &TaskContext,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let Some((&tag, payload)) = buf.split_first() else {
            return Err(DataFusionError::Internal(
                "PgSearchPhysicalExtensionCodec: empty buffer".into(),
            ));
        };
        match tag {
            TAG_PG_SEARCH_SCAN => PgSearchScanPlan::decode_for_dispatch(
                payload,
                self.parallel_state,
                &self.non_partitioning_segment_ids,
            ),
            other => Err(DataFusionError::NotImplemented(format!(
                "PgSearchPhysicalExtensionCodec: unknown physical node tag {other}"
            ))),
        }
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        if let Some(scan) = node.as_any().downcast_ref::<PgSearchScanPlan>() {
            buf.push(TAG_PG_SEARCH_SCAN);
            buf.extend_from_slice(&scan.encode_for_dispatch()?);
            return Ok(());
        }
        Err(DataFusionError::NotImplemented(format!(
            "PgSearchPhysicalExtensionCodec: no physical encoding for {}",
            node.name()
        )))
    }

    fn try_decode_udaf(&self, name: &str, _buf: &[u8]) -> Result<Arc<AggregateUDF>> {
        match name {
            "min" => Ok(dfa::min_max::min_udaf()),
            "max" => Ok(dfa::min_max::max_udaf()),
            "count" => Ok(dfa::count::count_udaf()),
            "sum" => Ok(dfa::sum::sum_udaf()),
            "avg" => Ok(dfa::average::avg_udaf()),
            _ => Err(DataFusionError::NotImplemented(format!(
                "PhysicalExtensionCodec is not provided for aggregate function {name}"
            ))),
        }
    }

    fn try_encode_udaf(&self, node: &AggregateUDF, buf: &mut Vec<u8>) -> Result<()> {
        buf.extend_from_slice(node.name().as_bytes());
        Ok(())
    }

    fn try_decode_udf(&self, name: &str, buf: &[u8]) -> Result<Arc<ScalarUDF>> {
        if name == "pdb_search_predicate" {
            let mut udf: SearchPredicateUDF = serde_json::from_slice(buf).map_err(|e| {
                DataFusionError::Internal(format!("Failed to deserialize SearchPredicateUDF: {e}"))
            })?;
            if let Some(plan_position) = udf.plan_position() {
                if !self.index_segment_ids.is_empty() {
                    let ids = self
                        .index_segment_ids
                        .get(plan_position)
                        .cloned()
                        .expect("missing canonical segment IDs for plan_position");
                    udf.set_canonical_segment_ids(ids);
                }
            }
            return Ok(Arc::new(ScalarUDF::new_from_impl(udf)));
        }

        if name.starts_with(PG_EXPR_UDF_PREFIX) {
            let mut udf: PgExprUdf = serde_json::from_slice(buf).map_err(|e| {
                DataFusionError::Internal(format!("Failed to deserialize PgExprUdf: {e}"))
            })?;
            udf.fixup_after_deserialize();
            return Ok(Arc::new(ScalarUDF::new_from_impl(udf)));
        }

        Err(DataFusionError::NotImplemented(format!(
            "UDF '{name}' deserialization not implemented"
        )))
    }

    fn try_encode_udf(&self, node: &ScalarUDF, buf: &mut Vec<u8>) -> Result<()> {
        let name = node.name();
        if name == "pdb_search_predicate" {
            let udf = node
                .inner()
                .as_any()
                .downcast_ref::<SearchPredicateUDF>()
                .ok_or_else(|| DataFusionError::Internal("UDF is not a SearchPredicateUDF".into()))?;
            let bytes = serde_json::to_vec(udf).map_err(|e| {
                DataFusionError::Internal(format!("Failed to serialize SearchPredicateUDF: {e}"))
            })?;
            buf.extend_from_slice(&bytes);
            return Ok(());
        }

        if name.starts_with(PG_EXPR_UDF_PREFIX) {
            let udf = node
                .inner()
                .as_any()
                .downcast_ref::<PgExprUdf>()
                .ok_or_else(|| DataFusionError::Internal("UDF is not a PgExprUdf".into()))?;
            let bytes = serde_json::to_vec(udf).map_err(|e| {
                DataFusionError::Internal(format!("Failed to serialize PgExprUdf: {e}"))
            })?;
            buf.extend_from_slice(&bytes);
            return Ok(());
        }

        Err(DataFusionError::NotImplemented(format!(
            "UDF '{name}' serialization not implemented"
        )))
    }
}

/// Compose the fork's [`DistributedCodec`] (handles `Network*Exec` / `StageExec`) with the
/// `pg_search` codec. Encode and decode MUST build the same list in the same order: the composed
/// codec records each node's codec index on the wire.
fn combined_codec(user: PgSearchPhysicalExtensionCodec) -> ComposedPhysicalExtensionCodec {
    ComposedPhysicalExtensionCodec::new(vec![Arc::new(DistributedCodec {}), Arc::new(user)])
}

/// Serialize one stage's physical subplan for dispatch. Context-free: only the recipe travels,
/// the receiving worker injects its own runtime state on decode.
pub fn serialize_physical_plan(plan: Arc<dyn ExecutionPlan>) -> Result<Vec<u8>> {
    let codec = combined_codec(PgSearchPhysicalExtensionCodec::default());
    let proto = PhysicalPlanNode::try_from_physical_plan(plan, &codec)?;
    Ok(prost::Message::encode_to_vec(&proto))
}

/// Deserialize a dispatched physical subplan, injecting the worker's runtime context so the
/// `PgSearchScanPlan` leaves rebuild their readers under this worker's MVCC view.
pub fn deserialize_physical_plan_with_runtime(
    bytes: &[u8],
    ctx: &TaskContext,
    parallel_state: Option<*mut ParallelScanState>,
    non_partitioning_segment_ids: Vec<HashSet<SegmentId>>,
    index_segment_ids: Vec<HashSet<SegmentId>>,
) -> Result<Arc<dyn ExecutionPlan>> {
    let codec = combined_codec(PgSearchPhysicalExtensionCodec {
        parallel_state,
        non_partitioning_segment_ids,
        index_segment_ids,
    });
    let proto = <PhysicalPlanNode as prost::Message>::decode(bytes).map_err(|e| {
        DataFusionError::Internal(format!("Failed to decode dispatched PhysicalPlanNode: {e}"))
    })?;
    proto.try_into_physical_plan(ctx, &codec)
}
