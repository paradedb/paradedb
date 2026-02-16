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

use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::joinscan::exchange::{
    get_mpp_config, register_stream_source, DsmExchangeConfig, DsmExchangeExec, StreamSource,
};
use crate::postgres::customscan::joinscan::transport::ParticipantId;
use crate::query::SearchQueryInput;
use crate::scan::execution_plan::MultiSegmentPlan;
use crate::scan::info::ScanInfo;
use crate::scan::table_provider::MppParticipantConfig;
use arrow_schema::SchemaRef;
use datafusion::catalog::TableProvider;
use datafusion::common::TableReference;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::logical_expr::{Extension, LogicalPlan, ScalarUDF};
use datafusion_proto::logical_plan::LogicalExtensionCodec;
use datafusion_proto::physical_plan::PhysicalExtensionCodec;
use serde::{Deserialize, Serialize};

use crate::scan::search_predicate_udf::SearchPredicateUDF;
use crate::scan::table_provider::PgSearchTableProvider;
use datafusion_proto::physical_plan::from_proto::parse_protobuf_partitioning;
use datafusion_proto::physical_plan::to_proto::serialize_partitioning;
use datafusion_proto::protobuf;
use prost::Message;

#[derive(Serialize, Deserialize)]
enum PhysicalNode {
    DsmExchange {
        config: DsmExchangeConfig,
        producer_partitioning_bytes: Vec<u8>,
        output_partitioning_bytes: Vec<u8>,
    },
    MultiSegment {
        scan_info: Box<ScanInfo>,
        fields: Vec<WhichFastField>,
        query_for_display: Box<SearchQueryInput>,
        target_partitioning_bytes: Vec<u8>,
    },
}

/// Datafusion `LogicalPlan`s are serialized/deserialized with protobuf.
/// Any custom nodes (e.g. UDFs, table providers) must use this codec to instruct
/// DataFusion how to serialize/deserialize them.
#[derive(Debug, Default)]
pub struct PgSearchExtensionCodec {}

unsafe impl Send for PgSearchExtensionCodec {}
unsafe impl Sync for PgSearchExtensionCodec {}

/// Generated code for `try_decode_udf` for a list of UDF types.
macro_rules! decode_udfs {
    ($($name:literal => $ty:ty),* $(,)?) => {
        fn try_decode_udf(&self, name: &str, buf: &[u8]) -> Result<Arc<ScalarUDF>> {
            match name {
                $(
                    $name => {
                        let udf: $ty = serde_json::from_slice(buf).map_err(|e| {
                            DataFusionError::Internal(format!(
                                "Failed to deserialize {}: {e}",
                                stringify!($ty)
                            ))
                        })?;
                        Ok(Arc::new(ScalarUDF::new_from_impl(udf)))
                    }
                )*
                _ => Err(DataFusionError::NotImplemented(format!(
                    "UDF '{}' deserialization not implemented",
                    name
                ))),
            }
        }
    };
}

/// Generated code for `try_encode_udf` for a list of UDF types.
macro_rules! encode_udfs {
    ($($name:literal => $ty:ty),* $(,)?) => {
        fn try_encode_udf(&self, node: &ScalarUDF, buf: &mut Vec<u8>) -> Result<()> {
            let name = node.name();
            match name {
                $(
                    $name => {
                        let udf = node
                            .inner()
                            .as_any()
                            .downcast_ref::<$ty>()
                            .ok_or_else(|| {
                                DataFusionError::Internal(format!(
                                    "UDF is not a {}",
                                    stringify!($ty)
                                ))
                            })?;
                        let bytes = serde_json::to_vec(udf).map_err(|e| {
                            DataFusionError::Internal(format!(
                                "Failed to serialize {}: {e}",
                                stringify!($ty)
                            ))
                        })?;
                        buf.extend_from_slice(&bytes);
                        Ok(())
                    }
                )*
                _ => Err(DataFusionError::NotImplemented(format!(
                    "UDF '{}' serialization not implemented",
                    name
                ))),
            }
        }
    };
}

impl PhysicalExtensionCodec for PgSearchExtensionCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        inputs: &[Arc<dyn datafusion::physical_plan::ExecutionPlan>],
        ctx: &TaskContext,
    ) -> Result<Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        let node: PhysicalNode = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize PhysicalNode: {e}"))
        })?;

        match node {
            PhysicalNode::DsmExchange {
                config,
                producer_partitioning_bytes,
                output_partitioning_bytes,
            } => {
                let producer_proto = protobuf::Partitioning::decode(
                    &producer_partitioning_bytes[..],
                )
                .map_err(|e| {
                    DataFusionError::Internal(format!(
                        "Failed to decode Producer Partitioning: {e}"
                    ))
                })?;
                let output_proto = protobuf::Partitioning::decode(&output_partitioning_bytes[..])
                    .map_err(|e| {
                    DataFusionError::Internal(format!("Failed to decode Output Partitioning: {e}"))
                })?;
                let schema = inputs[0].schema();
                let producer_partitioning =
                    parse_protobuf_partitioning(Some(&producer_proto), ctx, &schema, self)?
                        .ok_or_else(|| {
                            DataFusionError::Internal(
                                "Failed to parse producer partitioning".to_string(),
                            )
                        })?;
                let output_partitioning =
                    parse_protobuf_partitioning(Some(&output_proto), ctx, &schema, self)?
                        .ok_or_else(|| {
                            DataFusionError::Internal(
                                "Failed to parse output partitioning".to_string(),
                            )
                        })?;

                // Register this exchange as a procedure available on this node (Producer Side)
                let (participant_index, _) = get_mpp_config(ctx);
                register_stream_source(
                    StreamSource {
                        input: inputs[0].clone(),
                        partitioning: producer_partitioning.clone(),
                        config: config.clone(),
                    },
                    ParticipantId(participant_index as u16),
                );

                Ok(Arc::new(DsmExchangeExec::try_new(
                    inputs[0].clone(),
                    producer_partitioning,
                    output_partitioning,
                    config,
                )?))
            }
            PhysicalNode::MultiSegment {
                scan_info,
                fields,
                query_for_display,
                target_partitioning_bytes,
            } => {
                let schema = Arc::new(arrow_schema::Schema::new(
                    fields
                        .iter()
                        .map(|f| arrow_schema::Field::new(f.name(), f.arrow_data_type(), true))
                        .collect::<Vec<_>>(),
                ));

                let proto = protobuf::Partitioning::decode(&target_partitioning_bytes[..])
                    .map_err(|e| {
                        DataFusionError::Internal(format!("Failed to decode Partitioning: {e}"))
                    })?;
                let partitioning = parse_protobuf_partitioning(Some(&proto), ctx, &schema, self)?
                    .ok_or_else(|| {
                    DataFusionError::Internal("Failed to parse partitioning".to_string())
                })?;

                // Reconstruct MultiSegmentPlan by opening segments on this worker
                let mpp_config = ctx
                    .session_config()
                    .options()
                    .extensions
                    .get::<MppParticipantConfig>();
                let participant_index = mpp_config.as_ref().map(|c| c.index).unwrap_or(0);
                let total_participants = mpp_config
                    .as_ref()
                    .map(|c| c.total_participants)
                    .unwrap_or(1);

                let heaprelid = scan_info
                    .heaprelid
                    .ok_or_else(|| DataFusionError::Internal("Missing heaprelid".into()))?;
                let index_relid = scan_info
                    .indexrelid
                    .ok_or_else(|| DataFusionError::Internal("Missing indexrelid".into()))?;

                let heap_rel = crate::postgres::rel::PgSearchRelation::open(heaprelid);
                let index_rel = crate::postgres::rel::PgSearchRelation::open(index_relid);

                let reader = crate::index::reader::index::SearchIndexReader::open_with_context(
                    &index_rel,
                    scan_info.query.clone().unwrap_or(SearchQueryInput::All),
                    scan_info.score_needed,
                    crate::index::mvcc::MvccSatisfies::Snapshot,
                    None,
                    None,
                )
                .map_err(|e| DataFusionError::Internal(format!("Failed to open reader: {e}")))?;

                let ffhelper =
                    crate::index::fast_fields_helper::FFHelper::with_fields(&reader, &fields);
                let snapshot = unsafe { pgrx::pg_sys::GetActiveSnapshot() };
                let visibility = crate::postgres::heap::VisibilityChecker::with_rel_and_snap(
                    &heap_rel, snapshot,
                );
                let sort_order = scan_info.sort_order.clone();

                let ffhelper = Arc::new(ffhelper);
                let segment_readers = reader.segment_readers();
                let segments: Vec<crate::scan::execution_plan::ScanState> = segment_readers
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| {
                        if total_participants > 1 {
                            let (start, length) = crate::parallel_worker::chunk_range(
                                segment_readers.len(),
                                total_participants,
                                participant_index,
                            );
                            *i >= start && *i < start + length
                        } else {
                            true
                        }
                    })
                    .map(|(_, r)| {
                        let search_results =
                            reader.search_segments(std::iter::once(r.segment_id()));
                        let scanner = crate::scan::Scanner::new(
                            search_results,
                            None,
                            fields.clone(),
                            heaprelid.into(),
                        );
                        (
                            scanner,
                            Arc::clone(&ffhelper),
                            Box::new(visibility.clone()) as Box<dyn crate::scan::VisibilityChecker>,
                        )
                    })
                    .collect();

                Ok(Arc::new(MultiSegmentPlan::new_with_partitioning(
                    segments,
                    schema,
                    *query_for_display,
                    sort_order.as_ref(),
                    Some(partitioning),
                    *scan_info,
                    fields,
                )))
            }
        }
    }

    fn try_encode(
        &self,
        node: Arc<dyn datafusion::physical_plan::ExecutionPlan>,
        buf: &mut Vec<u8>,
    ) -> Result<()> {
        if let Some(exchange) = node.as_any().downcast_ref::<DsmExchangeExec>() {
            let producer_partitioning = &exchange.producer_partitioning;
            let output_partitioning = exchange.properties.output_partitioning();

            let producer_proto = serialize_partitioning(producer_partitioning, self)?;
            let output_proto = serialize_partitioning(output_partitioning, self)?;

            let physical_node = PhysicalNode::DsmExchange {
                config: exchange.config.clone(),
                producer_partitioning_bytes: producer_proto.encode_to_vec(),
                output_partitioning_bytes: output_proto.encode_to_vec(),
            };
            serde_json::to_writer(buf, &physical_node).map_err(|e| {
                DataFusionError::Internal(format!("Failed to serialize DsmExchangeExec: {e}"))
            })?;
            Ok(())
        } else if let Some(ms) = node.as_any().downcast_ref::<MultiSegmentPlan>() {
            let partitioning = &ms.target_partitioning;
            let proto = serialize_partitioning(partitioning, self)?;
            let physical_node = PhysicalNode::MultiSegment {
                scan_info: Box::new(ms.scan_info.clone()),
                fields: ms.fields.clone(),
                query_for_display: Box::new(ms.query_for_display.clone()),
                target_partitioning_bytes: proto.encode_to_vec(),
            };
            serde_json::to_writer(buf, &physical_node).map_err(|e| {
                DataFusionError::Internal(format!("Failed to serialize MultiSegmentPlan: {e}"))
            })?;
            Ok(())
        } else {
            Err(DataFusionError::Internal(format!(
                "Unknown physical node: {}",
                node.name()
            )))
        }
    }

    decode_udfs! {
        "pdb_search_predicate" => SearchPredicateUDF,
    }

    encode_udfs! {
        "pdb_search_predicate" => SearchPredicateUDF,
    }
}

impl LogicalExtensionCodec for PgSearchExtensionCodec {
    fn try_decode(
        &self,
        _buf: &[u8],
        _inputs: &[LogicalPlan],
        _ctx: &TaskContext,
    ) -> Result<Extension> {
        Err(DataFusionError::NotImplemented(
            "Extension node decoding not implemented".to_string(),
        ))
    }

    fn try_encode(&self, _node: &Extension, _buf: &mut Vec<u8>) -> Result<()> {
        Err(DataFusionError::NotImplemented(
            "Extension node encoding not implemented".to_string(),
        ))
    }

    fn try_decode_table_provider(
        &self,
        buf: &[u8],
        _table_ref: &TableReference,
        _schema: SchemaRef,
        _ctx: &TaskContext,
    ) -> Result<Arc<dyn TableProvider>> {
        let provider: PgSearchTableProvider = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize PgSearchTableProvider: {e}"))
        })?;
        Ok(Arc::new(provider))
    }

    fn try_encode_table_provider(
        &self,
        _table_ref: &TableReference,
        node: Arc<dyn TableProvider>,
        buf: &mut Vec<u8>,
    ) -> Result<()> {
        let provider = node
            .as_any()
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

    decode_udfs! {
        "pdb_search_predicate" => SearchPredicateUDF,
    }

    encode_udfs! {
        "pdb_search_predicate" => SearchPredicateUDF,
    }
}
