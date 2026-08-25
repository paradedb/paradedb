# Quantization Phase B repository mapping

Date: 2026-08-24

This is the §1.1 docs-only mapping checkpoint. No storage-format or runtime code
has been changed. Paths beginning with `../tantivy` refer to the sibling
ParadeDB Tantivy checkout.

## Generic-to-actual mapping

| Plan name | Actual owner | Actual type/function and contract |
|---|---|---|
| Vec-file lifecycle | Tantivy | `../tantivy/src/vector/plugin.rs`: `VectorPlugin::{create_writer, merge}` owns both `.vec` and `.centroids`; newly flushed segments are flat and clustering is a merge-time transform. |
| Vec-file writer | Tantivy | `../tantivy/src/vector/flat/writer.rs`: `FlatVecWriter::serialize` writes flat segments; `../tantivy/src/vector/ivf/plugin.rs`: `merge_ivf` writes IVF posting rows. Both use `CompositeWrite::for_field_with_idx`. |
| Vec-file reader | Tantivy | `../tantivy/src/vector/index_reader.rs`: `VectorIndexReader::open` opens `.vec` slots and the optional `.centroids` sidecar; `rows_slice` is the existing fp32 row payload. |
| Storage row order / row-to-doc mapping | Tantivy | `../tantivy/src/vector/ivf/plugin.rs`: `AssignedVector` rows are sorted by `(cluster, target_doc_id)` and written with an explicit `IdMap` in `.vec` slot 0, parallel to fp32 rows in slot 1. |
| Posting / cluster offset table | Tantivy | `../tantivy/src/vector/ivf/index.rs`: `IvfIndex::cluster_offsets` is the prefix-sum table from `.centroids` slot 1; `IvfIndex::cluster_range` resolves a cluster's contiguous `.vec` row range. No second posting table is needed. |
| Centroid storage | Tantivy | `../tantivy/src/vector/ivf/index.rs` and `../tantivy/src/vector/ivf/plugin.rs`: `IvfIndex::{serialize_centroids, centroid_bytes}` own `.centroids` slot 0; slots 1/2/3 are offsets, routing graph, and bounds. |
| Vector format version and slot registry | Tantivy | `../tantivy/src/vector/header.rs`: `VectorFileVersion` is currently V2; `vec_slot::{ID_MAP, ROWS}` are 0/1 and `centroid_slot::{CENTROIDS, OFFSETS, GRAPH, BOUNDS}` are 0/1/2/3. Phase B requires a V3 bump and new `.vec` slots after slot 1. |
| Composite-section placement | Tantivy | `../tantivy/src/directory/composite_file.rs`: `CompositeWrite::for_field_with_idx` records the current stream offset. It has no alignment primitive; quantized code-section alignment must be implemented as explicit padding before opening each code slot. |
| Segment metadata / component files | pg_search | `pg_search/src/postgres/storage/block.rs`: `SegmentMetaEntryImmutable::{vec, centroids}` stores the Postgres `FileEntry` handles; `pg_search/src/index/writer/segment_component.rs` and `reader/segment_component.rs` bridge Tantivy component I/O to linked Postgres blocks. |
| Index-level options input | pg_search | `pg_search/src/postgres/options.rs`: `BM25IndexOptionsData` / `BM25IndexOptions` own index reloptions; `pg_search/src/index/mod.rs::index_settings` converts them into Tantivy `IndexSettings`. |
| Persisted per-index metadata | pg_search + Tantivy | `pg_search/src/index/directory/utils.rs::{save_settings, load_metas}` persists one serialized Tantivy `IndexSettings` in the index metapage. Quantization layers, seeds, grids, metric, norm policy, and format version belong there and are then shared by all segments. |
| Merge path | Tantivy | `../tantivy/src/vector/plugin.rs::VectorPlugin::merge` selects `merge_flat` or `merge_ivf`; `../tantivy/src/vector/ivf/plugin.rs::merge_ivf` currently trains new centroids and reassigns every live vector before writing the target segment. |
| Query entry / exact fallback | Tantivy | `../tantivy/src/vector/backend.rs::VectorBackend::top_n_by` branches on `VectorIndexReader::index()`: flat `None` uses the brute-force `exact_top_n`; IVF `Some` retains cluster routing and the probe budget. Within IVF, `max_scan_levels = 0` or missing calibration uses full-precision scoring for routed posting rows, while a positive active prefix uses `quantized_top_n`. |
| Existing query preparation | Tantivy | `../tantivy/src/vector/prepared.rs`: `PreparedQuery<T>` owns metric-specific exact-query state. Phase B's immutable quantized `IndexCtx` and `QueryCtx` belong beside this type and are constructed once per segment query. |
| Existing IVF routing and work budget | Tantivy | `../tantivy/src/vector/backend.rs`: `approximate_top_n` / cluster scanning already route clusters, apply centroid-bound skips, and charge a probe work budget. Plane-1 admission and budget termination extend this path. |
| Probe instrumentation | Tantivy + pg_search | `../tantivy/src/vector/backend.rs::ProbeStats` is serialized into per-segment `Segment Info` by `pg_search/src/index/reader/index.rs::probe_stats_to_segment_info`; `pg_search/src/index/reader/io_stats.rs` attaches component buffer hits/reads. Phase B counters and timings extend these two existing channels. |
| Current vector GUCs | pg_search | `pg_search/src/gucs.rs` owns the IVF probe controls and `max_scan_levels`, consumed by `pg_search/src/index/reader/index.rs` and passed into Tantivy's per-query vector context. Level 0 disables quantized scoring without bypassing IVF routing or its probe budget. |
| Kernel implementation | quant-kernels | `../quant-kernels` is the single kernel workspace. `cascade` supplies layered encode/split-form data, `sign-plane` supplies 1-bit scoring, and `grid-plane` supplies 2–4-bit grid scoring. |

## Storage facts established by the mapping

- IVF `.vec` storage order is posting-membership order, not distinct-document
  order. A replicated document appears in more than one cluster row.
- Posting boundaries already address every cluster-contiguous array as long as
  each quantized slot has one fixed-stride entry per IVF posting row.
- Initial flat segments have no centroid context. Under the proposed
  compatibility contract they remain exact/no-slot until an IVF merge creates
  quantized posting rows.
- The existing `.vec` composite can hold plane-separated code, scale, and
  constant slots without a second component file. Slot numbering and byte
  alignment are part of the V3 format freeze, not this mapping commit.

## Checkpoint resolutions

The four mapping conflicts were resolved on 2026-08-24:

1. IVF merges use the sound slow path only: every newly assigned posting row
   is re-encoded. Verbatim copying is deferred until the clustering layer has
   an assignment-preserving merge mode.
2. Codes, scales, and constants are stored per posting-membership row, matching
   fp32 slot 1 under replication. Query rerank deduplicates by doc ID.
3. The canonical quant-kernels crates relocate into the Tantivy fork as
   workspace members; pg_search continues to obtain them through its pinned
   Tantivy revision.
4. V1 accepts layer widths 1 through 4 and rejects wider entries.

The §1.3 mapping also found that one pg_search index may contain multiple
vector fields with different dimensions and metrics. Quantization metadata is
therefore one field-keyed list persisted in `IndexSettings`: still per-index
and shared by all segments, but with dimension-correct grids and metric policy
for each field. The frozen byte contract is in
`../tantivy/src/vector/FORMAT.md`.

## Execution dependency

The available host is Apple Silicon (M5). ARM measurements can be recorded
locally; the required x86 companion measurements need an x86 runner and cannot
be produced in this workspace.
