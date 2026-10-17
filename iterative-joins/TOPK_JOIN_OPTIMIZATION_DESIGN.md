# Top-K Join Optimization & Plan-Time Heuristics Design (`RankJoinRule`)

## 1. Overview

When a join query has an `ORDER BY ... LIMIT K`, `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`) chooses between two execution strategies:

1. _One-Pass `HashJoinExec`_ (paired with `SegmentedTopKExec` or `SortExec(TopK)`).
2. _`RankJoinExec`_ (`iterative-joins/RANK_JOIN_DESIGN.md`), which combines _Sorted Access (`SA` — iterative Top-$K$ window pulls)_ and _Random Access (`RA` — batched join-key point lookups via `RankLookupPhysicalExpr` $\rightarrow$ `TermSetQuery`)_ across both two-sided (`SA + RA`) and single-sided (`SA` + `RA`-only) joins.

The core trade-off between these two strategies is _full-pass hash building_ versus _bounded Top-$K$ windowing + lazy point lookups_:

- `HashJoinExec` scans each table in a single pass in `DocId` order. When at least one side of a conjunctive join matches only a small number of documents, `HashJoinExec` builds a small hash table on that side and pushes a `TermSetQuery` into the probe side. However, when both inputs match many documents — or in a disjunctive cross-table `OR` query (`ScanMode::Tagged`), where `HashJoinExec` must scan _every row of every table_ (`base_query = All`) — `HashJoinExec` must score and buffer millions of rows.
- `RankJoinExec` uses `LazySortMode` (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`) for `SA` window pulls (leveraging Tantivy's Block-Max WAND collector) and `RankLookupPhysicalExpr` (`TermSetQuery`) for lazy `RA` point lookups (`CA` / `MPro`, `Fagin et al. PODS 2001`, `Chang & Hwang SIGMOD 2002`). By resolving unmatched top keys via `RA` point lookups, `RankJoinExec` lowers its stopping threshold to $f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$ without scanning the tail of either input and without scanning $0$-scored rows in disjunctive joins.

---

## 2. How `RankJoinExec` Adapts Across Query Shapes

Because `RankJoinExec` supports both `SA` and `RA` on each input, `RankJoinRule` configures `RankJoinExec` based on which inputs contribute to the `ORDER BY` expression and whether `ScanMode::Standard` or `ScanMode::Tagged` is used:

| Query Shape                                                    | Left Access | Right Access               | Why `RankJoinExec` Wins                                                                                                                                                             |
| :------------------------------------------------------------- | :---------- | :------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| _Disjunctive (`ScanMode::Tagged`, `WHERE a OR b`)_             | `SA + RA`   | `SA + RA`                  | Avoids scanning all non-matching (`score = 0.0`) rows in the database (`base_query = All`). `SA` pulls only positive-score matches; `RA` point-looks up missing partners on demand. |
| _Balanced Conjunctive (`ScanMode::Standard`, `WHERE a AND b`)_ | `SA + RA`   | `SA + RA`                  | Both sides use Block-Max WAND `SA` windows; lazy `RA` point lookups resolve top unmatched keys so orphan rows never force scanning the opposite table to completion.                |
| _Lopsided Score (`ORDER BY 100 * s_a + s_b`)_                  | `SA + RA`   | `SA + RA`                  | Pulls a small `SA` window from `a`, resolves those keys in `b` via `RA` point lookup, and terminates as soon as $100 \cdot s_a^{\text{bottom}} + s_b^{\text{bottom}} \le \theta_K$. |
| _Single-Sided Score / Field (`ORDER BY s_a DESC`, `NRJN`)_     | `SA`        | `RA`-only ($s_R \equiv 0$) | Pulls sorted windows from `a` and probes `b` via `TermSetQuery` (`initial_window = 0.0`) without sorting `b`.                                                                       |

---

## 3. Plan-Time Heuristics (`RankJoinRule`)

`RankJoinRule` inspects:

- $K$: `SortExec::fetch()` (`limit + offset`).
- $M_L, M_R$: Estimated matching documents on each side (`PgSearchScanPlan::planner_estimated_rows`).
- $s_L^{\max}, s_L^{\min}$ and $s_R^{\max}, s_R^{\min}$: Static score bounds (`estimate_query_max_score` on `resolved_query` or `local_queries`).
- `ScanMode`: `ScanMode::Standard` vs. `ScanMode::Tagged`.

`RankJoinRule` rewrites an inner `HashJoinExec` into `RankJoinExec` when the following gates pass:

### Gate 1: Input Eligibility (`SA` and/or `RA`)

1. Each child that contributes to the `ORDER BY` score expression must be able to execute `SA` (`LazySortConfig`):
   - Either a `PgSearchScanPlan` in `ScanMode::Standard` whose query is not a flat constant-score query (`AllQuery` or `ConstScoreQuery`),
   - Or a `PgSearchScanPlan` in `ScanMode::Tagged` with at least one non-constant-score query in `base_query` or `local_queries`,
   - Or a child `RankJoinExec` already producing a ranked stream.
2. Any child that does _not_ contribute to the `ORDER BY` score expression (`combiner.uses_left() == false` or `combiner.uses_right() == false`) must be a leaf `PgSearchScanPlan` in `ScanMode::Standard` (where every valid joined row must match the scored side's `SA` query, making `RA`-only execution on the unscored side sound).

### Gate 2: Score Contribution & Combiner Selection

- If both `left` and `right` contribute positively to the monotonic `ORDER BY` expression (`uses_left && uses_right`), their maximum score contributions $\Delta_L, \Delta_R$ must satisfy $\min(\Delta_L, \Delta_R) \ge 0.20 \cdot \max(\Delta_L, \Delta_R)$, and both inputs are configured for `SA + RA`.
- If only `left` (resp. `right`) contributes to the `ORDER BY` expression, the scored side is configured for `SA` and the unscored side is configured for `RA`-only (`initial_window = 0.0`, score contribution $= 0$).

### Gate 3: Cardinality & Initial Window Size

1. `LIMIT K` is small: $K \le \text{MAX\_RANK\_JOIN\_K}$ (`1,000`).
2. Both inputs must meet the minimum estimated row threshold (`paradedb.rank_join_min_rows`, default `10,000`; set to `0` in unit/regression tests to bypass the size floor).
3. Because `RankJoinExec` resolves unmatched top keys directly via `RA` point lookups rather than waiting for random `SA` window intersections, each scored `SA` leaf scan starts with a small initial window:
   $$W_{\text{init}} = \min\bigl(\max(K \cdot \text{topk\_retry\_scale\_factor},\; 1),\; \text{max\_topk\_chunk\_size}\bigr)$$
   while unscored `RA`-only leaf scans use $W_{\text{init}} = 0$.

---

## 4. Monotonicity Check & Optimizer Rule Ordering

### 4.1 Recognizing Monotonic Scoring Expressions

`RankJoinRule` checks that the `SortExec` expression is monotonic in each participating input by walking the `PhysicalExpr` tree and tracking whether each subexpression is `Increasing` or `Decreasing`:

| Expression Node                            | Allowed When                                                                                  | Effect on Child Direction                                                                     |
| :----------------------------------------- | :-------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------- |
| Leaf column / `pdb.score` / distance       | Belongs to one join input and maps to a Tantivy `OrderByInfo` (or child `RankJoinExec` score) | `Increasing` $\rightarrow$ `DESC`, `Decreasing` $\rightarrow$ `ASC` (when top sort is `DESC`) |
| `CASE WHEN __tag THEN score ELSE 0.0 END`  | Produced by `ScanMode::Tagged` for a table's tag score                                        | Same direction as `score` (`SA` orders by tag query score; `RA` evaluates `0.0` when false)   |
| `a + b`                                    | Always                                                                                        | Preserves direction of both `a` and `b`                                                       |
| `a - b`                                    | Always                                                                                        | Preserves `a`, flips `b`                                                                      |
| `c * a` or `a / c` (constant $c \neq 0$)   | Always                                                                                        | Preserves if $c > 0$, flips if $c < 0$                                                        |
| `a * b`                                    | Both `a` and `b` are non-negative ($\ge 0$, e.g. `pdb.score`, distance, `GREATEST(x, 0)`)     | Preserves direction of both `a` and `b`                                                       |
| `GREATEST(a, b, ...)` / `LEAST(a, b, ...)` | Always                                                                                        | Preserves direction of all arguments                                                          |
| `ln(a)`, `sqrt(a)`, `exp(a)`               | Always on valid domain                                                                        | Preserves direction of `a`                                                                    |

### 4.2 Rule Ordering in `build_base_session` (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`)

1. `RangePartitioningRule` & `LateMaterializationRule`
2. `RangeCoPartitionedJoinRule` (flips eligible `CollectLeft` hash joins to `Partitioned` when range co-partitioned)
3. `RankJoinRule` (rewrites eligible `HashJoinExec`s into `RankJoinExec`)
4. `FilterPushdown` (pushes `RankJoinExec`'s `RankLookupPhysicalExpr` dynamic filters down through `ProjectionExec` / child `RankJoinExec`s to leaf `PgSearchScanPlan`s)
5. `EnsureRequirements` (`EnforceDistribution` and `EnforceSorting`)
6. `DeferredPlacementRule` & `DeferredAggregateRule` (keep string decoding deferred to the final `K` rows above the join)
7. `SegmentedTopKRule` (applies to remaining One-Pass `HashJoinExec` plans)
8. `DistributedPhysicalOptimizerRule` (slices the plan into MPP stages)
