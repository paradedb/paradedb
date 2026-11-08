# MS MARCO global routing with SOAR replication

Stacks SOAR replication and V6 exact-row storage onto #7295, with centroid-parallel
SOAR assignment to retain build parallelism at MS MARCO cluster counts. The run
reuses the 113,520,750-row heap backup `20261102-224214F`, builds eight segments,
and verifies four postings per vector before querying. Each additional assignment
minimizes distance plus the cumulative alignment penalty from all prior residuals.
The SOAR weight remains 2; full-precision rows are still stored once.

The sweep selects `paradedb.vector_cluster_max_probe` in absolute cluster-work
units independently for top-10 and top-100 at R90/R95/R99. It starts at 100 units
and searches between 1 and 1,000,000 units, with adaptive probing enabled.

The CI result compares against the completed two-copy SOAR run
[42477854482](https://github.com/paradedb/paradedb/actions/runs/42477854482).
`compare_selected.py` verifies the baseline revision, query hash, and configuration
before presenting achieved recall, probe budgets, EXPLAIN buffer means, raw k6
percentiles and observed-window QPS, plus build metrics. Unreached targets and
missing measurements remain explicit. The benchmark configuration is identical;
the helper verifies two versus four postings per vector. Both runs use V6 exact-row
storage. Four-copy assignment performs four matrix passes, reusing the same bounded
scratch matrices as the two-copy kernel.

Dispatch `benchmark-pg_search-benchmarker-msmarco.yml` on the PR branch to rerun
its head and publish benchmark artifacts without posting a PR comment. The workflow saves extension/tool revisions, query
hash, persisted index options, segment statistics, plans, and raw measurements.
