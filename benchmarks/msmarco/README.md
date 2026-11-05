# MS MARCO 113M quantized routing with adaptive probing

This benchmark restores the existing 113,520,750-row heap snapshot from
`s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/` and builds a new
vector-only pg_search index. It does not generate a new snapshot.

The candidate starts at last night's quantized-router benchmark, ParadeDB
`e6c7dcc51677f3f20c3a1929f803fcb8da4810f3` (PR #7260). Its base is main
`f0f9ff86518af8bb66070b4056fcd52ad0b69f8a` with the pg_search integration from
PR #7128 and Tantivy #357 at `70d25cbf4f938eeb26f284bfa7ee5aa26996d810`.
The adaptive changes from Tantivy `462e2043e0465a4da13e42d2639921baff79fc67`
and the GUC integration from ParadeDB #7147 are applied on top. The combined
Tantivy revision is `ecddbac46d793cbc8ed56bc772f43517416d898d`, pinned in Cargo.toml and Cargo.lock.

`vector_router_quantization = true` persists the router representation in the
index. Parameterized top-10 and top-100 preflights must exercise quantized
routing and exact centroid reranking before recall sweeps begin.
`paradedb.vector_adaptive_probing = on` enables admission-based stopping and
periodic exact refinement across the globally ranked clusters.
`paradedb.vector_cluster_max_probe = 0` selects automatic adaptive headroom.
The sweep varies `paradedb.vector_search_effort` from 0.05 to 10.0, starting
at 1.25, with at most 20 points. Router rerank factor 1.0, minimum beam fraction
0.5 and router APS remain the same as last night. Search effort controls both
the routing window and stopping threshold; it is not a recall probability.

The run measures unfiltered recall@10 and recall@100 over 100 held-out query
vectors at R90/R95/R99, then measures each reached operating point with k6.
It retains build time/size, worker counts, sampled RSS, plans, routing counters,
recall reports and latency artifacts. Summed RSS includes shared mappings.
The existing shared publisher posts CI results to the triggering PR and dashboard.

The restored snapshot's worker pool is raised to 32 before building, matching
current CI configuration; parallel build and query limits remain eight.
Last night's run also used 32. The November 3 reference runs used eight, so
build-time comparisons against those runs must account for that difference.

Add `benchmark-msmarco-pg_search` to the draft PR to launch the single candidate.
The workflow removes the label after starting. The tools remain pinned to the
previous comparison's pdbench and Benchmarker commits.
