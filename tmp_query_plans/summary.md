# Benchmark Results & Analysis: Hash vs Range Partitioned Joins

- Generated: 2026-09-27 19:39:52
- Environment: StackOverflow 20M dataset on `postgresql://localhost:28818/postgres`
- Methodology: Cold run measured on initial execution, followed by 3 interleaved warm runs per variant using `psql` and `\timing on`.
- Plan capture: `EXPLAIN (ANALYZE, BUFFERS, TIMING, COSTS, SUMMARY, VERBOSE)` for cases where `hash` outperformed `range`.

---

## 1. Summary Table

| Query                                   | Cold Hash (ms) | Cold Range (ms) | Warm Hash Med (ms) | Warm Range Med (ms) | Ratio (Range/Hash) | Faster Variant |
| :-------------------------------------- | :------------: | :-------------: | :----------------: | :-----------------: | :----------------: | :------------: |
| `join_top_k_score_desc_low_selectivity` |     107.2      |      165.4      |        67.3        |        168.2        |       2.50x        |     `HASH`     |
| `join_semi_filter`                      |     134.7      |      132.1      |        66.0        |        96.5         |       1.46x        |     `HASH`     |
| `join_aggregate_topk_count`             |     2987.2     |     1528.5      |       1469.6       |       1529.3        |       1.04x        |     `HASH`     |
| `join_aggregate_sort`                   |     1871.2     |     1588.6      |       1601.1       |       1625.4        |       1.02x        |     `HASH`     |
| `join_aggregate_window_facet`           |    50701.2     |     45605.6     |      44829.0       |       44718.4       |       1.00x        |  `EQUIVALENT`  |
| `join_aggregate_disjunctive_count`      |     614.7      |      498.6      |       586.5        |        497.7        |       0.85x        |    `RANGE`     |
| `join_aggregate_groupby`                |     600.6      |      392.6      |       506.0        |        395.0        |       0.78x        |    `RANGE`     |
| `join_aggregate_date_histogram`         |     502.9      |      349.3      |       447.1        |        346.5        |       0.78x        |    `RANGE`     |
| `join_aggregate_multi`                  |     434.8      |      321.1      |       421.1        |        314.5        |       0.75x        |    `RANGE`     |
| `join_aggregate_count`                  |     784.7      |      265.3      |       352.5        |        261.9        |       0.74x        |    `RANGE`     |
| `join_distinct_parent_sort`             |     4522.4     |     1625.7      |       2761.9       |       1862.4        |       0.67x        |    `RANGE`     |
| `join_disjunctive_score_sort`           |     1694.2     |      879.9      |       1646.9       |        893.9        |       0.54x        |    `RANGE`     |
| `join_conjunctive_score_sort`           |     466.9      |      218.8      |       214.7        |        100.0        |       0.47x        |    `RANGE`     |
| `join_disjunctive_local_sort`           |     1586.9     |      639.1      |       1531.0       |        687.6        |       0.45x        |    `RANGE`     |
| `join_foreign_filter_local_sort`        |     572.8      |      103.5      |       510.6        |        96.0         |       0.19x        |    `RANGE`     |
| `join_permissioned_search`              |     656.2      |      117.7      |       613.4        |        111.4        |       0.18x        |    `RANGE`     |

---

## 2. Analysis of Queries Where Hash is Faster

The 4 queries where `hash` outperformed `range` fall into three architectural categories:

### Category A: Single-Stage Local Range Join vs Cross-Stage Dynamic Filter Pushdown

#### Queries:

- `join_top_k_score_desc_low_selectivity`: Hash `67.3 ms` vs Range `168.2 ms` (2.50x faster with Hash)
- `join_semi_filter`: Hash `66.0 ms` vs Range `96.5 ms` (1.46x faster with Hash)

#### Architecture: The "Network" Boundary

In ParadeDB's distributed engine, the "network" boundary is between stage boundaries: data crossing between stages undergoes serialization across workers (`NetworkShuffleExec`, `NetworkBroadcastExec`). Within a single stage, execution is task-local and no network serialization occurs.

#### Stage Structures & Plan Comparison for `join_top_k_score_desc_low_selectivity`:

- **Range Plan (`Stage 1` only)**:
  - Range co-partitioning successfully collapsed the entire join into a **single stage** (`Stage 1`).
  - Both `users u` and `stackoverflow_posts p` were range-partitioned on `u.id` and `p.owner_user_id`.
  - The `HashJoinExec: mode=Partitioned` ran task-locally within `Stage 1` with **zero network serialization** between workers. Only the final 40 TopK candidates (5 per worker) crossed the stage boundary to the coordinator via `NetworkCoalesceExec`.
- **Hash Plan (`Stage 1` -> `Stage 2`)**:
  - Hash mode used two stages: `Stage 1` scanned `u` and broadcast it across workers via `NetworkBroadcastExec` (serialization between workers).
  - In `Stage 2`, `HashJoinExec: mode=CollectLeft` probed `p` against the broadcast table.

#### Why Hash Was Faster Despite Incurring Network Serialization:

1. **Cross-Stage Dynamic Filter Pushdown**:
   - Because Hash mode materialized and broadcast `u` in `Stage 1` before starting `Stage 2`, DataFusion built a global dynamic filter from all matching `u.id` values and pushed it down into `p`'s leaf scan in `Stage 2`.
   - In `hash_plan.txt`, dynamic filter pushdown pruned over **270,000 rows** directly at the index scan level (`rows_pruned={0:34.2K, 3:83.0K, 4:56.6K, ...}`).
   - As a result, `p` only emitted **54.4K rows** into the join, and the join had a **100% probe hit rate**.
   - In Range mode, both `u` and `p` scans executed concurrently within `Stage 1`. Without an eager build stage to construct a global filter before scanning `p`, `p` emitted **230.7K rows** into the join (with an ~80% probe miss rate).
2. **Whole Segments vs Partial Segment Range Evaluation**:
   - In Hash mode, `p`'s 32 physical segments were partitioned whole across the 8 workers (4 whole segments per worker). Each worker scanned its segments sequentially without evaluating range boundary predicates. Max scan compute was only ~10-28ms per worker.
   - In Range mode, enforcing range boundaries on `owner_user_id` forced workers to evaluate range boundary predicates on 4 to 6 partial segments per worker (because `p` is not physically sorted on `owner_user_id`). Scan compute on `p` jumped to ~75-120ms per worker.
3. **The Trade-Off**:
   - Broadcasting `u` in Hash mode only serialized ~2 MB of data across workers, taking ~5-10ms.
   - The savings from whole-segment scanning (~60ms faster scan compute) plus dynamic filter pruning of 270K rows (~40ms faster join compute) far outweighed the small serialization cost of the broadcast stage boundary.

#### Contrast with `join_semi_filter`:

- In `join_semi_filter`, the planner still chose a multi-stage broadcast join (`Stage 1` -> `Stage 2`) due to the subquery structure.
- However, Range mode still stamped range boundaries on `p`, forcing `p`'s scanner into partial-segment boundary evaluations (increasing `p` scan compute from 13ms to 48ms) without collapsing the stage boundary.

---

### Category B: Range Data Skew / The Power-Law Straggler Problem

#### Query:

- `join_aggregate_topk_count`: Hash `1469.6 ms` vs Range `1529.3 ms` (1.04x faster with Hash on warm runs)

#### Query Characteristics:

- `stackoverflow_posts p JOIN badges b ON b.user_id = p.owner_user_id GROUP BY b.name ORDER BY count(*) DESC LIMIT 10`.
- Generates ~148.9 million intermediate joined rows.

#### Plan Differences:

- In Hash mode, `owner_user_id` is hashed modulo 64 and distributed across workers. The maximum rows handled by any single worker was 30.6 million rows.
- In Range mode, `RangePartitioningRule` divided `user_id` into range intervals: `[-∞..210916)`, `[210916..516188)`, etc.
- On StackOverflow, early registered users (low `user_id`) possess a disproportionately large share of all badges. Range partition 0 (`[-∞..210916)`) on Task 0 had to process **72.11 million joined rows** alone, taking 481ms in join compute, while Task 7 processed only 1.53 million rows (a 47x workload skew).
- Task 0 became a severe pipeline straggler, holding back the parallel aggregate stage.

#### Hypothesis:

When joining on keys with power-law frequency distributions (like user badges), hash partitioning produces balanced worker loads across hash buckets. Range partitioning, by contrast, clusters dense low IDs into a single partition, creating a straggler worker. On cold runs, Range was still faster (1528 ms vs 2987 ms) by avoiding network shuffles, but warm execution was limited by partition skew.

---

### Category C: Measurement Margin / Scan Boundary Overhead vs Shuffle Savings

#### Query:

- `join_aggregate_sort`: Hash `1601.1 ms` vs Range `1625.4 ms` (1.02x ratio, essentially tied within ~1.5% noise)

#### Plan Differences:

- Hash mode required two `NetworkShuffleExec` stages.
- Range mode eliminated all network shuffles.
- In Range mode, `p` required scanning 5 to 11 partial segments per worker to evaluate `id` range boundaries, increasing scan compute from ~650ms to ~800ms. This extra scan overhead offset the network shuffle savings, resulting in a virtual tie on warm cache.
- On cold runs, Range was noticeably faster (1588 ms vs 1871 ms).

---

## 3. Captured Query Plans Location

Query plans and metric breakdowns for all 4 cases are saved in the repository under:

- `tmp_query_plans/join_top_k_score_desc_low_selectivity/`
  - `hash_plan.txt`
  - `range_plan.txt`
  - `plan_metrics.json`
- `tmp_query_plans/join_semi_filter/`
  - `hash_plan.txt`
  - `range_plan.txt`
  - `plan_metrics.json`
- `tmp_query_plans/join_aggregate_topk_count/`
  - `hash_plan.txt`
  - `range_plan.txt`
  - `plan_metrics.json`
- `tmp_query_plans/join_aggregate_sort/`
  - `hash_plan.txt`
  - `range_plan.txt`
  - `plan_metrics.json`

---

## 4. Architectural Takeaways for Range-by-Default

1. **Overall Effectiveness**: Range partitioning is superior in 11 out of 16 queries (and tied in 1), showing up to 5.5x speedups (`join_permissioned_search`, `join_foreign_filter_local_sort`) by eliminating inter-stage network serialization.
2. **Cross-Stage Dynamic Filtering vs Single-Stage Co-partitioning**: Eliminating network stages via range co-partitioning is not always a net win if the build side is small and the probe side is not physically clustered on the join key. In such cases, a multi-stage broadcast join can be faster because:
   - The probe table is scanned as whole physical segments without range boundary checking.
   - An eager build stage can push down a global dynamic filter into the probe scan, pruning hundreds of thousands of candidate rows before the join.
3. **Power-Law Split Sizing**: When deriving range split points, incorporating key frequency estimates rather than pure table row counts could prevent severe worker stragglers on skewed foreign keys.
