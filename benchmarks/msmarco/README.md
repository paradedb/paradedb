# MS MARCO 113M quantized routing benchmark

This benchmark restores the existing 113,520,750-row heap snapshot from
`s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/` and builds a new
vector-only pg_search index. It does not generate a new snapshot.

The candidate combines ParadeDB main `f0f9ff86518af8bb66070b4056fcd52ad0b69f8a`
with the pg_search integration from PR #7128 and Tantivy #357 at
`70d25cbf4f938eeb26f284bfa7ee5aa26996d810`. This Tantivy head already includes
current Tantivy main `f0eac37ac81804c08baa0e9beabc8f89dabfa57e`.

`vector_router_quantization = true` persists the router representation in the
index. Parameterized top-10 and top-100 preflights must exercise quantized
routing and exact centroid reranking before recall sweeps begin. The current
main search budget is `paradedb.vector_cluster_max_probe`; the old experimental
adaptive-effort settings are not used.

The run measures unfiltered recall@10 and recall@100 over 100 held-out query
vectors at R90/R95/R99, then measures each reached operating point with k6.
It retains build time/size, worker counts, sampled RSS, plans, routing counters,
recall reports and latency artifacts. Summed RSS includes shared mappings.
The existing shared publisher posts CI results to the triggering PR and dashboard.

The restored snapshot's worker pool is raised to 32 before building, matching
current CI configuration; parallel build and query limits remain eight.
The earlier reference runs used a worker pool of eight, so build-time comparisons
must account for that configuration difference.

Add `benchmark-msmarco-pg_search` to the draft PR to launch the single candidate.
The workflow removes the label after starting. The tools remain pinned to the
previous comparison's pdbench and Benchmarker commits.
