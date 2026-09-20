# Stack Overflow Benchmark Suite

This directory contains schemas, indexes, and queries for benchmarking ParadeDB on the Stack Overflow dataset, against best practice Postgres indexes.

## Comparison Strategy

Queries are split into three families with different comparison strategies:

- Top-K:
  - Single table - Measures Block-Max WAND optimizations, single-worker behavior, and tiebreakers across selectivity tiers without external baseline comparisons.
    - Note that we currently do not compare to GIN here.
  - Joins - PostgreSQL can be competitive when supported by appropriate indexes (often finding efficient nested loop join plans or early-termination paths). B-tree and GIN indexes are defined in `indexes/bm25.sql`.
- Aggregates (`basescan.sql`):
  - Not compared against PostgreSQL. Standard PostgreSQL aggregates over GIN full-text indexes require scanning full posting lists and heap tuples, taking tens of seconds to minutes on large datasets. Instead, aggregate queries compare ParadeDB's DataFusion pushdown (`aggregate_scan.sql`) against PostgreSQL aggregation executing atop ParadeDB's columnar base scan (`basescan.sql`, with `paradedb.enable_aggregate_custom_scan = off`).
