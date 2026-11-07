# Date Histogram with Range Filter (Single Table, Tantivy)

- **Join**: None (single table)
- **Description**: `pdb.agg` `date_histogram` (`fixed_interval = "1d"`) over `stackoverflow_posts.creation_date` filtered by a full-text predicate (`body ||| 'code'`), a scalar equality predicate (`post_type_id = 1`), and a recent timestamp range (`creation_date >= '2015-01-01'`).
  Because `stackoverflow_posts_idx` is partitioned by `id, owner_user_id` (`target_segment_count = 32`) and `id` is chronologically correlated with `creation_date`, the `creation_date >= '2015-01-01'` filter matches zero rows in older `id`-partitioned segments. This exercises segment pruning before estimation and aggregate execution (`aggregate_nworkers`) and segment-level visibility checking on skewed/partitioned segments.

## Query Info (statistics from 20M dataset; larger or smaller datasets may vary):

- `'code'` selectivity on `stackoverflow_posts.body`: ~75.12% (15.04M matches).
- `post_type_id = 1` selectivity on `stackoverflow_posts.post_type_id`: ~40%.
- `creation_date >= '2015-01-01'` selectivity on `stackoverflow_posts.creation_date`: ~26.98% (5.40M matches).

## Comparison Strategy

Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. `aggregate_scan.sql` benchmarks the MVCC-verified `ParadeDB Aggregate Scan` path (marked `@mvcc_sensitive` so it also runs on dirtied heap pages), while `pdb_agg_no_mvcc.sql` provides the MVCC-disabled `pdb.agg(..., false)` baseline.
