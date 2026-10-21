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

## 3. Current Status

### Done

- **Unified `SA + RA` Scan (`PgSearchScanPlan` & `Scanner`)**:
  - Unified `BaseScan`'s `TopKScanExecState` onto `PgSearchScanPlan` via `LazySortConfig` (`SA` expanding Top-$K$ windows).
  - Added `RankLookupPhysicalExpr` (`RA` batched point lookups via `DynamicFilterPhysicalExpr` $\rightarrow$ `TermSetQuery`) with `emitted_docs` deduplication across `SA` and `RA` pulls and `PruningScorer` score-threshold pushdown (`search_ra_lookup`).
  - Added tagless `ScanMode::Disjunctive` (for two-sided score-sorted disjunctive joins) and `ScanMode::Tagged` support in `LazySortConfig` (for single-sided fast-field-sorted disjunctive joins).
- **`RankJoinExec` (`pg_search/src/scan/rank_join_exec.rs`)**:
  - Implemented pull-based symmetric hash rank join combining `SA` and two-tier `CA`/`MPro` `RA` point lookups (Tier-1 `score > 0.0` probes and Tier-2 `score = 0.0` probes), collapsing the stopping threshold to $T = f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
  - Implemented cascading `RA` point lookups (`parent_lookup_filters`) across $N$-way nested `RankJoinExec` trees.
  - Supported two-sided score joins, single-sided score joins (`NRJN`), and single-sided non-score fast-field joins (`OrderByFeature::Field`, over both `ScanMode::Standard` and `ScanMode::Tagged`).
- **Planner Rules & Heuristics (`JoinSearchPredicateRule` & `RankJoinRule`)**:
  - Elided predicate tagging into `ScanMode::Disjunctive` for eligible two-sided disjunctive score joins.
  - Gated `RankJoinRule` on $K \le 1,000$, score contribution balance ($\ge 0.20$), minimum row count (`paradedb.rank_join_min_rows`), unscored `RA` selectivity (`UNSCORED_SELECTIVITY_THRESHOLD = 0.08`), and disjunctive eligibility for two-sided joins.
  - Configured `RankJoinExec` and `lazy_sort` leaf scans to request a single `datafusion-distributed` task (`Maximum(1)`) so in-memory dynamic filters work when MPP is enabled.

### TODO

1. **MPP (`datafusion-distributed`) Support**:
   - Currently `RankJoinExec` and `lazy_sort` leaf scans force single-task execution (`Maximum(1)`) because `RankLookupPhysicalExpr` dynamic filters communicate via in-process shared memory (`Arc<RwLock<...>>`).
   - Support parallel MPP execution — e.g., across range-co-partitioned (`PartitionMode::Partitioned`) `RankJoinExec` tasks where each worker runs an independent `RankJoinExec` (`SA + RA`) within its co-partitioned join-key range below `SortPreservingMergeExec(fetch=K)`, or via cross-worker dynamic filters.
2. **True Lazy Iterator / Cursor- & Score-Bounded `LazySortConfig`**:
   - Currently each expanding `SA` window pull in `PgSearchScanPlan` re-executes `search_top_k_in_segments(limit, offset)` from the start of each segment (`limit + offset`) and discards already-emitted `DocAddress`es via `emitted_docs`.
   - Pass a `(score_or_value, DocAddress)` cursor (`search_after`) and/or a downward score floor $T_{\min}$ from `RankJoinExec` into `LazySortConfig` (or retain a stateful per-segment Top-$K$ iterator) so successive `SA` pulls do not re-evaluate earlier windows and can prune segments as soon as their upper bound drops below $T_{\min}$.
3. **Deferred String Columns (`https://github.com/paradedb/paradedb/issues/6839`)**:
   - Currently `LazySortConfig` and `RankJoinRule` require primary sort columns to be non-deferred static fast fields (`PgSearchScanPlan::is_sortable_fast_field`) and convert any deferred secondary tie-breaker columns back to eager delivery (`make_fields_eager`).
   - Support sorting on deferred string columns (see `https://github.com/paradedb/paradedb/issues/6839`) so primary and secondary string sort keys can use `LazySortConfig` and `RankJoinExec` without eager dictionary materialization.
