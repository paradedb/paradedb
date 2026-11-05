# MS MARCO pg_vectorsearch without replication

This ablation starts at the completed #7197 benchmark (`1eebb236510767edb54ce219368970ee260cbdce`)
and retains its pg_vectorsearch revision `1bc1c77b2a07611d57c8a1eae767939bd1e39ee1`,
PostgreSQL settings, snapshot, tool pins, 100 held-out queries and strict recall@10/100
R90/R95/R99 sweeps. The only index configuration change is disabling both
`boundary_epsilon` and `soar_lambda`. Disabling only the boundary gate would still
allow SOAR secondary assignments. Each vector must have one posting in this run.

After the index build, CI verifies both catalog options are zero and the physical
posting count equals 113,520,750 with no dead entries. This read occurs before query
warmups and measurements. Diagnostics preserve the verification JSON, index build
metrics, recall sweeps, plans and k6 results. Unreached targets remain explicit.

Use only the `benchmark-msmarco-pg_vectorsearch` label for one launch. The reference
run 41576241539 is complete and must not be restarted. The existing heap backup
`20261102-224214F` is restored from
`s3://paradedb-benchmark/snapshots/msmarco/v1/113m/pg_backrest/`; no new snapshot is built.

The worker pool remains eight to match #7197; build/query limits are eight, with
12 GiB shared buffers and maintenance memory. The concurrent pg_search experiment
uses a worker pool of 32, as did its own reference #7260. Keep that distinction in
build-time comparisons.

The shared publisher reports achieved recall and k6 p50/p95/p99/QPS on this PR and
the dashboard. Summed RSS includes shared mappings. No rolling baseline is updated.
