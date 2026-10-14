# Top-k by score, restricted by a join (low selectivity)

- **Join**: stackoverflow_posts -> users
- **Description**: This is a join that is blockmax-wand-eligible and uses the scores
  from a single table to drive the sort

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code developer' chosen deliberately to introduce a very large pool of candidates for the
  topk (~13.5% selectivity on the 20M dataset, ~2.7M joined matches; ~13.5K in the 100K dataset),
  to ensure we see the effect of the threshold tightening. Under database convention, this large
  candidate pool represents low selectivity (unselective filter).

## Native PostgreSQL Baseline Omission

The native PostgreSQL variant (`postgres.sql`) was dropped from this benchmark:

- Performance and setup: In PostgreSQL, this query ran in ~9.0 seconds per iteration (506x slower than ParadeDB's ~18 ms), and required a dedicated 410 MB GIN index on `stackoverflow_posts.body` that took 23 seconds to build on 1M rows for this single query alone.
- Query plan summary:
  - Scans GIN bitmaps on `stackoverflow_posts.body` (265k rows) and `users.about_me` (56k rows).
  - Executes a parallel bitmap heap scan with 11k lossy blocks, requiring 76k row rechecks.
  - Performs a parallel hash join yielding ~52k joined candidates across 5 workers.
  - Evaluates `ts_rank` on full `body` text across all 52k joined candidates before sorting with top-N heapsort. Because `ts_rank` cannot early-terminate across join candidates, it bottlenecks on scoring every joined row.
