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
   - Supports `ScanMode::Tagged` by running `SA` over the positive-score tag query while evaluating `active_tag_scorers` on `RA` point-lookup hits (treating non-matching rows as $0.0$-scored documents).
   - Replaces `BaseScan`'s custom `TopKScanExecState` loop with `PgSearchScanPlan` in `LazySortMode`.

2. `iterative-joins/RANK_JOIN_DESIGN.md` — _Unified `SA + RA` Rank Join (`RankJoinExec`)_:
   - Grounded in `Fagin et al. (PODS 2001 / JCSS 2003, NRA/TA/CA)`, `Chang & Hwang (SIGMOD 2002, MPro)`, `Bruno, Gravano, & Marian (ICDE 2002 / TODS 2004, Upper)`, `Ilyas, Aref, & Elmagarmid (VLDB 2003 / SIGMOD 2004, HRJN*/NRJN)`, and `Schnaitter & Polyzotis (PODS 2008 / TODS 2010, PBRJ)`.
   - Combines `SA` window pulls with lazy `CA`/`MPro`-scheduled `RA` point lookups: unresolved top tuples on one side whose optimistic bound $f(s_L(k), s_R^{\text{bottom}})$ blocks emission trigger a batched `RankLookupPhysicalExpr` point lookup on the opposite side, collapsing the stopping threshold from the `HRJN` corner bound to $T = f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
   - Executes single-sided ordered joins (`NRJN`, `ORDER BY pdb.score(a.id) DESC` or `ORDER BY a.col ASC`) directly as the special case where the unscored child contributes $0$ to the score (`SA` on the ordered child + `RA`-only with `initial_window = 0.0` on the unscored child).
   - Supports cascading `RA` point lookups through nested `RankJoinExec` nodes in $N \ge 3$ multi-way joins.

3. `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` — _Optimization & Plan-Time Heuristics (`RankJoinRule`)_:
   - Defines when `RankJoinRule` rewrites `HashJoinExec` into `RankJoinExec` across conjunctive (`ScanMode::Standard`), disjunctive (`ScanMode::Tagged`), and single-sided (`NRJN`) top-$K$ joins.

---

## 3. Project Implementation Roadmap

### Milestone 1: Shared Lazy Sorted Scan & `BaseScan` Unification (Completed)

- Added `LazySortConfig` to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) for `ExecutionState::Shared` and `ExecutionState::RangePartitioned`.
- Unified `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) and `queries` metric onto `PgSearchScanPlan`.

### Milestone 2: Core `RankJoinExec` (`SA`), `RankJoinRule` & `DeferredPlacementRule` (Completed)

- Implemented `RankJoinExec`, `RankJoinStream`, `RankedInputCursor`, `RankJoinRule`, `DeferredPlacementRule` integration, and `TAG_RANK_JOIN` protobuf serialization.

### Milestone 2.5: Unified `SA + RA` Point Lookups, Disjunctive (`ScanMode::Tagged`) & Single-Sided (`NRJN`) Joins (Serial Completed; MPP Pending)

- Implemented `RankLookupPhysicalExpr` (wrapped in `DynamicFilterPhysicalExpr`) and `SA + RA` execution with `emitted_docs` deduplication in `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`), including `ScanMode::Tagged` positive-score `SA` + $0$-score `RA`.
- Extended `RankJoinExec` and `RankJoinStream` (`pg_search/src/scan/rank_join_exec.rs`) with unresolved-key tracking, lazy `CA`/`MPro` point-lookup scheduling, cascading `RA` forwarding across nested $N$-way `RankJoinExec` trees, and single-sided (`SA` + `RA`-only) execution.
- Updated `RankJoinRule` (`pg_search/src/scan/rank_join_rule.rs`) to support `ScanMode::Tagged` disjunctive joins and single-sided (`NRJN`) score joins.

### Milestone 3: MPP Integration, Deferred Tie-Breaker Columns & Tantivy `max_score` Pushdown

- Complete MPP co-partitioned execution support, deferred tie-breaker column handling in `RankJoinRule`, single-sided fast-field column ordering, and cross-join score threshold pushdown.

---

## 4. Current Status

### Done

1. **Unified Design & Literature Alignment (`iterative-joins/*.md`)**:
   - Grounded the architecture in `Fagin et al. (PODS 2001 / JCSS 2003, NRA/TA/CA)`, `Chang & Hwang (SIGMOD 2002, MPro)`, `Bruno, Gravano, & Marian (ICDE 2002 / TODS 2004, Upper)`, `Ilyas, Aref, & Elmagarmid (VLDB 2003 / SIGMOD 2004, HRJN*/NRJN)`, and `Schnaitter & Polyzotis (PODS 2008 / TODS 2010, PBRJ)`.
   - Unified two-sided conjunctive, disjunctive (`ScanMode::Tagged`), and single-sided (`NRJN`) top-$K$ joins into `RankJoinExec` (`SA` on scored children + `RA`-only with `initial_window = 0.0` on unscored children).
2. **`PgSearchScanPlan` (`SA + RA` Scan Execution)**:
   - Added `RankLookupPhysicalExpr` (`pg_search/src/scan/pre_filter.rs`) and conversion to `ConstScoreQuery(TermSetQuery, 0.0)` (`try_convert_rank_lookup_to_query`).
   - Added `SearchIndexReader::with_tagged_sa_queries`, `prepare_ra_lookup_weights`, and `search_ra_lookup` (`pg_search/src/index/reader/index.rs`) to intersect `TermSetQuery` postings with alive/unemitted documents and evaluate exact BM25 / `ActiveTagScorer` scores (`pg_search/src/scan/batch_scanner.rs`).
   - Added `emitted_docs` (`HashSet<DocAddress>`) deduplication across `SA` and `RA` pulls and `SA` exhaustion signaling (`ScalarValue::Boolean(Some(false))` when kept alive for `RA`, `ScalarValue::Boolean(None)` when fully exhausted) in `PgSearchScanPlan::execute` (`pg_search/src/scan/execution_plan.rs`).
3. **`RankJoinExec` & `RankJoinRule` (Serial End-to-End)**:
   - Implemented `SA + RA` execution in `RankJoinStream` and `RankedInputCursor` (`pg_search/src/scan/rank_join_exec.rs`) with dynamically scaled `RA` batching (`RA_INITIAL_BATCH_SIZE = 64`, growing by `topk_retry_scale_factor` up to `max_topk_chunk_size`) and interleaved `RA` lookups before draining large `SA` windows, resolving the "orphan top-1" stall and tightening the stopping threshold to $f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
   - Implemented cascading `RA` point lookups (`parent_lookup_filters`) across nested $N$-way `RankJoinExec` trees without force-draining paused `SA` buffers.
   - Implemented `ScanMode::Tagged` cross-table `OR` rank joins and single-sided (`NRJN`) score-ordered joins in `RankJoinRule` (`pg_search/src/scan/rank_join_rule.rs`).
   - Added serial (`max_parallel_workers_per_gather = 0`) end-to-end regression tests in `pg_search/tests/pg_regress/sql/joinscan_sortby_score.sql` (`TEST 8`, `8b`, `8c`, `8d`) covering 2-way conjunctive, 3-way conjunctive, 2-way disjunctive (`OR`), 3-way disjunctive (`OR`), and 2-way single-sided (`NRJN`) rank joins.
   - Gated `RankJoinRule` off when MPP is active (`!mpp_is_active()`, the inverse of `RangePartitioningRule` / `RangeCoPartitionedJoinRule`), since cross-process dynamic filters are currently disabled in `datafusion-distributed`.

### TODO

1. **Eliminating `ScanMode::Tagged` Under `RankJoinExec`**:
   - Under `RankJoinExec`, every candidate is seeded by at least one `SA` stream (`score > 0`), so an all-unmatched tuple can never be formed and both the `__tag_i` boolean columns and the disjunctive `JoinFilter` (`tag_0 OR tag_1 OR ...`) are tautologically redundant.
   - Currently, `ScanMode::Tagged` populates `Scanner::tagged_queries` (`pg_search/src/scan/batch_scanner.rs`), causing `materialize_topk_window` (`ensure_segment_tag_scorers`) to construct and seek a fresh BM25 `TermScorer` (`tq.weight.scorer(segment_reader, 1.0)`) on every segment transition across all `SA` and `RA` batches (re-evaluating BM25 on `SA` hits after `TopK` already scored them, inflating shared-buffer reads, and disabling `can_pushdown_score_threshold()`).
   - Run `RankJoinExec` leaves without `tagged_queries` (using standard `SA` over the leaf predicate + $0.0$-score fallback for non-matching `RA` hits in `search_ra_lookup` in `pg_search/src/index/reader/index.rs`) and strip the redundant all-tag disjunction `JoinFilter`.
2. **Downward Score-Floor Pushdown Across Nested `RankJoinExec`s & to Tantivy `TopDocs`**:
   - Once a parent `RankJoinExec` has $K$ candidate matches in `candidate_heap` with threshold $T$ (or dispatches a cascading `RA` lookup for rows with max batch score $s_{\max}$), invert `ScoreCombiner` (using the opposite side's `top_score` or $s_{\max}$) to push a dynamic minimum required score floor $T_{\min}$ down into child `RankJoinExec` nodes (`RankLookupPhysicalExpr` / `poll_parent_lookup`) and leaf `PgSearchScanPlan` `SA`/`RA` pulls.
   - This prunes low-scoring single-match `(user, post)` `RA` fanout pairs (where one side has `score = 0.0`) inside child joins before materialization.
3. **Per-Segment Join-Key Min/Max Pruning in `search_ra_lookup`**:
   - Prune segments in `SearchIndexReader::search_ra_lookup` (`pg_search/src/index/reader/index.rs`) against `SegmentStatsSnapshot` min/max fast-field bounds on the join key before constructing per-segment `TermSetScorer`s.
4. **MPP (`datafusion-distributed`) Support & Verification**:
   - Once cross-process dynamic filters (or single-stage co-partitioned dynamic filters) are supported under MPP, re-enable `RankJoinRule` under `mpp_is_active()` for range-co-partitioned (`PartitionMode::Partitioned`) joins.
   - Update `plan_contains_filter_id` in `pg_search/src/scan/rank_join_exec.rs` to descend into `datafusion_distributed::DistributedLeafExec` (`original()` and `variants()`), whose `.children()` is empty.
   - Add MPP regression tests for range-co-partitioned `RankJoinExec`.
5. **Deferred String Columns in Secondary `ORDER BY` Tie-Breakers**:
   - When a secondary tie-breaker references a non-join-key `TEXT` column (e.g., `ORDER BY score DESC, files.id ASC`), `LateMaterializationRule` places `TantivyDecodeExec` above `HashJoinExec`, changing the column's `DataType` from `UInt64` to `Utf8View` and causing `RankJoinRule` to decline the rewrite in favor of `SegmentedTopKExec`. Teach `RankJoinRule` / `DeferredPlacementRule` to eagerly materialize secondary tie-breaker columns below `RankJoinExec`.
6. **Single-Sided Non-Score Column Ordering (`OrderByFeature::Field`)**:
   - Extend `RankJoinRule` to rewrite single-sided non-score column orderings (`ORDER BY a.col ASC/DESC LIMIT K`) into `RankJoinExec` (`SA` by fast field on `a`, `RA`-only on `b`).
