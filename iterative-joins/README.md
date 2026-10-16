# Iterative Top-K Joins Project

## 1. Project Overview

In `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`), queries with an `ORDER BY ... LIMIT K` currently execute in a single pass using `HashJoinExec` (and, when join-level search predicates are present, predicate tagging via `ScanMode::Tagged`) followed by `SortExec(TopK)` or `SegmentedTopKExec`. While a single pass in `DocId` order is optimal when at least one side of the join is small, it cannot use Tantivy's Block-Max WAND pruning on the build side or across multi-table scoring expressions, forcing Tantivy to score every matching document.

Meanwhile, `BaseScan` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) already uses an iterative windowing strategy (`search_top_k_in_segments`) that pulls a small sorted window from Tantivy, filters for PostgreSQL heap MVCC visibility, and only re-queries a deeper window if fewer than `K` rows survive.

This project unifies `BaseScan`'s iterative Top-K scan into DataFusion's `PgSearchScanPlan` and builds two lazy, order-preserving Top-K join operators on top of it, paired with plan-time heuristics that choose between one-pass joins and iterative Top-K joins.

---

## 2. Design Documents & Architecture

The design is split across four documents in `iterative-joins/`:

1. `iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` — _Shared Lazy Sorted Scan (`LazySortConfig` on `PgSearchScanPlan`)_:
   - Adds a lazy sorted stream mode to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) that yields progressively deeper Top-K windows `[offset .. offset + local_limit)` on each pull while preserving `PgSearchScanPlan`'s existing `ExecutionState::Shared` and `ExecutionState::RangePartitioned` modes.
   - Replaces `BaseScan`'s custom `TopKScanExecState` loop with `PgSearchScanPlan` in `LazySortMode`.
   - Provides the sorted input stream primitive required by both `RankJoinExec` and `IterativeLookupJoinExec`.

2. `iterative-joins/RANK_JOIN_DESIGN.md` — _Rank Join (`RankJoinExec`)_:
   - A composable binary join operator that consumes two lazy-sorted inputs (`PgSearchScanPlan` in `LazySortMode` or a child `RankJoinExec`) in score-guided order under a monotonic scoring function $f(s_L, s_R)$ (such as `pdb.score(a.id) + pdb.score(b.id) DESC`).
   - Maintains symmetric in-memory hash tables ($H_L, H_R$) and a candidate max-heap, advancing whichever input currently holds up the global upper-bound threshold $T = \max(U_L, U_R)$ and emitting results as soon as their score is $\ge T$.
   - Uses DataFusion's standard `PartitionMode::Partitioned` and `PartitionMode::CollectLeft` for MPP execution.

3. `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` — _Optimization & Plan-Time Heuristics (`RankJoinRule` & `IterativeLookupJoinRule`)_:
   - Defines when the optimizer chooses One-Pass `HashJoinExec`, `RankJoinExec`, or `IterativeLookupJoinExec`.
   - Specifies the initial v1 heuristics used when _only_ `RankJoinExec` and One-Pass `HashJoinExec` exist and join selectivity statistics are unavailable: checking `ScanMode::Standard`, verifying two-sided score balance via `Query::index_max_score`, and comparing the expected intersection depth $W_{\text{est}} = c \cdot \sqrt{K \cdot \max(D_L, D_R)}$ against $\min(M_L, M_R)$.
   - Specifies monotonicity verification for `SortExec` expressions and physical optimizer rule ordering in `pg_search/src/postgres/customscan/joinscan/scan_state.rs`.

4. `iterative-joins/ITERATIVE_TOPK_JOIN_DESIGN.md` — _Iterative Lookup Join (`IterativeLookupJoinExec`)_:
   - A follow-on join operator for queries where only _one_ side drives the `ORDER BY` (or where one side's score heavily dominates the other's, or when joining an unscored lookup table above a `RankJoinExec`).
   - Pulls a sorted window from its outer child, pushes the batch's join keys into a repeatable inner `PgSearchScanPlan` via `DynamicFilterPhysicalExpr` (`InListExpr` $\rightarrow$ `TermSetQuery`), and emits matches in outer sort order.

---

## 3. Project Implementation Roadmap

`RankJoinExec` is our first target join operator; it depends on `iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` and the v1 `RankJoinRule` portion of `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`. `IterativeLookupJoinExec` (`iterative-joins/ITERATIVE_TOPK_JOIN_DESIGN.md`) follows afterward.

### Milestone 1: Shared Lazy Sorted Scan & `BaseScan` Unification (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`)

- Add `LazySortConfig` to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`), supporting both `ExecutionState::Shared` (caching claimed segments across window re-queries) and `ExecutionState::RangePartitioned` (extending `SearchIndexReader::search_top_k_in_segments` in `pg_search/src/index/reader/index.rs` for range-filtered segments).
- Serialize `Option<LazySortConfig>` in `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`).
- Refactor `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) to drive `PgSearchScanPlan` in `LazySortMode` via `poll_next_sync`, validating the shared scan primitive against the existing `BaseScan` Top-K test suite.

### Milestone 2: `RankJoinExec`, v1 `RankJoinRule` & MPP Support (`iterative-joins/RANK_JOIN_DESIGN.md`, `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`)

- Implement `RankJoinExec`, `RankJoinStream`, and `RankedInputCursor` with symmetric hash tables ($H_L, H_R$), candidate max-heap, stopping threshold $T = \max(U_L, U_R)$, and $\arg\max_i U_i$ input scheduling for `PartitionMode::Partitioned` and `PartitionMode::CollectLeft`.
- Implement the monotonic scoring expression analyzer and `RankJoinRule` (using the v1 selectivity-free heuristics from `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` §3 and §4) behind `paradedb.enable_rank_join`, and register it in `build_base_session` (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`).
- Teach `DeferredPlacementRule` (`pg_search/src/scan/deferred_placement_rule.rs`) about `RankJoinExec` so deferred string decoding remains above the join and visibility filter.
- Add `TAG_RANK_JOIN` serialization in `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`).
- Add unit and `pg_regress` tests covering 2-way and 3-way rank joins, MVCC dead-tuple re-querying, deferred string materialization, and MPP execution (`RangePartitioned`, `CollectLeft`, and `NetworkShuffleExec`).

### Milestone 3: Tantivy `max_score` Integration (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` §4, `iterative-joins/RANK_JOIN_DESIGN.md` §3.3)

- Integrate Tantivy's `Weight::index_max_score` and `Weight::max_score` into `LazySortMode` (for $O(1)$ segment skipping in `search_top_k_in_segments`) and `RankJoinRule` (for plan-time score balance checks).
- Wire verified post-filter top-$K$ score thresholds back into `LazySortMode` / `PruningScorer` for cross-join Block-Max WAND pruning.

### Milestone 4: `IterativeLookupJoinExec` & Hybrid Composition (`iterative-joins/ITERATIVE_TOPK_JOIN_DESIGN.md`, `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` §2)

- Add repeatable execution support to `PgSearchScanPlan` and retain `PreFilter` fallback when `InListPushdown::Skip` triggers on small segments (`pg_search/src/scan/pre_filter.rs`).
- Implement `IterativeLookupJoinExec` (`IterativeJoinPartitionMode::{Partitioned, ReplicatedInner}`), `IterativeLookupJoinRule` behind `paradedb.enable_iterative_lookup_join`, `DeferredPlacementRule` integration, and `TAG_ITERATIVE_LOOKUP_JOIN` MPP serialization.
- Test standalone `IterativeLookupJoinExec` queries (single-table and heavily lopsided score queries) as well as hybrid multi-table plans where `RankJoinExec` feeds into the outer side of `IterativeLookupJoinExec`.
