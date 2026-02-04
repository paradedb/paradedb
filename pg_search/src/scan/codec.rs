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
use datafusion::common::TableReference;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::logical_expr::{Extension, LogicalPlan, ScalarUDF};
use datafusion_proto::logical_plan::LogicalExtensionCodec;

use crate::postgres::customscan::joinscan::udf::RowInSetUDF;
use crate::scan::table_provider::PgSearchTableProvider;

/// Datafusion `LogicalPlan`s are serialized/deserialized with protobuf.
/// Any custom nodes (e.g. UDFs, table providers) must use this codec to instruct
/// DataFusion how to serialize/deserialize them.
#[derive(Debug, Default)]
pub struct PgSearchExtensionCodec {
    /// Shared state for parallel scans, containing the list of segments to be processed.
    pub parallel_state: Option<*mut crate::postgres::ParallelScanState>,
    /// The OID of the index being parallelized. In a JoinScan, only the first table
    /// is partitioned using `parallel_state`, while subsequent tables are fully replicated.
    /// This OID ensures `parallel_state` is only injected into the correct `PgSearchTableProvider`.
    pub parallel_index_relid: Option<pgrx::pg_sys::Oid>,
}

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
        let mut provider: PgSearchTableProvider = serde_json::from_slice(buf).map_err(|e| {
            DataFusionError::Internal(format!("Failed to deserialize PgSearchTableProvider: {e}"))
        })?;
        // Only inject parallel state if this provider matches the index we parallelized (the first one)
        if provider.index_relid() == self.parallel_index_relid {
            provider.set_parallel_state(self.parallel_state);
        }
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
        "row_in_set" => RowInSetUDF,
    }

    encode_udfs! {
        "row_in_set" => RowInSetUDF,
    }
}
