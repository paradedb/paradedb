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

//! Physical-plan codec for leader-dispatched MPP fragments.
//!
//! The leader builds the distributed physical plan once and ships each stage's subplan to the
//! workers; the workers run their fragments without re-planning. DataFusion's `Network*Exec`
//! boundaries are serialized by the fork's [`DistributedCodec`]; this codec handles the
//! `pg_search` custom execs that sit inside a stage. It mirrors the LOGICAL
//! [`crate::scan::codec`] (same UDF/UDAF handling, same per-source segment-ID injection) but
//! travels post-optimization physical nodes instead of a logical plan.

use std::sync::Arc;

use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::logical_expr::ScalarUDF;
use datafusion::physical_optimizer::filter_pushdown::FilterPushdown;
use datafusion::physical_optimizer::PhysicalOptimizerRule;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_distributed::DistributedCodec;
use datafusion_proto::physical_plan::{
    AsExecutionPlan, ComposedPhysicalExtensionCodec, PhysicalExtensionCodec,
};
use datafusion_proto::protobuf::PhysicalPlanNode;
use tantivy::index::SegmentId;

use crate::api::{HashMap, HashSet};
use crate::index::fast_fields_helper::FFHelper;
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
use crate::postgres::customscan::pg_expr_udf::{PgExprUdf, PG_EXPR_UDF_PREFIX};
use crate::postgres::ParallelScanState;
use crate::scan::execution_plan::PgSearchScanPlan;
use crate::scan::filter_passthrough_exec::FilterPassthroughExec;
use crate::scan::search_predicate_udf::SearchPredicateUDF;
use crate::scan::segmented_topk_exec::SegmentedTopKExec;
use crate::scan::tantivy_lookup_exec::TantivyLookupExec;

/// Byte tags identifying each custom exec in the extension payload. The composed codec already
/// records which codec decoded a node; the tag picks the exec within this codec.
const TAG_PG_SEARCH_SCAN: u8 = 2;
const TAG_VISIBILITY_FILTER: u8 = 4;
const TAG_TANTIVY_LOOKUP: u8 = 5;
const TAG_SEGMENTED_TOPK: u8 = 6;

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
    /// The `ExprContext` workers use to evaluate heap filters.
    expr_context: Option<*mut pgrx::pg_sys::ExprContext>,
}

// Same justification as the logical `PgSearchExtensionCodec`: Postgres extensions run
// single-threaded, so the raw `ParallelScanState` pointer never crosses a real thread boundary.
unsafe impl Send for PgSearchPhysicalExtensionCodec {}
unsafe impl Sync for PgSearchPhysicalExtensionCodec {}

impl PhysicalExtensionCodec for PgSearchPhysicalExtensionCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        inputs: &[Arc<dyn ExecutionPlan>],
        ctx: &TaskContext,
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
                self.expr_context,
            ),
            // The deferred execs (visibility ctid resolvers, tantivy lookup, segmented top-k)
            // carry live `FFHelper`s that can't travel. Decode is bottom-up, so the scans below
            // are already rebuilt; pull their helpers out of the decoded subtree. A column whose
            // scan sits behind a network boundary rebuilds its helper from the column's
            // `DeferredLookupRebuild` instead.
            TAG_VISIBILITY_FILTER => {
                let input = single_input(inputs)?;
                let resolvers = collect_ctid_resolvers(&input);
                VisibilityFilterExec::decode_for_dispatch(
                    payload,
                    input,
                    resolvers,
                    &self.index_segment_ids,
                )
            }
            TAG_TANTIVY_LOOKUP => {
                let input = single_input(inputs)?;
                let ffhelpers = collect_ffhelpers_by_indexrelid(&input);
                TantivyLookupExec::decode_for_dispatch(
                    payload,
                    input,
                    ffhelpers,
                    &self.non_partitioning_segment_ids,
                    self.parallel_state,
                )
            }
            TAG_SEGMENTED_TOPK => {
                let input = single_input(inputs)?;
                let ffhelpers = collect_ffhelpers_by_indexrelid(&input);
                // Re-collect the live ctid resolvers from the decoded subtree so a dispatched
                // fragment can rebuild its absorbed visibility data (same as VFExec above).
                let resolvers = collect_ctid_resolvers(&input);
                let stk_resolvers = resolvers.into_iter().map(|(pos, _, ff)| (pos, ff)).collect();
                SegmentedTopKExec::decode_for_dispatch(
                    payload,
                    input,
                    ffhelpers,
                    stk_resolvers,
                    ctx,
                    &self.non_partitioning_segment_ids,
                    self.parallel_state,
                )
            }
            other => Err(DataFusionError::NotImplemented(format!(
                "PgSearchPhysicalExtensionCodec: unknown physical node tag {other}"
            ))),
        }
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        if let Some(scan) = node.downcast_ref::<PgSearchScanPlan>() {
            buf.push(TAG_PG_SEARCH_SCAN);
            buf.extend_from_slice(&scan.encode_for_dispatch()?);
            return Ok(());
        }
        if let Some(vis) = node.downcast_ref::<VisibilityFilterExec>() {
            buf.push(TAG_VISIBILITY_FILTER);
            buf.extend_from_slice(&vis.encode_for_dispatch()?);
            return Ok(());
        }
        if let Some(lookup) = node.downcast_ref::<TantivyLookupExec>() {
            buf.push(TAG_TANTIVY_LOOKUP);
            buf.extend_from_slice(&lookup.encode_for_dispatch()?);
            return Ok(());
        }
        if let Some(topk) = node.downcast_ref::<SegmentedTopKExec>() {
            buf.push(TAG_SEGMENTED_TOPK);
            buf.extend_from_slice(&topk.encode_for_dispatch()?);
            return Ok(());
        }
        Err(DataFusionError::NotImplemented(format!(
            "PgSearchPhysicalExtensionCodec: no physical encoding for {}",
            node.name()
        )))
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
                        .ok_or_else(|| {
                            DataFusionError::Internal(format!(
                                "missing canonical segment IDs for plan_position {plan_position}"
                            ))
                        })?;
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
                .downcast_ref::<SearchPredicateUDF>()
                .ok_or_else(|| {
                    DataFusionError::Internal("UDF is not a SearchPredicateUDF".into())
                })?;
            let bytes = serde_json::to_vec(udf).map_err(|e| {
                DataFusionError::Internal(format!("Failed to serialize SearchPredicateUDF: {e}"))
            })?;
            buf.extend_from_slice(&bytes);
            return Ok(());
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
            return Ok(());
        }

        // Not ours: encode nothing so the expression travels by name and the decoding session
        // resolves it from its registry (DataFusion built-ins are registered there).
        Ok(())
    }
}

/// Per-scan runtime handles pulled from a decoded subtree, used to re-wire the deferred execs.
struct ScanRuntime {
    indexrelid: u32,
    ffhelper: Option<Arc<FFHelper>>,
    ctid_plan_position: Option<usize>,
}

/// Walk a decoded subtree collecting each `PgSearchScanPlan`'s runtime handles. Nested stages
/// decode as `Remote` (no inline child), so the walk covers this fragment's own scans; columns
/// whose scan lives in another fragment rebuild their helpers from `DeferredLookupRebuild`.
/// Binding a resolver to any reader it finds is fine: canonical segment sets give every proc
/// the same `segment_ord` layout.
fn collect_scan_runtime(plan: &Arc<dyn ExecutionPlan>, out: &mut Vec<ScanRuntime>) {
    if let Some(scan) = plan.downcast_ref::<PgSearchScanPlan>() {
        out.push(ScanRuntime {
            indexrelid: scan.indexrelid,
            ffhelper: scan.ffhelper(),
            ctid_plan_position: scan.deferred_ctid_plan_position(),
        });
    }
    for child in plan.children() {
        collect_scan_runtime(child, out);
    }
}

fn single_input(inputs: &[Arc<dyn ExecutionPlan>]) -> Result<Arc<dyn ExecutionPlan>> {
    match inputs {
        [one] => Ok(Arc::clone(one)),
        _ => Err(DataFusionError::Internal(format!(
            "PgSearchPhysicalExtensionCodec: expected one input, got {}",
            inputs.len()
        ))),
    }
}

/// `(plan_position, indexrelid, ffhelper)` for each scan that resolves deferred ctids, for the
/// visibility exec.
fn collect_ctid_resolvers(input: &Arc<dyn ExecutionPlan>) -> Vec<(usize, u32, Arc<FFHelper>)> {
    let mut scans = Vec::new();
    collect_scan_runtime(input, &mut scans);
    scans
        .into_iter()
        .filter_map(|s| match (s.ctid_plan_position, s.ffhelper) {
            (Some(pos), Some(ff)) => Some((pos, s.indexrelid, ff)),
            _ => None,
        })
        .collect()
}

/// `indexrelid -> ffhelper` for the tantivy lookup exec.
fn collect_ffhelpers_by_indexrelid(input: &Arc<dyn ExecutionPlan>) -> HashMap<u32, Arc<FFHelper>> {
    let mut scans = Vec::new();
    collect_scan_runtime(input, &mut scans);
    let mut map = HashMap::default();
    for s in scans {
        if let Some(ff) = s.ffhelper {
            map.insert(s.indexrelid, ff);
        }
    }
    map
}

/// [`DistributedCodec`] with the pg_search UDF names carved out of its UDF/UDAF handling.
///
/// The composed codec takes the first `Ok` per call, and the trait's default `try_encode_udf`
/// returns `Ok` writing nothing, so a bare `DistributedCodec` at position 0 would shadow the
/// pg_search UDF serialization: no `fun_definition` would ever travel, and a dispatched stage
/// retaining a `pdb_search_predicate` / `pg_expr_*` expression would fail decode on the worker
/// (their registry has no such functions). Position 0 still matters for everything else:
/// `prost` skips default values, so only position 0 with an empty blob encodes to zero bytes,
/// which is what keeps registry-resolved built-ins travelling by name. So this wrapper accepts
/// (encodes nothing for) every name except ours, and declines ours so composition falls through
/// to [`PgSearchPhysicalExtensionCodec`], which ships the real definition.
#[derive(Debug)]
struct DistributedCodecHostingPgSearchUdfs(DistributedCodec);

fn is_pg_search_udf(name: &str) -> bool {
    name == "pdb_search_predicate" || name.starts_with(PG_EXPR_UDF_PREFIX)
}

impl PhysicalExtensionCodec for DistributedCodecHostingPgSearchUdfs {
    fn try_decode(
        &self,
        buf: &[u8],
        inputs: &[Arc<dyn ExecutionPlan>],
        ctx: &TaskContext,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        self.0.try_decode(buf, inputs, ctx)
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        self.0.try_encode(node, buf)
    }

    fn try_encode_udf(&self, node: &ScalarUDF, _buf: &mut Vec<u8>) -> Result<()> {
        if is_pg_search_udf(node.name()) {
            return Err(DataFusionError::NotImplemented(format!(
                "UDF '{}' is encoded by the pg_search codec",
                node.name()
            )));
        }
        Ok(())
    }

    fn try_decode_udf(&self, name: &str, _buf: &[u8]) -> Result<Arc<ScalarUDF>> {
        Err(DataFusionError::NotImplemented(format!(
            "UDF '{name}' is not registered on the decoding session and carries no definition"
        )))
    }
}

/// Compose the fork's [`DistributedCodec`] (handles `Network*Exec` / `StageExec`) with the
/// `pg_search` codec. Encode and decode MUST build the same list in the same order: the composed
/// codec records each node's codec index on the wire.
fn combined_codec(user: PgSearchPhysicalExtensionCodec) -> ComposedPhysicalExtensionCodec {
    ComposedPhysicalExtensionCodec::new(vec![
        Arc::new(DistributedCodecHostingPgSearchUdfs(DistributedCodec {})),
        Arc::new(user),
    ])
}

/// Serialize one stage's physical subplan for dispatch. Context-free: only the recipe travels,
/// the receiving worker injects its own runtime state on decode.
pub fn serialize_physical_plan(plan: Arc<dyn ExecutionPlan>) -> Result<Vec<u8>> {
    // FilterPassthroughExec only matters during filter-pushdown optimization; once the plan is
    // finalized it delegates to its inner node, so strip it and ship the inner directly.
    let plan = plan
        .transform_down(|node| {
            if let Some(fp) = node.downcast_ref::<FilterPassthroughExec>() {
                Ok(Transformed::yes(Arc::clone(fp.inner())))
            } else {
                Ok(Transformed::no(node))
            }
        })?
        .data;
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
    expr_context: Option<*mut pgrx::pg_sys::ExprContext>,
) -> Result<Arc<dyn ExecutionPlan>> {
    let codec = combined_codec(PgSearchPhysicalExtensionCodec {
        parallel_state,
        non_partitioning_segment_ids,
        index_segment_ids,
        expr_context,
    });
    let proto = <PhysicalPlanNode as prost::Message>::decode(bytes).map_err(|e| {
        DataFusionError::Internal(format!("Failed to decode dispatched PhysicalPlanNode: {e}"))
    })?;
    let plan = proto.try_into_physical_plan(ctx, &codec)?;
    // Dynamic filters (hash-join keys, top-k bounds) are process-local Arcs shared between a
    // node and the scans below it; the proto layer snapshots them into static expressions, so
    // the shared link can't ride the wire. Re-running the post-optimization pushdown pass on
    // the decoded fragment re-creates the links, so this task's probe scans prune against its
    // own build side instead of scanning every segment. The relink is possible only because a
    // fragment keeps an operator and its probe scans in the same process; a link that crossed
    // fragments would need the filter values shipped between processes.
    //
    // Prior art: the same pass was proposed for datafusion-distributed
    // (https://github.com/datafusion-contrib/datafusion-distributed/pull/348) and closed in
    // favor of fixing it in DataFusion proper, by serializing and deduping dynamic filters so
    // decode re-shares one instance (https://github.com/apache/datafusion/pull/20416, design
    // discussion in https://github.com/apache/datafusion/issues/21207). Once DataFusion
    // round-trips the filters natively, this pass can go.
    FilterPushdown::new_post_optimization().optimize(plan, ctx.session_config().options())
}
