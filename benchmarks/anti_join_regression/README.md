# Anti-join regression reproduction

The regression analysis, including matched plan excerpts, repeated timing
distributions, code links, and the evidence-status matrix, is tracked in
[#5999](https://github.com/paradedb/paradedb/issues/5999).

This is a synthetic reproduction of a `NOT EXISTS` anti-join over two
BM25-indexed tables. It is not the original user's dataset.

The fixture uses:

- PostgreSQL 17.7 on Apple Silicon;
- optimized (`--release`) builds of ParadeDB 0.24.3 and 0.25.3;
- 100,000 catalog rows and 75,000 ownership rows;
- six BM25 segments per index, created by committed insert waves;
- one pgbench client, 20 warm-up transactions, then 100 measured transactions;
- `max_parallel_workers_per_gather = 4` and `work_mem = '1GB'`.

## Setup

Install the release under test, create a fresh database, and run:

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -f benchmarks/anti_join_regression/setup_heap.sql \
  -f benchmarks/anti_join_regression/install_index.sql \
  -f benchmarks/anti_join_regression/load_data.sql
```

Verify that both indexes have multiple independently claimable segments:

```sql
SELECT 'library' AS idx, count(*) AS segments
FROM paradedb.index_info('anti_bench_library_bm25')
UNION ALL
SELECT 'owned', count(*)
FROM paradedb.index_info('anti_bench_owned_bm25');
```

Both indexes had six segments in the recorded runs.

## Plan evidence

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -f benchmarks/anti_join_regression/explain.sql
```

0.24.3 used `Gather Merge` over `Parallel Custom Scan (ParadeDB Join
Scan)`, with four workers planned and launched. The selective cases emitted
the issue-4152 suboptimal-partitioning warning.

0.25.3 used `DistributedExec` and reported `MPP Launch: workers=4`. The warmed
`first_frame` median was 44.315 ms, although that aggregate bucket does not
isolate worker plan decoding, distributed execution, and transport.

Single `EXPLAIN ANALYZE` execution times were:

| case | 0.24.3 | 0.25.3 | ratio |
| --- | ---: | ---: | ---: |
| browse, no search | 38.664 ms | 90.643 ms | 2.34x |
| browse + `dragon` | 36.753 ms | 79.284 ms | 2.16x |
| browse + `love` | 33.664 ms | 80.674 ms | 2.40x |

## Warmed pgbench evidence

For each query, warm it and then collect 100 transactions:

```bash
pgbench "$DATABASE_URL" -n -c 1 -j 1 -t 20 \
  -f benchmarks/anti_join_regression/browse_dragon.sql
pgbench "$DATABASE_URL" -n -c 1 -j 1 -t 100 \
  -f benchmarks/anti_join_regression/browse_dragon.sql
```

Repeat with `browse_all.sql` and `browse_love.sql`.

| case | 0.24.3 | 0.25.3 | ratio |
| --- | ---: | ---: | ---: |
| browse, no search | 36.632 ms | 53.816 ms | 1.47x |
| browse + `dragon` | 35.547 ms | 67.168 ms | 1.89x |
| browse + `love` | 36.421 ms | 67.680 ms | 1.86x |

The 0.25.3 `dragon` measurement was repeated immediately after the MPP-off
control and remained at 67.168 ms.

## MPP-off control

First capture both execution paths in one session:

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -f benchmarks/anti_join_regression/explain_dragon_controls.sql
```

The first plan must contain `DistributedExec` and `MPP Launch`. The second must
contain neither marker and must remain a `ParadeDB Join Scan`.

```bash
pgbench "$DATABASE_URL" -n -c 1 -j 1 -t 20 \
  -f benchmarks/anti_join_regression/browse_dragon_mpp_off.sql
pgbench "$DATABASE_URL" -n -c 1 -j 1 -t 100 \
  -f benchmarks/anti_join_regression/browse_dragon_mpp_off.sql
```

On 0.25.3, `dragon` measured 67.168 ms with MPP enabled and 20.924 ms with
MPP disabled: MPP was 3.21x slower. The disabled plan contained neither
`DistributedExec` nor `MPP Launch` and completed as a serial ParadeDB Join
Scan.

These are performance measurements, not pg_regress assertions. Fixed latency
thresholds would be too hardware-sensitive for a correctness test; the plan
markers are the stable execution-path assertions.
