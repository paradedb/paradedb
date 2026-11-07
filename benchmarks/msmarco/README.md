# MS MARCO absolute probe benchmark

Benchmarks the supplied pg_search commit on the 113M-row MS MARCO corpus,
using the published heap and ground-truth snapshots. The sweep selects
`paradedb.vector_cluster_max_probe` in absolute cluster-work units independently
for top-10 and top-100 at R90/R95/R99. It starts at 100 units and searches
between 1 and 1,000,000 units, with adaptive probing enabled.

The workflow records the extension commit and configuration in its diagnostics.
Run it with `workflow_dispatch` and pass the PR commit as `commit`; benchmark
scripts and configuration come from the workflow ref.
