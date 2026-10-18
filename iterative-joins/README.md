# Iterative Top-K Joins Project

## 1. Project Overview

In `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`), queries with an `ORDER BY ... LIMIT K` historically executed in a single pass using `HashJoinExec` (and, when join-level or disjunctive search predicates were present, predicate tagging via `ScanMode::Tagged`) followed by `SortExec(TopK)` or `SegmentedTopKExec`. While a single pass in `DocId` order is optimal when at least one side of a conjunctive join is small, it cannot use Tantivy's Block-Max WAND pruning across multi-table scoring expressions, and under `ScanMode::Tagged` (`base_query = All`) it must scan every row of every table in the database.

Meanwhile, `BaseScan` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) uses an iterative windowing strategy (`search_top_k_in_segments`) that pulls a small sorted window from Tantivy, filters for PostgreSQL heap MVCC visibility, and only re-queries a deeper window if fewer than `K` rows survive.

This project unifies `BaseScan`'s iterative Top-$K$ scan (`Sorted Access` / `SA`) and on-demand batched point lookups (`Random Access` / `RA` via `RankLookupPhysicalExpr` $\rightarrow$ `TermSetQuery`) into `PgSearchScanPlan`, and builds a single composable Top-$K$ join operator — `RankJoinExec` — that handles conjunctive multi-table joins, disjunctive (`OR`) multi-table joins, lopsided score joins, and single-sided lookup joins (`NRJN`).

---

## 2. Design Documents & Architecture

The design is documented across three files in `iterative-joins/`:

1. `iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` — _Shared Lazy Sorted (`SA`) & Point-Lookup (`RA`) Scan (`PgSearchScanPlan`)_:
   - Adds `LazySortConfig` (`SA` expanding Top-$K$ windows) and `RankLookupPhysicalExpr` (`RA` batched point lookups via `DynamicFilterPhysicalExpr`) to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`), with `emitted_docs` deduplication so no document is emitted twice across `SA` and `RA` pulls.
   - Supports `ScanMode::Disjunctive` by running `SA` over the positive-score disjunctive query (`with_disjunctive_sa_queries`) while scoring `RA` point-lookup hits directly against pre-built `ra_weights` (treating non-matching rows as $0.0$-scored documents without `Scanner::tagged_queries` or boolean `__tag_i` columns).
   - Replaces `BaseScan`'s custom `TopKScanExecState` loop with `PgSearchScanPlan` in `LazySortMode`.

2. `iterative-joins/RANK_JOIN_DESIGN.md` — _Unified `SA + RA` Rank Join (`RankJoinExec`)_:
   - Grounded in `Fagin et al. (PODS 2001 / JCSS 2003, NRA/TA/CA)`, `Chang & Hwang (SIGMOD 2002, MPro)`, `Bruno, Gravano, & Marian (ICDE 2002 / TODS 2004, Upper)`, `Ilyas, Aref, & Elmagarmid (VLDB 2003 / SIGMOD 2004, HRJN*/NRJN)`, and `Schnaitter & Polyzotis (PODS 2008 / TODS 2010, PBRJ)`.
   - Combines `SA` window pulls with lazy `CA`/`MPro`-scheduled `RA` point lookups: unresolved top tuples on one side whose optimistic bound $f(s_L(k), s_R^{\text{bottom}})$ blocks emission trigger a batched `RankLookupPhysicalExpr` point lookup on the opposite side, collapsing the stopping threshold from the `HRJN` corner bound to $T = f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
   - Executes single-sided ordered joins (`NRJN`, `ORDER BY pdb.score(a.id) DESC` or `ORDER BY a.col ASC`) directly as the special case where the unscored child contributes $0$ to the score (`SA` on the ordered child + `RA`-only with `initial_window = 0.0` on the unscored child).
   - Supports cascading `RA` point lookups through nested `RankJoinExec` nodes in $N \ge 3$ multi-way joins.

3. `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` — _Optimization & Plan-Time Heuristics (`RankJoinRule`)_:
   - Defines when `JoinSearchPredicateRule` and `RankJoinRule` rewrite `HashJoinExec` into `RankJoinExec` across conjunctive (`ScanMode::Standard`), disjunctive (`ScanMode::Disjunctive`), and single-sided (`NRJN`) top-$K$ joins using shared `SubtreeScoreStats` cost-model gates.

---

## 3. Project Implementation Roadmap

### Milestone 1: Shared Lazy Sorted Scan & `BaseScan` Unification (Completed)

- Added `LazySortConfig` to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) for `ExecutionState::Shared` and `ExecutionState::RangePartitioned`.
- Unified `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) and `queries` metric onto `PgSearchScanPlan`.

### Milestone 2: Core `RankJoinExec` (`SA`), `RankJoinRule` & `DeferredPlacementRule` (Completed)

- Implemented `RankJoinExec`, `RankJoinStream`, `RankedInputCursor`, `RankJoinRule`, `DeferredPlacementRule` integration, and `TAG_RANK_JOIN` protobuf serialization.

### Milestone 2.5: Unified `SA + RA` Point Lookups, Disjunctive (`ScanMode::Disjunctive`) & Single-Sided (`NRJN`) Joins (Serial Completed; MPP Pending)

- Implemented `RankLookupPhysicalExpr` (wrapped in `DynamicFilterPhysicalExpr`) and `SA + RA` execution with `emitted_docs` deduplication in `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`), including `ScanMode::Disjunctive` positive-score `SA` + $0$-score `RA` without predicate tagging.
- Extended `RankJoinExec` and `RankJoinStream` (`pg_search/src/scan/rank_join_exec.rs`) with unresolved-key tracking, lazy `CA`/`MPro` point-lookup scheduling, cascading `RA` forwarding across nested $N$-way `RankJoinExec` trees, and single-sided (`SA` + `RA`-only) execution.
- Updated `JoinSearchPredicateRule` (`pg_search/src/postgres/customscan/joinscan/search_predicate_rule.rs`) and `RankJoinRule` (`pg_search/src/scan/rank_join_rule.rs`) to elide predicate tagging and plan `ScanMode::Disjunctive` for eligible disjunctive top-$K$ joins and single-sided (`NRJN`) score joins.

### Milestone 3: MPP Integration, Deferred Tie-Breaker Columns & Tantivy `max_score` Pushdown

- Complete MPP co-partitioned execution support, deferred tie-breaker column handling in `RankJoinRule`, single-sided fast-field column ordering, and cross-join score threshold pushdown.

---

## 4. Current Status

### Done

1. **Unified Design & Literature Alignment (`iterative-joins/*.md`)**:
   - Grounded the architecture in `Fagin et al. (PODS 2001 / JCSS 2003, NRA/TA/CA)`, `Chang & Hwang (SIGMOD 2002, MPro)`, `Bruno, Gravano, & Marian (ICDE 2002 / TODS 2004, Upper)`, `Ilyas, Aref, & Elmagarmid (VLDB 2003 / SIGMOD 2004, HRJN*/NRJN)`, and `Schnaitter & Polyzotis (PODS 2008 / TODS 2010, PBRJ)`.
   - Unified two-sided conjunctive, disjunctive (`ScanMode::Disjunctive`), and single-sided (`NRJN`) top-$K$ joins into `RankJoinExec` (`SA` on scored children + `RA`-only with `initial_window = 0.0` on unscored children).
2. **`PgSearchScanPlan` (`SA + RA` Scan Execution & `ScanMode::Disjunctive`)**:
   - Added `RankLookupPhysicalExpr` (`pg_search/src/scan/pre_filter.rs`) and conversion to `ConstScoreQuery(TermSetQuery, 0.0)` (`try_convert_rank_lookup_to_query`).
   - Added `ScanMode::Disjunctive` (`pg_search/src/scan/info.rs`) and `SearchIndexReader::with_disjunctive_sa_queries`, `prepare_ra_lookup_weights`, and `search_ra_lookup` (`pg_search/src/index/reader/index.rs`) to run `SA` over positive-score disjuncts and intersect `TermSetQuery` postings with alive/unemitted documents while evaluating exact BM25 scores directly from pre-built `ra_weights` (eliminating `ScanMode::Tagged`, `__tag_i` boolean columns, and `Scanner::tagged_queries` under `RankJoinExec`).
   - Added `emitted_docs` (`HashSet<DocAddress>`) deduplication across `SA` and `RA` pulls and `SA` exhaustion signaling (`ScalarValue::Boolean(Some(false))` when kept alive for `RA`, `ScalarValue::Boolean(None)` when fully exhausted) in `PgSearchScanPlan::execute` (`pg_search/src/scan/execution_plan.rs`).
3. **`RankJoinExec`, `JoinSearchPredicateRule` & `RankJoinRule` (Serial End-to-End)**:
   - Implemented `SA + RA` execution in `RankJoinStream` and `RankedInputCursor` (`pg_search/src/scan/rank_join_exec.rs`) with dynamically scaled `RA` batching (`RA_INITIAL_BATCH_SIZE = 64`, growing by `topk_retry_scale_factor` up to `max_topk_chunk_size`) and interleaved `RA` lookups before draining large `SA` windows, resolving the "orphan top-1" stall and tightening the stopping threshold to $f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
   - Implemented cascading `RA` point lookups (`parent_lookup_filters`) across nested $N$-way `RankJoinExec` trees without force-draining paused `SA` buffers.
   - Unified `SubtreeScoreStats` Gates 1–3 across `JoinSearchPredicateRule` (`pg_search/src/postgres/customscan/joinscan/search_predicate_rule.rs`) and `RankJoinRule` (`pg_search/src/scan/rank_join_rule.rs`) so eligible disjunctive top-$K$ joins elide `__tag_i` columns and redundant `JoinFilter`s at logical plan time and rewrite in a single pass into `RankJoinExec` over `ScanMode::Disjunctive` leaves.
   - Added serial (`max_parallel_workers_per_gather = 0`) end-to-end regression tests in `pg_search/tests/pg_regress/sql/joinscan_sortby_score.sql` (`TEST 8`, `8b`, `8c`, `8d`) covering 2-way conjunctive, 3-way conjunctive, 2-way disjunctive (`OR`), 3-way disjunctive (`OR`), and 2-way single-sided (`NRJN`) rank joins.
   - Gated `RankJoinRule` off when MPP is active (`!mpp_is_active()`, the inverse of `RangePartitioningRule` / `RangeCoPartitionedJoinRule`), since cross-process dynamic filters are currently disabled in `datafusion-distributed`.
4. **Local Top-$K$ Heap Bound (`fetch = K`), Two-Tier `RA` Scheduling, & Score-Floor Pushdown into Tantivy `PruningScorer` (`RankJoinExec`, `PgSearchScanPlan`, `SearchIndexReader`)**:
   - Propagated `fetch: Option<usize>` onto the root `RankJoinExec` (`pg_search/src/scan/rank_join_rule.rs`) so `RankJoinStream` terminates once `K` rows are emitted and uses the $K$-th candidate score $T_K$ to bound `RA` lookups.
   - Partitioned unresolved `SA` keys in `RankedInputCursor` (`pg_search/src/scan/rank_join_exec.rs`) into two tiers:
     - **Tier 1 (`PositiveScoreMatch`, `score_threshold = Some(0.0)`)**: probes the opposite side for strictly positive-score matches ($s_{\text{other}} > 0$), tightening an unresolved key's upper bound from $g(s(k), \underline{s}_{\text{other}})$ down to $g(s(k), 0.0)$ without materializing $0.0$-score documents.
     - **Tier 2 (`AllMatches`, `score_threshold = None`)**: only issued when $g(s^{\text{pos}}(k), 0.0)$ still clears the remaining corner bounds and top candidate (`cutoff`), popping only keys whose Tier-2 bound $g(s^{\text{pos}}(k), 0.0) \ge \text{cutoff}$.
   - Added `score_threshold: Option<f32>` to `RankLookupPhysicalExpr` (`pg_search/src/scan/pre_filter.rs`), `ScoreCombiner::left_threshold_for` / `right_threshold_for`, and `SearchIndexReader::search_ra_lookup` (`pg_search/src/index/reader/index.rs`), pushing Tantivy's `PruningScorer` (`ScoreThreshold::gt(0.0)` or `ScoreThreshold::ge(min_score)`) into `tag_weights` / `base_weight` before fast-field materialization and advancing `lookup_scorer` directly on `BitsetFromPostings` (`has_fast_bitset()`).
   - Balanced `SA` cursor advancement by `sa_rows_advanced` when both sides are `ra_capable` (2-competitive with the omniscient offline oracle for $U_{\text{SA}}$), fixed `next_buffered_sa_side` starvation when `top_clears_sa` is true, and raised `bitset_max_density_unique` floor in `try_convert_rank_lookup_to_query` to match `bitset_max_density_multi` (`0.005`), cutting `join_disjunctive_score_sort` (`hash_partitioned.sql`) execution time in half (`11.8s -> 5.9s` debug build, `97.8K` shared buffers vs. `~300K+`, `comments` rows `20.06K -> 1.59K`, `stackoverflow_posts` rows `27.51K -> 9.23K`).

### TODO

1. **CPU / Buffer Profiling & `EXPLAIN ANALYZE` Telemetry for `RankJoinExec` and `PgSearchScanPlan`**:
   - Record `dynamic_filter_pushdown_<strategy>` and `dynamic_filter_pushdown_score` metrics inside the `pending_lookup` (`RA`) branch of `PgSearchScanPlan::execute` (`pg_search/src/scan/execution_plan.rs`), which currently hits `continue;` before updating those counters.
   - Add granular metrics / profiling to break down where `stackoverflow_posts` (`3.85s` across `20` queries, `9.23K` rows) and `users` (`1.35s` across `24` queries, `5.72K` rows) spend time: `SA` (`search_sa_page`) vs. Tier-1 `RA` (`score_threshold = Some(0.0)`) vs. Tier-2 / cascading `RA` (`score_threshold = None`), `try_convert_rank_lookup_to_query` bitset construction, Tantivy scoring, and fast-field / heap visibility checks.
2. **Coalesce Under-Full Tier-1 `RA` Batches Across Small Child Batches (`RankJoinExec`)**:
   - In `RankJoinStream::next_io_step` (`pg_search/src/scan/rank_join_exec.rs`), because $s_L^{\text{pos}} \ge \underline{s}_L$, `eff_ra_l = g(s_l_pos, bot_r) >= g(bot_l, bot_r) = eff_sa` is always true whenever `unresolved_key_count() > 0`. When a child `RankJoinExec` emits small batches (`Inner` emits `1.68K` rows across `13` batches, ~129 rows/batch) or pauses mid-batch to emit ready rows, `has_buffered_rows()` becomes `false` with fewer than `ra_batch_size` unresolved keys, triggering under-full Tier-1 `RA` queries (`9` `RA` queries on `comments` for `1.68K` rows instead of `3–4`, and `12` Tier-1 `RA` queries on `users` instead of `8`). Prefer pulling another `SA` batch to fill `ra_batch_size` (or finishing buffered `SA` rows before emitting intermediate non-root batches) unless `top_candidate >= eff_sa` is ready to terminate.
3. **Two-Tier Cascading `RA` / Positive-Score Pruning in `execute_parent_lookup` ($N \ge 3$ Joins)**:
   - When an outer `RankJoinExec` issues a Tier-1 (`score_threshold = Some(0.0)`) lookup into a child `RankJoinExec` (`Inner`), `execute_parent_lookup` (`pg_search/src/scan/rank_join_exec.rs`) computes `target_thresh = combiner.right_threshold_for(0.0, left_upper)`, which is `None` whenever `left_upper > 0`. This fetches target rows (`stackoverflow_posts`) with `score_threshold = None` (`~1.59K` rows), most of which have `p_score == 0.0` AND `u_score == 0.0` on their `owner_user_id` and get discarded by `parent_thresh = Some(0.0)`. Avoid materializing $0.0$-score target rows whose opposite join partner also has $0.0$ score, and push a tighter `score_threshold` when the probing outer batch's max score is bounded by the outer $K$-th candidate $T_K$.
4. **Cross-Join Downward `SA` Score-Floor Pushdown ($T_{\min}$ into Child `SA` Cursors)**:
   - Once the root `RankJoinExec` has $K$ candidates with $K$-th score $T_K$ and opposite-side upper bound $\bar{s}_{\text{other}}$, push $T_{\min} = \text{combiner.threshold\_for}(T_K, \bar{s}_{\text{other}})$ downward to child `RankJoinExec`s and leaf `SA` cursors (`LazySortConfig`) so leaf `SA` stops pulling windows once its `bottom_score < T_min`.
5. **Avoid Re-Running `TopDocs(limit + offset)` from Scratch on Successive `SA` Pulls**:
   - Pass a `(score, DocAddress)` cursor (`search_after`) or retain per-segment state across `SA` window pulls in `PgSearchScanPlan` so pulling window $p$ does not re-evaluate windows $1 \dots p-1$.
6. **Per-Segment Join-Key Min/Max Pruning in `search_ra_lookup`**:
   - Prune segments in `SearchIndexReader::search_ra_lookup` (`pg_search/src/index/reader/index.rs`) against `SegmentStatsSnapshot` min/max fast-field bounds on the join key before constructing per-segment `TermSetScorer`s.
7. **Deferred String Columns in Secondary `ORDER BY` Tie-Breakers**:
   - When a secondary tie-breaker references a non-join-key `TEXT` column (e.g., `ORDER BY score DESC, files.id ASC`), `LateMaterializationRule` places `TantivyDecodeExec` above `HashJoinExec`, changing the column's `DataType` from `UInt64` to `Utf8View` and causing `RankJoinRule` to decline the rewrite in favor of `SegmentedTopKExec`. Teach `RankJoinRule` / `DeferredPlacementRule` to eagerly materialize secondary tie-breaker columns below `RankJoinExec`.
8. **Single-Sided Non-Score Column Ordering (`OrderByFeature::Field`)**:
   - Extend `RankJoinRule` to rewrite single-sided non-score column orderings (`ORDER BY a.col ASC/DESC LIMIT K`) into `RankJoinExec` (`SA` by fast field on `a`, `RA`-only on `b`).
9. **MPP (`datafusion-distributed`) Support & Verification**:
   - Once cross-process dynamic filters (or single-stage co-partitioned dynamic filters) are supported under MPP, re-enable `RankJoinRule` under `mpp_is_active()` for range-co-partitioned (`PartitionMode::Partitioned`) joins.
   - Update `plan_contains_filter_id` in `pg_search/src/scan/rank_join_exec.rs` to descend into `datafusion_distributed::DistributedLeafExec` (`original()` and `variants()`), whose `.children()` is empty.
   - Add MPP regression tests for range-co-partitioned `RankJoinExec`.
