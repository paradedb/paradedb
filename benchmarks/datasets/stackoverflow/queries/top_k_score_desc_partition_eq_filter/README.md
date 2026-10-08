# TopK Score DESC with a Partition-Key Equality Filter, Partitioned vs Unpartitioned (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with `owner_user_id = 22656`. `owner_user_id` is a
  `partition_by` column, so on the partitioned index, segments whose `owner_user_id` range excludes
  22656 are skipped using per-segment statistics.

## Query Info

- `partitioned.sql` queries `stackoverflow_posts` (index partitioned by `id,owner_user_id`).
- `unpartitioned.sql` queries `stackoverflow_posts_unpartitioned`: the same rows and the same index
  without `partition_by`, created in `indexes/bm25.sql`.
- `owner_user_id = 22656` is Jon Skeet, one of the most active answerers on Stack Overflow, chosen so
  the filter is expected to match rows at every sampled dataset size (check the row count of the
  first run).
