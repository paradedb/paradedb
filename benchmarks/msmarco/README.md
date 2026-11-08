# MS MARCO global routing with SOAR replication

Stacks SOAR replication and V6 exact-row storage onto #7295, with centroid-parallel
SOAR assignment to retain build parallelism at MS MARCO cluster counts. The run
reuses the 113,520,750-row heap backup `20261102-224214F`, builds eight segments,
and verifies two postings per vector before querying.

The sweep selects `paradedb.vector_cluster_max_probe` in absolute cluster-work
units independently for top-10 and top-100 at R90/R95/R99. It starts at 100 units
and searches between 1 and 1,000,000 units, with adaptive probing enabled.

The CI result compares against the completed, non-replicated #7295 run
[42419496747](https://github.com/paradedb/paradedb/actions/runs/42419496747).
`compare_selected.py` verifies the baseline revision, query hash, and configuration
before presenting achieved recall, probe budgets, EXPLAIN buffer means, raw k6
percentiles and observed-window QPS, plus build metrics. Unreached targets and
missing measurements remain explicit. The benchmark configuration differs only
by enabling `spill`; engine changes also include V6 exact-row storage.

Apply `benchmark-msmarco-pg_search` once to this PR to run its immutable head and
publish the result comment. The workflow saves extension/tool revisions, query
hash, persisted index options, segment statistics, plans, and raw measurements.
