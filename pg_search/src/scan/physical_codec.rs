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

//! Real physical codec for our custom `ExecutionPlan` nodes.
//!
//! Companion to [`crate::scan::codec::PgSearchExtensionCodec`] (the logical-plan codec). The
//! logical codec is used today on the DSM path: leader ships a serialized logical plan, workers
//! deserialize and re-run physical planning. This module exists to support the planned move to
//! shipping fully-built physical subplans on workers' Request response cycle — see PR description
//! for the dispatch-flip follow-up.
//!
//! Today the codec is registered as a *parallel* codec to the existing
//! [`crate::scan::codec::PgSearchPhysicalCodecStub`]: the stub stays the one used by
//! `with_distributed_user_codec` until the dispatch-flip PR lands. That keeps every commit in
//! this PR safe even while individual exec encoders/decoders are being filled in: the new codec
//! is exercised by round-trip tests but never produces bytes a worker reads.
//!
//! ## Wire format
//!
//! All custom execs serialize through one `CustomExec` prost message that holds a oneof of the
//! supported variants. Adding a new exec means adding a variant and a match arm in
//! `try_encode` / `try_decode`.
//!
//! ## Reconstruction model
//!
//! Several execs hold tantivy runtime state (`ScanState`, `FFHelper`, etc.) that can't ship over
//! the wire. The encoded form carries only declarative inputs (`indexrelid`, segment IDs, query,
//! schema). `try_decode` rebuilds runtime state on the worker side via the same constructors
//! that the table provider uses today.
//!
//! ## On the `dead_code` allow
//!
//! Nothing in production registers this codec yet — `with_distributed_user_codec` still gets
//! [`crate::scan::codec::PgSearchPhysicalCodecStub`]. The struct, the proto messages, and the
//! per-exec encode/decode helpers are exercised only by this file's `#[cfg(test)]` round-trip
//! tests. Once the dispatch-flip PR swaps the registration, the allow comes off naturally.

#![allow(dead_code)]

use std::sync::Arc;

use arrow_schema::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::TaskContext;
use datafusion::physical_plan::ExecutionPlan;
use datafusion_proto::physical_plan::PhysicalExtensionCodec;
use datafusion_proto::protobuf::DfSchema;
// `pgrx::pg_sys` is only referenced by the production codec functions (gated behind
// `#[cfg(not(test))]` to keep coverage builds from linking PG runtime symbols). In test builds
// the import would be unused and trigger `unused_imports`; gate it out.
#[cfg(not(test))]
use pgrx::pg_sys;
use prost::Message;

/// Top-level wire envelope. One variant per custom `ExecutionPlan` we need to ship.
#[derive(Clone, PartialEq, ::prost::Message)]
struct CustomExecProto {
    #[prost(oneof = "custom_exec_proto::Variant", tags = "1, 2, 3, 4")]
    pub variant: Option<custom_exec_proto::Variant>,
}

mod custom_exec_proto {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Variant {
        #[prost(message, tag = "1")]
        VisibilityFilter(super::VisibilityFilterProto),
        #[prost(message, tag = "2")]
        TantivyLookup(super::TantivyLookupProto),
        #[prost(message, tag = "3")]
        SegmentedTopK(super::SegmentedTopKProto),
        #[prost(message, tag = "4")]
        PgSearchScan(super::PgSearchScanProto),
    }
}

/// Wire form for [`crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec`].
///
/// Carries the `plan_pos_oids` and `table_names`. `ctid_resolvers` is rewired on the worker by
/// `VisibilityCtidResolverRule` running on the rebuilt plan, so we don't ship it.
#[derive(Clone, PartialEq, ::prost::Message)]
struct VisibilityFilterProto {
    /// Repeated `plan_position` values. Parallel to `heap_oids` / `table_names`.
    #[prost(uint64, repeated, tag = "1")]
    pub plan_positions: Vec<u64>,
    /// Repeated heap OID values. Parallel to `plan_positions`.
    #[prost(uint32, repeated, tag = "2")]
    pub heap_oids: Vec<u32>,
    /// EXPLAIN-display table names. Parallel to `plan_positions`.
    #[prost(string, repeated, tag = "3")]
    pub table_names: Vec<String>,
}

/// Wire form for [`crate::scan::tantivy_lookup_exec::TantivyLookupExec`].
///
/// `decoders` is rebuilt on the worker by looking up each `PhysicalDeferredField`'s
/// `(col_idx, canonical_column)` in the input schema. `ffhelpers` is reconstructed per-index
/// from the `indexrelids` list (one `FFHelper` per index OID that any deferred field
/// references).
#[derive(Clone, PartialEq, ::prost::Message)]
struct TantivyLookupProto {
    /// One entry per deferred field carried in the lookup.
    #[prost(message, repeated, tag = "1")]
    pub fields: Vec<PhysicalDeferredFieldProto>,
    /// Unique index relids referenced by any field; used to seed the `ffhelpers` map.
    #[prost(uint32, repeated, tag = "2")]
    pub indexrelids: Vec<u32>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
struct PhysicalDeferredFieldProto {
    #[prost(uint64, tag = "1")]
    pub col_idx: u64,
    #[prost(string, tag = "2")]
    pub display_name: String,
    #[prost(bool, tag = "3")]
    pub is_bytes: bool,
    /// Encoded form of `CanonicalColumn` (indexrelid + field name + variant tag).
    #[prost(message, optional, tag = "4")]
    pub canonical: Option<CanonicalColumnProto>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
struct CanonicalColumnProto {
    #[prost(uint32, tag = "1")]
    pub indexrelid: u32,
    /// `FFIndex` is a `usize` slot in the per-segment fast-field cache. Stored as `u64` on the
    /// wire to side-step platform-dependent serialization of `usize`.
    #[prost(uint64, tag = "2")]
    pub ff_index: u64,
}

/// Wire form for [`crate::scan::segmented_topk_exec::SegmentedTopKExec`].
///
/// Carries the LIMIT, the sort spec, the deferred sort columns, and the deduped index relids
/// referenced by those deferred columns (same FFHelper-reconstruction pattern as
/// [`TantivyLookupProto`]). Schema/properties are derived from the wrapped child input via the
/// codec's `inputs` slice.
#[derive(Clone, PartialEq, ::prost::Message)]
struct SegmentedTopKProto {
    /// LIMIT N — the upper bound on rows the topk emits.
    #[prost(uint64, tag = "1")]
    pub fetch: u64,
    /// Sort keys in priority order. Each entry is a full `PhysicalSortExpr`-equivalent
    /// (expression bytes + asc/desc + nulls placement).
    #[prost(message, repeated, tag = "2")]
    pub sort_keys: Vec<SortKeyProto>,
    /// Deferred sort columns whose values are resolved from tantivy fast fields rather than the
    /// input batch. Empty for the common case (no deferred columns in the sort).
    #[prost(message, repeated, tag = "3")]
    pub deferred_columns: Vec<DeferredSortColumnProto>,
    /// Unique index relids referenced by `deferred_columns`. Worker uses this to seed the
    /// `FFHelper` map — same placeholder caveat as `TantivyLookupProto::indexrelids`.
    #[prost(uint32, repeated, tag = "4")]
    pub indexrelids: Vec<u32>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
struct SortKeyProto {
    /// `datafusion_proto::protobuf::PhysicalExprNode` encoded as bytes via
    /// `serialize_physical_expr(&expr, &codec).encode_to_vec()`. Carries the full expression
    /// tree, so it covers `Column` and any other `PhysicalExpr` variant a sort key might be.
    #[prost(bytes, tag = "1")]
    pub expr_bytes: Vec<u8>,
    /// True if ASC; false if DESC.
    #[prost(bool, tag = "2")]
    pub ascending: bool,
    /// True if NULLS FIRST; false if NULLS LAST.
    #[prost(bool, tag = "3")]
    pub nulls_first: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
struct DeferredSortColumnProto {
    /// Position of this column in the sort key list (parallel index into the lex ordering).
    #[prost(uint64, tag = "1")]
    pub sort_col_idx: u64,
    #[prost(message, optional, tag = "2")]
    pub canonical: Option<CanonicalColumnProto>,
}

/// Wire form for [`crate::scan::execution_plan::PgSearchScanPlan`].
///
/// The scan's tantivy `Vec<ScanState>` doesn't survive serialization — workers rebuild it from
/// `indexrelid` + the per-segment recipe parameters via the same `PgSearchTableProvider`
/// construction path the logical-codec hits today. The encoded form carries only what
/// `PgSearchTableProvider::scan_state_for_partition` needs.
#[derive(Clone, PartialEq, ::prost::Message)]
struct PgSearchScanProto {
    /// Index relation OID. Workers re-open via `pg_sys::RelationIdGetRelation`.
    #[prost(uint32, tag = "1")]
    pub indexrelid: u32,
    /// Output schema (prost-encoded `DfSchema`).
    #[prost(message, optional, tag = "2")]
    pub schema: Option<DfSchema>,
    /// JSON-encoded `SearchQueryInput` (already serde-derived in tree).
    #[prost(string, tag = "3")]
    pub query_for_display_json: String,
    /// Per-partition recipe payloads. Each partition's recipe (Eager / Lazy / Prefetched) is
    /// encoded as JSON for forward-compatibility; switching to a typed message is a follow-up if
    /// the JSON cost shows up in benches.
    #[prost(string, repeated, tag = "4")]
    pub partition_recipes_json: Vec<String>,
    /// Sort spec, if the leader declared one. Encoded shape matches `SortKeyProto`.
    #[prost(message, optional, tag = "5")]
    pub sort_order: Option<SortKeyProto>,
    /// JSON-encoded `Vec<DeferredField>` (already serde-derived).
    #[prost(string, tag = "6")]
    pub deferred_fields_json: String,
    /// `deferred_ctid_plan_position`, encoded as `u32::MAX` for `None` and the position otherwise.
    #[prost(uint32, tag = "7")]
    pub deferred_ctid_plan_position: u32,
    /// Dynamic filters as serialized `PhysicalExprNode`s, in plan order.
    #[prost(bytes, repeated, tag = "8")]
    pub dynamic_filters: Vec<Vec<u8>>,
    /// `serde_json` bytes of the `PgSearchTableProvider` that built this scan on the leader.
    /// Workers deserialize, inject runtime context (segment IDs by `plan_position`,
    /// `parallel_state`, `expr_context`/`planstate`), and replay `scan_inner` to rebuild the
    /// per-partition `Vec<ScanState>`. Empty for scans that didn't go through the production
    /// `create_scan` path (test fixtures).
    #[prost(bytes, tag = "9")]
    pub table_provider_json: Vec<u8>,
}

const DEFERRED_CTID_NONE: u32 = u32::MAX;

/// The real physical codec. Replaces `PgSearchPhysicalCodecStub` once the dispatch-flip PR
/// lands; until then it's exercised exclusively via round-trip unit tests in this module.
///
/// Stateless. Worker-side reconstruction reaches for tantivy state via the `TaskContext` and PG
/// catalog at decode time, the same way [`crate::scan::codec::PgSearchExtensionCodec`] does in
/// the logical path today.
#[derive(Debug, Default)]
pub struct PgSearchPhysicalCodec;

impl PhysicalExtensionCodec for PgSearchPhysicalCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        inputs: &[Arc<dyn ExecutionPlan>],
        ctx: &TaskContext,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let envelope = CustomExecProto::decode(buf).map_err(|e| {
            DataFusionError::Internal(format!("PgSearchPhysicalCodec: prost decode failed: {e}"))
        })?;
        let variant = envelope.variant.ok_or_else(|| {
            DataFusionError::Internal(
                "PgSearchPhysicalCodec: CustomExecProto missing variant".into(),
            )
        })?;
        match variant {
            #[cfg(not(test))]
            custom_exec_proto::Variant::VisibilityFilter(p) => {
                decode_visibility_filter(p, inputs, ctx)
            }
            #[cfg(test)]
            custom_exec_proto::Variant::VisibilityFilter(_) => {
                Err(DataFusionError::NotImplemented(
                    "VisibilityFilterExec decode is excluded from cargo-test/llvm-cov builds \
                     (pulls in `pg_sys::Oid` whose CGU references `PG_exception_stack`); \
                     covered by `cargo pgrx test`"
                        .into(),
                ))
            }
            custom_exec_proto::Variant::TantivyLookup(p) => decode_tantivy_lookup(p, inputs, ctx),
            custom_exec_proto::Variant::SegmentedTopK(p) => decode_segmented_topk(p, inputs, ctx),
            #[cfg(not(test))]
            custom_exec_proto::Variant::PgSearchScan(p) => decode_pgsearch_scan(p, inputs, ctx),
            #[cfg(test)]
            custom_exec_proto::Variant::PgSearchScan(_) => Err(DataFusionError::NotImplemented(
                "PgSearchScan decode is excluded from cargo-test builds; covered by \
                 `cargo pgrx test`"
                    .into(),
            )),
        }
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        let variant = match node.name() {
            #[cfg(not(test))]
            "VisibilityFilterExec" => custom_exec_proto::Variant::VisibilityFilter(
                encode_visibility_filter(node.as_ref())?,
            ),
            #[cfg(test)]
            "VisibilityFilterExec" => {
                return Err(DataFusionError::NotImplemented(
                    "VisibilityFilterExec encode is excluded from cargo-test/llvm-cov builds; \
                     covered by `cargo pgrx test`"
                        .into(),
                ));
            }
            "TantivyLookupExec" => {
                custom_exec_proto::Variant::TantivyLookup(encode_tantivy_lookup(node.as_ref())?)
            }
            "SegmentedTopKExec" => {
                custom_exec_proto::Variant::SegmentedTopK(encode_segmented_topk(node.as_ref())?)
            }
            #[cfg(not(test))]
            "PgSearchScan" => {
                custom_exec_proto::Variant::PgSearchScan(encode_pgsearch_scan(node.as_ref())?)
            }
            #[cfg(test)]
            "PgSearchScan" => {
                return Err(DataFusionError::NotImplemented(
                    "PgSearchScan encode is excluded from cargo-test builds; covered by \
                     `cargo pgrx test`"
                        .into(),
                ));
            }
            other => {
                return Err(DataFusionError::Internal(format!(
                    "PgSearchPhysicalCodec::try_encode: unrecognized custom exec {other}"
                )));
            }
        };
        let envelope = CustomExecProto {
            variant: Some(variant),
        };
        envelope.encode(buf).map_err(|e| {
            DataFusionError::Internal(format!("PgSearchPhysicalCodec: prost encode failed: {e}"))
        })
    }

    // UDAF round-trip for the same built-in aggregates the logical codec handles
    // (`scan/codec.rs::PgSearchExtensionCodec::try_decode_udaf`). Shipped physical plans that
    // wrap an `AggregateExec` with `count`/`sum`/etc. reach this path; without it the worker
    // errors with "PhysicalExtensionCodec is not provided for aggregate function {name}". The
    // encoder writes the UDF name as bytes; the decoder rebuilds via DataFusion's built-in
    // factories. Encoding is name-only because all five aggregates are stateless built-ins.
    fn try_decode_udaf(
        &self,
        name: &str,
        _buf: &[u8],
    ) -> Result<Arc<datafusion::logical_expr::AggregateUDF>> {
        use datafusion::functions_aggregate as dfa;
        match name {
            "min" => Ok(dfa::min_max::min_udaf()),
            "max" => Ok(dfa::min_max::max_udaf()),
            "count" => Ok(dfa::count::count_udaf()),
            "sum" => Ok(dfa::sum::sum_udaf()),
            "avg" => Ok(dfa::average::avg_udaf()),
            _ => Err(DataFusionError::NotImplemented(format!(
                "PgSearchPhysicalCodec::try_decode_udaf: aggregate '{name}' not registered"
            ))),
        }
    }

    fn try_encode_udaf(
        &self,
        node: &datafusion::logical_expr::AggregateUDF,
        buf: &mut Vec<u8>,
    ) -> Result<()> {
        buf.extend_from_slice(node.name().as_bytes());
        Ok(())
    }
}

// ---------- VisibilityFilterExec ----------
//
// `VisibilityFilterExec` carries `Vec<(usize, pg_sys::Oid)>` in its `plan_pos_oids` field, and
// the codec has to construct those `pg_sys::Oid` values via `pg_sys::Oid::from(u32)` at decode
// time. `pgrx_pg_sys`'s `Oid` definition lives in the same compilation unit as the FFI guard
// (`pg_guard_ffi_boundary_impl`), which references the PG-runtime global `PG_exception_stack`.
// Coverage builds (`cargo llvm-cov`) don't dead-strip uncalled production code, so the CGU's
// reference to `PG_exception_stack` becomes a hard link error — PG provides that symbol at
// extension-load time, but a standalone test binary doesn't have it.
//
// Plain `cargo test` works (DCE drops the uncalled symbols), but CI uses llvm-cov for PR runs.
// Gate these halves out of test builds the same way `decode_pgsearch_scan` is gated; the codec
// is covered by `cargo pgrx test` instead. A future commit can refactor `VisibilityFilterExec`
// to take `Vec<(usize, u32)>` and do the Oid wrap internally, after which this gate comes off.

#[cfg(not(test))]
fn encode_visibility_filter(node: &dyn ExecutionPlan) -> Result<VisibilityFilterProto> {
    use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
    let vf = node
        .as_any()
        .downcast_ref::<VisibilityFilterExec>()
        .ok_or_else(|| {
            DataFusionError::Internal(
                "encode_visibility_filter: node named VisibilityFilterExec but downcast failed"
                    .into(),
            )
        })?;
    let pairs = vf.plan_pos_oids();
    let (plan_positions, heap_oids): (Vec<u64>, Vec<u32>) = pairs
        .iter()
        .map(|(p, oid)| (*p as u64, oid.to_u32()))
        .unzip();
    Ok(VisibilityFilterProto {
        plan_positions,
        heap_oids,
        table_names: vf.table_names().to_vec(),
    })
}

#[cfg(not(test))]
fn decode_visibility_filter(
    proto: VisibilityFilterProto,
    inputs: &[Arc<dyn ExecutionPlan>],
    _ctx: &TaskContext,
) -> Result<Arc<dyn ExecutionPlan>> {
    use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;
    let input = inputs.first().cloned().ok_or_else(|| {
        DataFusionError::Internal(
            "decode_visibility_filter: VisibilityFilterExec must have exactly one input".into(),
        )
    })?;
    if proto.plan_positions.len() != proto.heap_oids.len()
        || proto.plan_positions.len() != proto.table_names.len()
    {
        return Err(DataFusionError::Internal(
            "decode_visibility_filter: plan_positions / heap_oids / table_names length mismatch"
                .into(),
        ));
    }
    let plan_pos_oids: Vec<(usize, pg_sys::Oid)> = proto
        .plan_positions
        .iter()
        .zip(proto.heap_oids.iter())
        .map(|(p, oid)| (*p as usize, pg_sys::Oid::from(*oid)))
        .collect();
    let exec = VisibilityFilterExec::new(input, plan_pos_oids, proto.table_names)?;
    Ok(Arc::new(exec))
}

// ---------- TantivyLookupExec ----------

fn encode_tantivy_lookup(node: &dyn ExecutionPlan) -> Result<TantivyLookupProto> {
    use crate::scan::tantivy_lookup_exec::TantivyLookupExec;
    let exec = node
        .as_any()
        .downcast_ref::<TantivyLookupExec>()
        .ok_or_else(|| {
            DataFusionError::Internal(
                "encode_tantivy_lookup: node named TantivyLookupExec but downcast failed".into(),
            )
        })?;
    let fields: Vec<PhysicalDeferredFieldProto> = exec
        .deferred_fields()
        .iter()
        .map(|d| PhysicalDeferredFieldProto {
            col_idx: d.col_idx as u64,
            display_name: d.display_name.clone(),
            is_bytes: d.is_bytes,
            canonical: Some(CanonicalColumnProto {
                indexrelid: d.canonical.indexrelid,
                ff_index: d.canonical.ff_index as u64,
            }),
        })
        .collect();
    // Unique index relids referenced by any field; used to seed the `ffhelpers` map on the
    // worker. `deferred_fields()` is already deduplicated by `(col_idx, canonical)` but the
    // canonical's `indexrelid` is not — multiple deferred columns can sit on the same index.
    let mut indexrelids: Vec<u32> = exec
        .deferred_fields()
        .iter()
        .map(|d| d.canonical.indexrelid)
        .collect();
    indexrelids.sort_unstable();
    indexrelids.dedup();
    Ok(TantivyLookupProto {
        fields,
        indexrelids,
    })
}

fn decode_tantivy_lookup(
    proto: TantivyLookupProto,
    inputs: &[Arc<dyn ExecutionPlan>],
    _ctx: &TaskContext,
) -> Result<Arc<dyn ExecutionPlan>> {
    use crate::api::HashMap;
    use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper};
    use crate::scan::tantivy_lookup_exec::{PhysicalDeferredField, TantivyLookupExec};

    let input = inputs.first().cloned().ok_or_else(|| {
        DataFusionError::Internal(
            "decode_tantivy_lookup: TantivyLookupExec must have exactly one input".into(),
        )
    })?;
    let deferred_fields: Vec<PhysicalDeferredField> = proto
        .fields
        .into_iter()
        .map(|f| {
            let canonical_proto = f.canonical.ok_or_else(|| {
                DataFusionError::Internal(
                    "decode_tantivy_lookup: deferred field missing canonical column".into(),
                )
            })?;
            Ok(PhysicalDeferredField {
                col_idx: f.col_idx as usize,
                display_name: f.display_name,
                is_bytes: f.is_bytes,
                canonical: CanonicalColumn {
                    indexrelid: canonical_proto.indexrelid,
                    ff_index: canonical_proto.ff_index as usize,
                },
            })
        })
        .collect::<Result<_>>()?;

    // FFHelper map population: at production runtime (worker thread inside a PG backend) we'd
    // open each `indexrelid` via `pg_sys::RelationIdGetRelation`, build a `SearchIndexReader`,
    // and call `FFHelper::with_fields(reader, deferred fields on that index)`. That requires
    // PG-backend context which `TaskContext` doesn't surface, so the actual wiring lands in the
    // dispatch-flip commit alongside a `with_index_open` helper. For the codec PR scope (this
    // commit) we hand back an empty `FFHelper` per relid — round-trip tests verify field shape,
    // and any pre-dispatch-flip caller that tries to *execute* the decoded plan will trip a
    // clear error on the first lookup miss.
    let ffhelpers: HashMap<u32, std::sync::Arc<FFHelper>> = proto
        .indexrelids
        .into_iter()
        .map(|relid| (relid, std::sync::Arc::new(FFHelper::empty())))
        .collect();

    let exec = TantivyLookupExec::new(input, deferred_fields, ffhelpers)?;
    Ok(Arc::new(exec))
}

// ---------- SegmentedTopKExec ----------

fn encode_segmented_topk(node: &dyn ExecutionPlan) -> Result<SegmentedTopKProto> {
    use crate::scan::segmented_topk_exec::SegmentedTopKExec;
    use datafusion_proto::physical_plan::to_proto::serialize_physical_expr;

    let exec = node
        .as_any()
        .downcast_ref::<SegmentedTopKExec>()
        .ok_or_else(|| {
            DataFusionError::Internal(
                "encode_segmented_topk: node named SegmentedTopKExec but downcast failed".into(),
            )
        })?;

    // We're encoding sort-key inner exprs (`PhysicalExpr`). They're never custom; the user codec
    // only matters for UDFs reachable through those exprs, and the topk's sort keys are
    // columns/literals/standard scalars. Pass a `PgSearchPhysicalCodec` instance through to
    // serialize_physical_expr regardless — it's the contract.
    let inner_codec = PgSearchPhysicalCodec;
    let sort_keys: Vec<SortKeyProto> = exec
        .sort_exprs()
        .iter()
        .map(|s| {
            let expr_node = serialize_physical_expr(&s.expr, &inner_codec)?;
            let mut expr_bytes = Vec::new();
            expr_node.encode(&mut expr_bytes).map_err(|e| {
                DataFusionError::Internal(format!(
                    "encode_segmented_topk: PhysicalExprNode encode failed: {e}"
                ))
            })?;
            Ok::<_, DataFusionError>(SortKeyProto {
                expr_bytes,
                ascending: !s.options.descending,
                nulls_first: s.options.nulls_first,
            })
        })
        .collect::<Result<_>>()?;

    let deferred_columns: Vec<DeferredSortColumnProto> = exec
        .deferred_columns()
        .iter()
        .map(|d| DeferredSortColumnProto {
            sort_col_idx: d.sort_col_idx as u64,
            canonical: Some(CanonicalColumnProto {
                indexrelid: d.canonical.indexrelid,
                ff_index: d.canonical.ff_index as u64,
            }),
        })
        .collect();

    let mut indexrelids: Vec<u32> = exec
        .deferred_columns()
        .iter()
        .map(|d| d.canonical.indexrelid)
        .collect();
    indexrelids.sort_unstable();
    indexrelids.dedup();

    Ok(SegmentedTopKProto {
        fetch: exec.k() as u64,
        sort_keys,
        deferred_columns,
        indexrelids,
    })
}

fn decode_segmented_topk(
    proto: SegmentedTopKProto,
    inputs: &[Arc<dyn ExecutionPlan>],
    ctx: &TaskContext,
) -> Result<Arc<dyn ExecutionPlan>> {
    use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper};
    use crate::scan::segmented_topk_exec::{DeferredSortColumn, SegmentedTopKExec};
    use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
    use datafusion_proto::physical_plan::from_proto::parse_physical_expr;
    use datafusion_proto::protobuf::PhysicalExprNode;

    let input = inputs.first().cloned().ok_or_else(|| {
        DataFusionError::Internal(
            "decode_segmented_topk: SegmentedTopKExec must have exactly one input".into(),
        )
    })?;
    let input_schema = input.schema();
    let inner_codec = PgSearchPhysicalCodec;

    let mut sort_exprs_vec: Vec<PhysicalSortExpr> = Vec::with_capacity(proto.sort_keys.len());
    for sk in &proto.sort_keys {
        let expr_node = PhysicalExprNode::decode(sk.expr_bytes.as_slice()).map_err(|e| {
            DataFusionError::Internal(format!(
                "decode_segmented_topk: PhysicalExprNode decode failed: {e}"
            ))
        })?;
        let expr = parse_physical_expr(&expr_node, ctx, &input_schema, &inner_codec)?;
        sort_exprs_vec.push(PhysicalSortExpr {
            expr,
            options: arrow_schema::SortOptions {
                descending: !sk.ascending,
                nulls_first: sk.nulls_first,
            },
        });
    }
    let sort_exprs = LexOrdering::new(sort_exprs_vec).ok_or_else(|| {
        DataFusionError::Internal(
            "decode_segmented_topk: empty sort key list (SegmentedTopK requires at least one)"
                .into(),
        )
    })?;

    let deferred_columns: Vec<DeferredSortColumn> = proto
        .deferred_columns
        .into_iter()
        .map(|d| {
            let canonical = d.canonical.ok_or_else(|| {
                DataFusionError::Internal(
                    "decode_segmented_topk: deferred sort column missing canonical".into(),
                )
            })?;
            Ok(DeferredSortColumn {
                sort_col_idx: d.sort_col_idx as usize,
                canonical: CanonicalColumn {
                    indexrelid: canonical.indexrelid,
                    ff_index: canonical.ff_index as usize,
                },
            })
        })
        .collect::<Result<_>>()?;

    // Same placeholder caveat as `decode_tantivy_lookup` — the real `FFHelper` for each index
    // gets wired by the dispatch-flip commit, which has access to PG-backend state. Until then
    // we hand back an empty helper that's structurally correct but won't service runtime
    // fast-field lookups.
    let _ = proto.indexrelids; // currently unused at decode time; reserved for the rebuild path.
    let ffhelper = std::sync::Arc::new(FFHelper::empty());

    let exec = SegmentedTopKExec::new(
        input,
        sort_exprs,
        deferred_columns,
        ffhelper,
        proto.fetch as usize,
    );
    Ok(Arc::new(exec))
}

// ---------- PgSearchScanPlan ----------
//
// `PgSearchScanPlan` transitively pulls in `SearchQueryInput`, which derives `pgrx::PostgresType`
// and references PG runtime globals (`CacheMemoryContext`, etc.). A plain cargo-test binary
// can't resolve those symbols at link time, so the codec halves for this exec are gated out of
// test builds and covered by `cargo pgrx test` instead.

#[cfg(not(test))]
fn encode_pgsearch_scan(node: &dyn ExecutionPlan) -> Result<PgSearchScanProto> {
    use crate::scan::execution_plan::PgSearchScanPlan;
    use datafusion_proto::physical_plan::to_proto::serialize_physical_expr;

    let scan = node
        .as_any()
        .downcast_ref::<PgSearchScanPlan>()
        .ok_or_else(|| {
            DataFusionError::Internal(
                "encode_pgsearch_scan: node named PgSearchScan but downcast failed".into(),
            )
        })?;

    // Schema: lift from the plan's PlanProperties and convert to the DfSchema proto used by
    // datafusion_proto. Workers reconstruct an arrow `SchemaRef` from this on decode.
    let arrow_schema = scan.schema();
    let df_schema_ref = std::sync::Arc::new(
        datafusion::common::DFSchema::try_from(arrow_schema.as_ref().clone()).map_err(|e| {
            DataFusionError::Internal(format!(
                "encode_pgsearch_scan: arrow schema -> DFSchema failed: {e}"
            ))
        })?,
    );
    let schema_proto: DfSchema = (&df_schema_ref).try_into().map_err(|e| {
        DataFusionError::Internal(format!(
            "encode_pgsearch_scan: DFSchema -> proto failed: {e}"
        ))
    })?;

    // JSON-encode the serde-derived fields. Keeps the wire format flexible while we iterate on
    // the dispatch flip; once the shape stabilises we can switch to dedicated prost messages if
    // the JSON cost shows up in profiles.
    let query_for_display_json = serde_json::to_string(scan.query_for_display()).map_err(|e| {
        DataFusionError::Internal(format!(
            "encode_pgsearch_scan: SearchQueryInput JSON encode failed: {e}"
        ))
    })?;
    let sort_order = scan
        .sort_order()
        .map(|so| {
            let canon = CanonicalColumnProto {
                indexrelid: 0,
                ff_index: 0,
            };
            // SortByField doesn't fit the col-idx-based SortKeyProto; fall back to a JSON
            // payload encoded as expr_bytes for now. Decode is the inverse.
            let bytes = serde_json::to_vec(so).map_err(|e| {
                DataFusionError::Internal(format!(
                    "encode_pgsearch_scan: SortByField JSON encode failed: {e}"
                ))
            })?;
            let _ = canon;
            Ok::<_, DataFusionError>(SortKeyProto {
                expr_bytes: bytes,
                ascending: !matches!(
                    so.direction,
                    crate::postgres::options::SortByDirection::Desc
                ),
                nulls_first: false,
            })
        })
        .transpose()?;

    let deferred_fields_json = serde_json::to_string(scan.deferred_fields()).map_err(|e| {
        DataFusionError::Internal(format!(
            "encode_pgsearch_scan: DeferredField JSON encode failed: {e}"
        ))
    })?;

    let deferred_ctid_plan_position = scan
        .deferred_ctid_plan_position()
        .map(|p| p as u32)
        .unwrap_or(DEFERRED_CTID_NONE);

    let inner_codec = PgSearchPhysicalCodec;
    let dynamic_filters: Vec<Vec<u8>> = scan
        .dynamic_filters()
        .iter()
        .map(|expr| {
            let proto = serialize_physical_expr(expr, &inner_codec)?;
            let mut bytes = Vec::new();
            proto.encode(&mut bytes).map_err(|e| {
                DataFusionError::Internal(format!(
                    "encode_pgsearch_scan: dynamic_filter encode failed: {e}"
                ))
            })?;
            Ok::<_, DataFusionError>(bytes)
        })
        .collect::<Result<_>>()?;

    // `partition_recipes_json` is now superseded by the `table_provider_json` blob below:
    // workers replay `scan_inner` to rebuild the per-partition `Vec<ScanState>` from scratch
    // rather than reading per-partition recipes off the wire. Left as an empty Vec on the
    // proto so deployments mid-rollout don't see a wire-shape mismatch; can be retired once
    // the dispatch-flip lands in stable.
    let partition_recipes_json: Vec<String> = Vec::new();

    // Ship the serialized `PgSearchTableProvider` populated by `create_scan` (Phase 2b). If
    // the scan didn't go through `create_scan` (test fixtures), the field is `None` and we
    // ship an empty Vec — workers detect the empty payload and skip reconstruction.
    let table_provider_json = scan
        .serialized_table_provider()
        .map(|s| s.to_vec())
        .unwrap_or_default();

    Ok(PgSearchScanProto {
        indexrelid: scan.indexrelid,
        schema: Some(schema_proto),
        query_for_display_json,
        partition_recipes_json,
        sort_order,
        deferred_fields_json,
        deferred_ctid_plan_position,
        dynamic_filters,
        table_provider_json,
    })
}

#[cfg(not(test))]
fn decode_pgsearch_scan(
    proto: PgSearchScanProto,
    _inputs: &[Arc<dyn ExecutionPlan>],
    _ctx: &TaskContext,
) -> Result<Arc<dyn ExecutionPlan>> {
    use crate::scan::execution_plan::PgSearchScanPlan;

    // Rebuild the arrow schema from the proto DfSchema.
    let df_schema_proto = proto.schema.ok_or_else(|| {
        DataFusionError::Internal("decode_pgsearch_scan: missing schema field".into())
    })?;
    let df_schema: datafusion::common::DFSchema = (&df_schema_proto).try_into().map_err(|e| {
        DataFusionError::Internal(format!(
            "decode_pgsearch_scan: DFSchema proto -> DFSchema failed: {e}"
        ))
    })?;
    let arrow_schema: SchemaRef = std::sync::Arc::new(df_schema.as_arrow().clone());

    let query_for_display = serde_json::from_str(&proto.query_for_display_json).map_err(|e| {
        DataFusionError::Internal(format!(
            "decode_pgsearch_scan: SearchQueryInput JSON decode failed: {e}"
        ))
    })?;

    let sort_order = proto
        .sort_order
        .map(|sk| {
            serde_json::from_slice::<crate::postgres::options::SortByField>(&sk.expr_bytes).map_err(
                |e| {
                    DataFusionError::Internal(format!(
                        "decode_pgsearch_scan: SortByField JSON decode failed: {e}"
                    ))
                },
            )
        })
        .transpose()?;

    let deferred_fields: Vec<crate::scan::late_materialization::DeferredField> =
        serde_json::from_str(&proto.deferred_fields_json).map_err(|e| {
            DataFusionError::Internal(format!(
                "decode_pgsearch_scan: DeferredField JSON decode failed: {e}"
            ))
        })?;

    let deferred_ctid_plan_position = if proto.deferred_ctid_plan_position == DEFERRED_CTID_NONE {
        None
    } else {
        Some(proto.deferred_ctid_plan_position as usize)
    };

    // Same FFHelper-placeholder caveat as the other execs: dispatch-flip wires the real one.
    let ffhelper = if !deferred_fields.is_empty() || deferred_ctid_plan_position.is_some() {
        Some(std::sync::Arc::new(
            crate::index::fast_fields_helper::FFHelper::empty(),
        ))
    } else {
        None
    };

    // Empty states placeholder. Phase 2d wires the actual reconstruction (deserialize
    // `proto.table_provider_json`, inject runtime context, call `scan_inner`).
    let states: Vec<crate::scan::execution_plan::ScanState> = Vec::new();
    let _ = proto.partition_recipes_json; // superseded by `table_provider_json` (Phase 2c).
    let _ = proto.dynamic_filters; // dynamic filters are re-pushed via FilterPushdown after construction; nothing to do here yet.
    let _ = proto.table_provider_json; // consumed by the reconstruction landing in Phase 2d.

    let plan = PgSearchScanPlan::new(
        states,
        arrow_schema,
        query_for_display,
        sort_order.as_ref(),
        deferred_fields,
        ffhelper,
        proto.indexrelid,
        deferred_ctid_plan_position,
    );

    Ok(Arc::new(plan))
}

// ---------- shared helpers ----------

#[allow(dead_code)]
fn schema_to_proto(_schema: &SchemaRef) -> Result<DfSchema> {
    // Filled in alongside the PgSearchScan codec — the proto type is what `datafusion_proto`
    // already produces, but constructing it requires running the proto's field encoder over the
    // arrow schema. Leaving the wrapper here so the call site is ergonomic when it gets wired.
    Err(DataFusionError::NotImplemented(
        "schema_to_proto helper not yet implemented".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::{DataType, Field, Schema};
    use datafusion::physical_plan::empty::EmptyExec;

    fn empty_input(n_cols: usize) -> Arc<dyn ExecutionPlan> {
        let fields: Vec<Field> = (0..n_cols)
            .map(|i| Field::new(format!("c{i}"), DataType::Int64, true))
            .collect();
        Arc::new(EmptyExec::new(Arc::new(Schema::new(fields))))
    }

    fn codec() -> PgSearchPhysicalCodec {
        PgSearchPhysicalCodec
    }

    fn ctx() -> Arc<TaskContext> {
        Arc::new(TaskContext::default())
    }

    #[test]
    fn try_decode_rejects_empty_buffer() {
        let codec = codec();
        let err = codec.try_decode(&[], &[], &ctx()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("missing variant") || msg.contains("prost decode failed"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn try_encode_rejects_unknown_node_name() {
        let codec = codec();
        let plan = empty_input(1);
        let mut buf = Vec::new();
        let err = codec.try_encode(plan, &mut buf).unwrap_err();
        assert!(
            err.to_string().contains("unrecognized custom exec"),
            "unexpected error: {err}"
        );
    }

    // VisibilityFilterExec round-trip lives in `tests::visibility_filter_round_trip` once the
    // exec's constructor accepts construction without PG state (currently `new()` requires
    // running on a backend thread for `EquivalenceProperties`; the input we build here is an
    // `EmptyExec`, so it should be safe). Smoke check first:

    // Note: `pgsearch_scan_round_trip` lives in the pgrx-gated `scan::tests` module rather than
    // here — `PgSearchScanPlan::new` indirectly touches PG symbols (via `SearchQueryInput`'s
    // `PostgresType` derive), and a plain cargo-test binary can't resolve those at link time.
    // The cargo-test surface checks proto-level round-tripping for the simpler execs that don't
    // pull PG in; full PgSearchScan coverage runs under `cargo pgrx test`.

    #[test]
    fn segmented_topk_round_trip() {
        use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper};
        use crate::scan::segmented_topk_exec::{DeferredSortColumn, SegmentedTopKExec};
        use arrow_schema::SortOptions;
        use datafusion::physical_expr::expressions::Column;
        use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
        use std::sync::Arc;

        let input_schema = Arc::new(Schema::new(vec![
            Field::new("score", DataType::Float64, false),
            Field::new("title", DataType::Utf8, true),
        ]));
        let input: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(input_schema));

        let col_score =
            Arc::new(Column::new("score", 0)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>;
        let sort_exprs = LexOrdering::new(vec![PhysicalSortExpr {
            expr: Arc::clone(&col_score),
            options: SortOptions {
                descending: true,
                nulls_first: false,
            },
        }])
        .unwrap();
        let deferred_columns = vec![DeferredSortColumn {
            sort_col_idx: 0,
            canonical: CanonicalColumn {
                indexrelid: 16384,
                ff_index: 2,
            },
        }];
        let exec = Arc::new(SegmentedTopKExec::new(
            Arc::clone(&input),
            sort_exprs.clone(),
            deferred_columns.clone(),
            Arc::new(FFHelper::empty()),
            10,
        )) as Arc<dyn ExecutionPlan>;

        let codec = codec();
        let mut buf = Vec::new();
        codec.try_encode(Arc::clone(&exec), &mut buf).unwrap();

        let decoded = codec
            .try_decode(&buf, std::slice::from_ref(&input), &ctx())
            .unwrap();

        let topk = decoded
            .as_any()
            .downcast_ref::<SegmentedTopKExec>()
            .expect("decoded plan is a SegmentedTopKExec");
        assert_eq!(topk.k(), 10);
        let got_sort = topk.sort_exprs();
        assert_eq!(got_sort.len(), 1);
        assert!(got_sort[0].options.descending);
        assert!(!got_sort[0].options.nulls_first);
        let got_deferred = topk.deferred_columns();
        assert_eq!(got_deferred.len(), 1);
        assert_eq!(got_deferred[0].sort_col_idx, 0);
        assert_eq!(got_deferred[0].canonical.indexrelid, 16384);
        assert_eq!(got_deferred[0].canonical.ff_index, 2);
    }

    #[test]
    fn tantivy_lookup_round_trip() {
        use crate::index::fast_fields_helper::CanonicalColumn;
        use crate::scan::tantivy_lookup_exec::{PhysicalDeferredField, TantivyLookupExec};
        use arrow_schema::UnionFields;
        use std::sync::Arc;

        // TantivyLookupExec's `build_schema_and_decoders` only treats Union-typed input columns
        // as candidate decode targets; pass-through columns ignore matching deferred entries.
        // Build an input schema where col 1 is a Union so the lookup picks it up.
        let union_fields = UnionFields::try_new(
            vec![0_i8],
            vec![Field::new("inner", DataType::UInt64, false)],
        )
        .expect("UnionFields::try_new");
        let union_dt = DataType::Union(union_fields, arrow_schema::UnionMode::Dense);
        let input_schema = Arc::new(Schema::new(vec![
            Field::new("ctid", DataType::UInt64, false),
            Field::new("body", union_dt, true),
        ]));
        let input: Arc<dyn ExecutionPlan> = Arc::new(EmptyExec::new(input_schema));

        let deferred_fields = vec![PhysicalDeferredField {
            col_idx: 1,
            display_name: "body".to_string(),
            is_bytes: false,
            canonical: CanonicalColumn {
                indexrelid: 16384,
                ff_index: 0,
            },
        }];
        let exec = Arc::new(
            TantivyLookupExec::new(
                Arc::clone(&input),
                deferred_fields.clone(),
                crate::api::HashMap::default(),
            )
            .unwrap(),
        ) as Arc<dyn ExecutionPlan>;

        let codec = codec();
        let mut buf = Vec::new();
        codec.try_encode(Arc::clone(&exec), &mut buf).unwrap();

        let decoded = codec
            .try_decode(&buf, std::slice::from_ref(&input), &ctx())
            .unwrap();

        let lookup = decoded
            .as_any()
            .downcast_ref::<TantivyLookupExec>()
            .expect("decoded plan is a TantivyLookupExec");
        let got = lookup.deferred_fields();
        assert_eq!(got.len(), deferred_fields.len());
        for (g, e) in got.iter().zip(deferred_fields.iter()) {
            assert_eq!(g.col_idx, e.col_idx);
            assert_eq!(g.display_name, e.display_name);
            assert_eq!(g.is_bytes, e.is_bytes);
            assert_eq!(g.canonical.indexrelid, e.canonical.indexrelid);
            assert_eq!(g.canonical.ff_index, e.canonical.ff_index);
        }
        // Empty FFHelper placeholder stashed for every unique indexrelid the encoded form
        // referenced; dispatch-flip commit replaces this with PG-state-backed reconstruction.
        assert!(lookup.ffhelper(16384).is_some());
    }

    // Note: `visibility_filter_round_trip` lives in the pgrx-gated test surface (`cargo pgrx
    // test`) rather than here. The codec's `decode_visibility_filter` constructs `pg_sys::Oid`
    // values, which pulls in a pgrx_pg_sys compilation unit that transitively references
    // `PG_exception_stack` — a PG-runtime global that plain `cargo test` / `cargo llvm-cov`
    // can't link. Same reason `pgsearch_scan_round_trip` is integration-tested only.
}
