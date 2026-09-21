# Stack Overflow Benchmark Suite

This directory contains schemas, indexes, and queries for benchmarking ParadeDB on the Stack Overflow dataset, against best practice Postgres indexes.

## Comparison Strategy

Queries are split into three families with different comparison strategies:

- Top-K:
  - Single table - Measures Block-Max WAND optimizations, single-worker behavior, and tiebreakers across selectivity tiers without external baseline comparisons.
    - Note that we currently do not compare to GIN here.
  - Joins - PostgreSQL can be competitive when supported by appropriate indexes (often finding efficient nested loop join plans or early-termination paths). B-tree and GIN indexes are defined in `indexes/bm25.sql`. Highly uncompetitive PostgreSQL join baselines (such as `join_top_k_score_desc_low_selectivity/postgres.sql`, which took ~9.0 seconds per iteration and required a dedicated 410 MB GIN index on `stackoverflow_posts.body`) were omitted to keep benchmark runtimes fast; see that query directory's `README.md` for a query plan breakdown.
- Aggregates:
  - Not compared against PostgreSQL. Standard PostgreSQL aggregates over GIN full-text indexes require scanning full posting lists and heap tuples, taking tens of seconds to minutes on large datasets. ParadeDB executes these queries faster than PostgreSQL even when using its base scan (`paradedb.enable_aggregate_custom_scan = off`). To keep benchmark execution fast, `basescan.sql` variants are retained only for representative queries (`count_filter`, `aggregate_topk_count`, `join_aggregate_count`, and `cardinality/basescan_distinct.sql`), while the remaining aggregate queries benchmark DataFusion pushdown (`aggregate_scan.sql`).
