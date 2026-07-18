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

use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::catalog::TableProvider;
use datafusion::common::{DFSchemaRef, DataFusionError, Result, TableReference};
use datafusion::execution::TaskContext;
use datafusion::functions_aggregate as dfa;
use datafusion::logical_expr::{AggregateUDF, Extension, LogicalPlan, ScalarUDF};
use datafusion_proto::logical_plan::LogicalExtensionCodec;
use datafusion_proto::protobuf::DfSchema;
use pgrx::pg_sys::{ExprContext, Oid, PlanState};
use tantivy::index::SegmentId;

use crate::api::HashSet;
use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterNode;
use crate::postgres::customscan::pg_expr_udf::{PgExprUdf, PG_EXPR_UDF_PREFIX};
use crate::postgres::ParallelScanState;
use crate::scan::late_materialization::{DeferredField, LateMaterializeNode};
use crate::scan::search_predicate_udf::SearchPredicateUDF;
use crate::scan::table_provider::PgSearchTableProvider;

/// Datafusion `LogicalPlan`s are serialized/deserialized with protobuf.
/// Any custom nodes (e.g. UDFs, table providers) must use this codec to instruct
/// DataFusion how to serialize/deserialize them.
#[derive(Debug, Default)]
struct PgSearchExtensionCodec {
    /// Shared state for parallel scans, containing the list of segments to be processed.
    parallel_state: Option<*mut ParallelScanState>,
    /// Postgres expression context, needed for heap filtering and runtime parameters.
    expr_context: Option<*mut ExprContext>,
    /// Executor planstate, needed to initialize runtime Postgres expressions in source queries.
    planstate: Option<*mut PlanState>,
    /// Canonical segment ID sets for all join sources, indexed by plan_position.
    index_segment_ids: Vec<HashSet<SegmentId>>,
}

unsafe impl Send for PgSearchExtensionCodec {}
unsafe impl Sync for PgSearchExtensionCodec {}

impl LogicalExtensionCodec for PgSearchExtensionCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        inputs: &[LogicalPlan],
        _ctx: &TaskContext,
    ) -> Result<Extension> {
        if buf.is_empty() {
            return Err(DataFusionError::Internal(
                "Empty buffer for Extension decode".into(),
            ));
        }

        // TODO: This uses a manual byte-tagging scheme to identify custom Extension nodes.
        // If we add more custom node types, we should switch this payload to a proper Serde
        // enum (e.g. `bincode` or `serde_json` of an enum wrapper) to cleanly handle variants.
        let tag = buf[0];
        if tag == 1 {
            if inputs.len() != 1 {
                return Err(DataFusionError::Internal(
                    "LateMaterializeNode requires exactly one input".into(),
                ));
            }
            let input_plan = inputs[0].clone();

            let mut offset = 1;

            let schema_len_bytes = buf.get(offset..offset + 4).ok_or_else(|| {
                DataFusionError::Internal("truncated buffer: missing schema length".into())
            })?;
            let schema_len = u32::from_le_bytes(schema_len_bytes.try_into().unwrap()) as usize;
            offset += 4;

            let schema_bytes = buf.get(offset..offset + schema_len).ok_or_else(|| {
                DataFusionError::Internal("truncated buffer: incomplete schema data".into())
            })?;
            offset += schema_len;

            let df_schema_proto: DfSchema = prost::Message::decode(schema_bytes).map_err(|e| {
                DataFusionError::Internal(format!("Failed to decode schema: {}", e))
            })?;

            let output_schema: DFSchemaRef =
                Arc::new((&df_schema_proto).try_into().map_err(|e| {
                    DataFusionError::Internal(format!("Failed to parse schema: {}", e))
                })?);

            let deferred_len_bytes = buf.get(offset..offset + 4).ok_or_else(|| {
                DataFusionError::Internal("truncated buffer: missing deferred fields length".into())
            })?;
            let deferred_len = u32::from_le_bytes(deferred_len_bytes.try_into().unwrap()) as usize;
            offset += 4;

            let deferred_fields_bytes =
                buf.get(offset..offset + deferred_len).ok_or_else(|| {
                    DataFusionError::Internal(
                        "truncated buffer: incomplete deferred fields data".into(),
                    )
                })?;
            let deferred_fields: Vec<DeferredField> = serde_json::from_slice(deferred_fields_bytes)
                .map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to deserialize deferred fields: {}",
                        e
                    ))
                })?;

            let node = Arc::new(LateMaterializeNode {
                input: input_plan,
                output_schema,
                deferred_fields,
            });

            return Ok(Extension { node });
        }

        if tag == 2 {
            if inputs.len() != 1 {
                return Err(DataFusionError::Internal(
                    "VisibilityFilterNode requires exactly one input".into(),
                ));
            }
            let input_plan = inputs[0].clone();
            let payload_len_bytes = buf.get(1..5).ok_or_else(|| {
                DataFusionError::Internal("truncated buffer: missing visibility length".into())
            })?;
            let payload_len = u32::from_le_bytes(payload_len_bytes.try_into().unwrap()) as usize;
            let payload = buf.get(5..5 + payload_len).ok_or_else(|| {
                DataFusionError::Internal("truncated buffer: incomplete visibility payload".into())
            })?;
            let (plan_pos_oids, table_names): (Vec<(usize, Oid)>, Vec<String>) =
                serde_json::from_slice(payload).map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to deserialize visibility payload: {e}"
                    ))
                })?;
            return Ok(Extension {
                node: Arc::new(VisibilityFilterNode::new(
                    input_plan,
                    plan_pos_oids,
                    table_names,
                )),
            });
        }

        Err(DataFusionError::NotImplemented(format!(
            "Extension node decoding not implemented for tag {}",
            tag
        )))
    }

    fn try_encode(&self, node: &Extension, buf: &mut Vec<u8>) -> Result<()> {
        if let Some(mat_node) = node.node.as_any().downcast_ref::<LateMaterializeNode>() {
            let schema_proto: DfSchema =
                mat_node.output_schema.as_ref().try_into().map_err(|e| {
                    DataFusionError::Internal(format!("Failed to convert schema: {}", e))
                })?;

            let bytes = serde_json::to_vec(&mat_node.deferred_fields).map_err(|e| {
                DataFusionError::Internal(format!("Failed to serialize deferred fields: {}", e))
            })?;

            buf.push(1);
            let schema_bytes = prost::Message::encode_to_vec(&schema_proto);

            buf.extend_from_slice(&(schema_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(&schema_bytes);
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(&bytes);
            return Ok(());
        }

        if let Some(vis_node) = node.node.as_any().downcast_ref::<VisibilityFilterNode>() {
            let payload: (&[(usize, Oid)], &[String]) =
                (&vis_node.plan_pos_oids, &vis_node.table_names);
            let bytes = serde_json::to_vec(&payload).map_err(|e| {
                DataFusionError::Internal(format!(
                    "Failed to serialize visibility plan positions: {e}"
                ))
            })?;
            buf.push(2);
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(&bytes);
            return Ok(());
        }

        Err(DataFusionError::NotImplemented(format!(
            "Extension node encoding not implemented for {:?}",
            node.node.name()
        )))
    }

    fn try_decode_table_provider(
        &self,
        buf: &[u8],
        _table_ref: &TableReference,
        _schema: SchemaRef,
        _ctx: &TaskContext,
    ) -> Result<Arc<dyn TableProvider>> {
        let mut provider: PgSearchTableProvider = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize PgSearchTableProvider: {e}"))
        })?;
        // MPP sources also call `checkout_segment_for_source` against
        // `parallel_state`, so inject the pointer for them too.
        if provider.is_parallel() || provider.mpp_source_idx().is_some() {
            provider.set_parallel_state(self.parallel_state);
        }
        provider.set_expr_context(self.expr_context);
        provider.set_planstate(self.planstate);
        Ok(Arc::new(provider))
    }

    fn try_encode_table_provider(
        &self,
        _table_ref: &TableReference,
        node: Arc<dyn TableProvider>,
        buf: &mut Vec<u8>,
    ) -> Result<()> {
        let provider = node
            .downcast_ref::<PgSearchTableProvider>()
            .ok_or_else(|| {
                DataFusionError::Internal(
                    "TableProvider is not a PgSearchTableProvider".to_string(),
                )
            })?;
        let bytes = serde_json::to_vec(provider).map_err(|e| {
            DataFusionError::Internal(format!("Failed to serialize PgSearchTableProvider: {e}"))
        })?;
        buf.extend_from_slice(&bytes);
        Ok(())
    }

    fn try_decode_udaf(&self, name: &str, _buf: &[u8]) -> Result<Arc<AggregateUDF>> {
        match name {
            "min" => Ok(dfa::min_max::min_udaf()),
            "max" => Ok(dfa::min_max::max_udaf()),
            "count" => Ok(dfa::count::count_udaf()),
            "sum" => Ok(dfa::sum::sum_udaf()),
            "avg" => Ok(dfa::average::avg_udaf()),
            _ => Err(DataFusionError::NotImplemented(format!(
                "LogicalExtensionCodec is not provided for aggregate function {name}"
            ))),
        }
    }

    fn try_encode_udaf(&self, node: &AggregateUDF, buf: &mut Vec<u8>) -> Result<()> {
        // Built-in aggregates are looked up by name on decode, no state to serialize
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
            "UDF '{}' deserialization not implemented",
            name
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

        Err(DataFusionError::NotImplemented(format!(
            "UDF '{}' serialization not implemented",
            name
        )))
    }
}

/// Serializes a DataFusion `LogicalPlan` to bytes using the `PgSearchExtensionCodec`.
pub fn serialize_logical_plan(plan: &LogicalPlan) -> Result<bytes::Bytes> {
    datafusion_proto::bytes::logical_plan_to_bytes_with_extension_codec(
        plan,
        &PgSearchExtensionCodec::default(),
    )
}

/// Deserializes a DataFusion `LogicalPlan` using a codec populated with the
/// runtime state required by execution.
#[allow(clippy::too_many_arguments)]
pub fn deserialize_logical_plan_with_runtime(
    bytes: &[u8],
    ctx: &TaskContext,
    parallel_state: Option<*mut ParallelScanState>,
    expr_context: Option<*mut ExprContext>,
    planstate: Option<*mut PlanState>,
    index_segment_ids: Vec<HashSet<SegmentId>>,
) -> Result<LogicalPlan> {
    let codec = PgSearchExtensionCodec {
        parallel_state,
        expr_context,
        planstate,
        index_segment_ids,
    };
    datafusion_proto::bytes::logical_plan_from_bytes_with_extension_codec(bytes, ctx, &codec)
}
