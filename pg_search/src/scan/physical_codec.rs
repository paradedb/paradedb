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
/// Carries the LIMIT, the column-encoded sort spec, and the deferred-field plumbing the topk
/// needs to track. Schema/properties are derived from the wrapped child input via the codec's
/// `inputs` slice.
#[derive(Clone, PartialEq, ::prost::Message)]
struct SegmentedTopKProto {
    /// LIMIT N — the upper bound on rows the topk emits.
    #[prost(uint64, tag = "1")]
    pub fetch: u64,
    /// Sort key columns + ASC/DESC, in priority order.
    #[prost(message, repeated, tag = "2")]
    pub sort_keys: Vec<SortKeyProto>,
    /// Optional dynamic-filter threshold column expression; encoded as a serialized
    /// `datafusion_proto::physical_plan::PhysicalExprNode`.
    #[prost(bytes, optional, tag = "3")]
    pub dynamic_filter_threshold: Option<Vec<u8>>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
struct SortKeyProto {
    /// Column index in the input schema.
    #[prost(uint64, tag = "1")]
    pub col_idx: u64,
    /// True if ASC; false if DESC.
    #[prost(bool, tag = "2")]
    pub ascending: bool,
    /// True if NULLS FIRST; false if NULLS LAST.
    #[prost(bool, tag = "3")]
    pub nulls_first: bool,
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
            custom_exec_proto::Variant::VisibilityFilter(p) => {
                decode_visibility_filter(p, inputs, ctx)
            }
            custom_exec_proto::Variant::TantivyLookup(p) => decode_tantivy_lookup(p, inputs, ctx),
            custom_exec_proto::Variant::SegmentedTopK(p) => decode_segmented_topk(p, inputs, ctx),
            custom_exec_proto::Variant::PgSearchScan(p) => decode_pgsearch_scan(p, inputs, ctx),
        }
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        let variant = match node.name() {
            "VisibilityFilterExec" => custom_exec_proto::Variant::VisibilityFilter(
                encode_visibility_filter(node.as_ref())?,
            ),
            "TantivyLookupExec" => {
                custom_exec_proto::Variant::TantivyLookup(encode_tantivy_lookup(node.as_ref())?)
            }
            "SegmentedTopKExec" => {
                custom_exec_proto::Variant::SegmentedTopK(encode_segmented_topk(node.as_ref())?)
            }
            "PgSearchScan" => {
                custom_exec_proto::Variant::PgSearchScan(encode_pgsearch_scan(node.as_ref())?)
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
}

// ---------- VisibilityFilterExec ----------

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

fn encode_segmented_topk(_node: &dyn ExecutionPlan) -> Result<SegmentedTopKProto> {
    Err(DataFusionError::NotImplemented(
        "encode_segmented_topk: SegmentedTopKExec codec not yet implemented".into(),
    ))
}

fn decode_segmented_topk(
    _proto: SegmentedTopKProto,
    _inputs: &[Arc<dyn ExecutionPlan>],
    _ctx: &TaskContext,
) -> Result<Arc<dyn ExecutionPlan>> {
    Err(DataFusionError::NotImplemented(
        "decode_segmented_topk: SegmentedTopKExec codec not yet implemented".into(),
    ))
}

// ---------- PgSearchScanPlan ----------

fn encode_pgsearch_scan(_node: &dyn ExecutionPlan) -> Result<PgSearchScanProto> {
    Err(DataFusionError::NotImplemented(
        "encode_pgsearch_scan: PgSearchScanPlan codec not yet implemented".into(),
    ))
}

fn decode_pgsearch_scan(
    _proto: PgSearchScanProto,
    _inputs: &[Arc<dyn ExecutionPlan>],
    _ctx: &TaskContext,
) -> Result<Arc<dyn ExecutionPlan>> {
    Err(DataFusionError::NotImplemented(
        "decode_pgsearch_scan: PgSearchScanPlan codec not yet implemented".into(),
    ))
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

    #[test]
    fn tantivy_lookup_round_trip() {
        use crate::index::fast_fields_helper::CanonicalColumn;
        use crate::scan::tantivy_lookup_exec::{PhysicalDeferredField, TantivyLookupExec};
        use arrow_schema::UnionFields;
        use std::sync::Arc;

        // TantivyLookupExec's `build_schema_and_decoders` only treats Union-typed input columns
        // as candidate decode targets; pass-through columns ignore matching deferred entries.
        // Build an input schema where col 1 is a Union so the lookup picks it up.
        let union_fields = UnionFields::new(
            vec![0_i8],
            vec![Field::new("inner", DataType::UInt64, false)],
        );
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

    #[test]
    fn visibility_filter_round_trip() {
        use crate::postgres::customscan::joinscan::visibility_filter::VisibilityFilterExec;

        let input = empty_input(2);
        let plan_pos_oids = vec![
            (0_usize, pg_sys::Oid::from(16384_u32)),
            (1_usize, pg_sys::Oid::from(16385_u32)),
        ];
        let table_names = vec!["posts".to_string(), "comments".to_string()];
        let exec = Arc::new(
            VisibilityFilterExec::new(input.clone(), plan_pos_oids.clone(), table_names.clone())
                .unwrap(),
        ) as Arc<dyn ExecutionPlan>;

        let codec = codec();
        let mut buf = Vec::new();
        codec.try_encode(Arc::clone(&exec), &mut buf).unwrap();

        let decoded = codec
            .try_decode(&buf, std::slice::from_ref(&input), &ctx())
            .unwrap();

        let vf = decoded
            .as_any()
            .downcast_ref::<VisibilityFilterExec>()
            .expect("decoded plan is a VisibilityFilterExec");
        assert_eq!(vf.plan_pos_oids(), plan_pos_oids.as_slice());
        assert_eq!(vf.table_names(), table_names.as_slice());
    }
}
