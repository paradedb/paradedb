# Partitioned build reproductions

These scripts build a `bm25` index with `partition_by` on a throwaway PostgreSQL cluster. They reproduce `assertion failed: end <= orig_range.end` on `pg_search` `0.26.3`.

## Setup

Install `pg_search` into a local PostgreSQL. Then, start a cluster.

```bash
./cluster.sh <pg bin dir> <data dir> <port>
```

The cluster preloads `pg_search` and starts with `RUST_BACKTRACE=1`. The server log is `<data dir>.log`. For a Rust panic, the `DETAIL` of the error has the stack.

## Scripts

| Script          | What it does                                                                                                                                                                                                   |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `shape.sql`     | Builds a wide index: 26 fields, a `jsonb` column indexed twice, and eight tenants with one at 81% of the rows. The memory budget is small, so each large partition flushes many segments and merges them once. |
| `large_key.sql` | Builds an index where one partition has a column with a dictionary over 4 GiB. With `top_level=0` the column is a JSON key. With `top_level=1` it is a top-level text field.                                   |
| `leaves.sh`     | Prints the largest partition's share of the rows, for 4, 8 and 16 partitions and several tenant orders.                                                                                                        |

```bash
PSQL="psql -h localhost -p <port> -U postgres -d postgres -X"

$PSQL -f shape.sql
$PSQL -v rows=7000000 -v mwm=32MB -v workers=1 \
  -v index_options="partition_by = 'tenant_id, ctid', target_segment_count = 4" -f shape.sql
$PSQL -f large_key.sql
$PSQL -v top_level=1 -f large_key.sql
./leaves.sh $PSQL
```

`large_key.sql` needs about 30 GB of disk and 10 minutes.

## Results on `0.26.3`

The runs used PostgreSQL 17.7 on macOS arm64 and a debug build of `v0.26.3`.

| Run                                                      | Result                                                                                                                                                         |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `shape.sql`, 2M rows, 16 partitions                      | The build passes. The four largest partitions have 20% of the rows each. Each one merges about 160 flushed segments in one merge.                              |
| `shape.sql`, 7M rows, 4 partitions, `shared_buffers=1GB` | The build fails with `no unpinned buffers available`, about a minute into two concurrent merges of 1,190 and 1,209 segments.                                   |
| Same run, `shared_buffers=3GB`                           | The build passes. The same two merges complete.                                                                                                                |
| `large_key.sql`, `top_level=0`                           | The build passes. The large partition is one segment of 6.59M rows and 9.5 GB, with 5.0 GB of columnar data. The aggregate on `key1` fails with the assertion. |
| `large_key.sql`, `top_level=1`                           | `CREATE INDEX CONCURRENTLY` fails with the assertion, right after the merge of the large partition.                                                            |

The build failure has this stack.

```text
tantivy_common::file_slice::FileSlice::slice          common/src/file_slice.rs:245
tantivy_common::file_slice::FileSlice::split          common/src/file_slice.rs:347
tantivy_sstable::dictionary::Dictionary::open         sstable/src/dictionary.rs:306
tantivy_columnar::column::serialize::open_column_bytes
tantivy_columnar::column::serialize::open_column_str
tantivy_columnar::dynamic_column::DynamicColumnHandle::open
pg_search::index::stats::plugin::column_stats
pg_search::index::stats::plugin::empirical_stats      index/stats/plugin.rs:186
pg_search::index::stats::plugin::write_stats          index/stats/plugin.rs:127
<StatsPlugin as tantivy::plugin::SegmentPlugin>::merge
tantivy::indexer::merger::IndexMerger::write          src/indexer/merger.rs:546
tantivy::indexer::segment_updater::merge_foreground
<SearchIndexMerger as Mergeable>::merge_segments      index/writer/index.rs:636
WorkerBuildState::merge_now                           postgres/build_parallel.rs:1062
WorkerBuildState::finish_partition                    postgres/build_parallel.rs:1316
WorkerBuildState::drain_partitioned                   postgres/build_parallel.rs:1260
BuildWorker::do_build                                 postgres/build_parallel.rs:489
```

The aggregate fails in the same `open_column_str` call. Its caller is `build_terms_or_cardinality_nodes` in `src/aggregation/agg_data.rs`.

`leaves.sh` prints these shares for tenants at 81%, 15.74%, 2.81% and 5 x 0.09% of the rows.

| Sort rank of the large tenant    | 4 partitions | 8 partitions | 16 partitions |
| -------------------------------- | ------------ | ------------ | ------------- |
| First                            | 40.5%        | 20.4%        | 10.5%         |
| Second, after the 15.74% one     | 42.2%        | 40.5%        | 20.4%         |
| Fourth, with the 15.74% one last | 42.0%        | 21.2%        | 10.9%         |
| Fifth                            | 40.7%        | 40.5%        | 20.4%         |
| Last                             | 25.3%        | 12.9%        | 10.4%         |

## What the results show

The columnar format stores the length of a column's dictionary as `u32`. A dictionary over 4 GiB gets a wrong length, and the next open of that column reads past the end of a slice.

A partitioned build writes one segment per partition. After the merge of a partition, the stats component opens every top-level column of the merged segment. It doesn't open JSON key columns. So, a top-level dictionary over 4 GiB fails the build, and a JSON key dictionary over 4 GiB fails the first query or merge that reads the key.

A unique `uuid` takes about 20 bytes in a dictionary. An index over 4M rows with only a `uuid` key has 23.8 bytes of columnar data per row. So, a `uuid` key field passes 4 GiB at about 220M rows in one partition.

The default partition count is `max(4, max_parallel_workers_per_gather)` times the number of `partition_by` fields. The build reads the setting from its own session. With two fields and a setting of 4 or less, the build makes 8 partitions. For the tenant shares above, 8 partitions can put 40.5% of the table in one partition. That partition passes 220M rows when the table has about 550M rows or more. With 16 partitions, the largest share is 20.4%.

## Not covered

- I didn't run these scripts against a build with `u64` lengths.
- I didn't build a `uuid` key at 220M rows. The limit comes from the measured bytes per row.
