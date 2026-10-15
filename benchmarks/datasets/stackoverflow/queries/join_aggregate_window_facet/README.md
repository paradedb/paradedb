# Multi-Facet Window Aggregates on JOIN

- **Join**: stackoverflow_posts -> comments
- **Description**: Uses `pdb.agg(...) OVER ()` window functions to retrieve Top K hits
  alongside global facet counts across multiple dimensions.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code' selectivity on stackoverflow_posts.body: ~75%

## Comparison Strategy

`pdb.agg(...) OVER ()` over a join is only supported by ParadeDB's Join Scan, so native PostgreSQL (`postgres.sql`) and Base Scan (`basescan.sql`) variants are not applicable.
