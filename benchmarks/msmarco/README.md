# MS MARCO adaptive probing benchmark

Benchmark-only candidate based on ParadeDB main `17f0af99e` and Tantivy main
`d1d4de7c4`, with the latest adaptive probing from #7279 / Tantivy #378.
Quantized routing is enabled and the router rerank factor is 2. Boundary
replication is disabled in this arm.

The `benchmark-msmarco-pg_search` label runs the 113,520,750-vector corpus using
the existing heap snapshot and 100 held-out queries. It sweeps adaptive effort
from 0.05 to 10 at strict R90/R95/R99 for top-10 and top-100, then measures k6
latencies at reached targets. Per-query work ceilings in `probe_caps.json`
are the R99 fixed-probe settings from #7260: 0.106 and 0.079. These ceilings
also apply at R90/R95 and are carried into the plans and k6 sessions.

The candidate keeps eight target segments, training ratio 0.32, router minimum
beam fraction 0.5, APS enabled, 12 GiB buffers/maintenance memory, eight requested
workers and a 32-process worker pool. The workflow verifies quantized routing
before the sweep and at every reached operating point. Diagnostics retain the
configuration, probe caps, tool revisions, plans and sweep reports. No mainline
baseline is updated by a PR run.
