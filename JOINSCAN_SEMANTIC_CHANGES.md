# JoinScan Migration: Semantic Changes & Architectural Evolution

This document summarizes the semantic, architectural, and behavioral changes introduced during the migration of ParadeDB's `JoinScan` from PostgreSQL's `set_join_pathlist_hook` to `create_upper_paths_hook` at `UPPERREL_FINAL`, rooted in the design and goals described in `JOINSCAN_MIGRATION_PLAN.md`.

---

## 1. Motivation & Core Architectural Shift

### Background (`JOINSCAN_MIGRATION_PLAN.md`)

Previously, `JoinScan` hooked into PostgreSQL's query planner via `set_join_pathlist_hook`. This hook runs bottom-up for every candidate relation pair `(outerrel, innerrel)` explored during join search. Operating at this stage forced `JoinScan` to:

- Reconstruct global join trees across multiple permutations and intermediate join paths.
- Reverse-engineer PostgreSQL's join search order and maintain partial states.
- Maintain ~2,500+ lines of fragile bottom-up join resolution, clause absorbing, and pathlist stitching across `joinscan/mod.rs`, `joinscan/planning.rs`, and `joinscan/predicate.rs`.

### The Shift to `UPPERREL_FINAL`

`create_upper_paths_hook` is called after relation scan and join exploration are complete. Placing `JoinScan` at `UPPERREL_FINAL`:

- **Single-pass AST construction**: Direct extraction of the complete join tree from `root->parse->jointree` using the shared parse-tree pipeline in `aggregatescan/datafusion_build.rs`.
- **Root node ownership**: At `UPPERREL_FINAL`, the `CustomPath` attaches directly to `final_rel`. The resulting `CustomScan` becomes the root node of the execution plan, eliminating redundant upper PostgreSQL `Sort`, `Limit`, or `Projection` nodes.
- **Unified pipeline**: Scans, join conditions, join-level filters, score bubbling, DISTINCT deduplication, ORDER BY sorting, and LIMIT/OFFSET are encapsulated inside a single DataFusion plan.

---

## 2. Semantic & Behavioral Changes

### A. Full Query Pipeline Ownership at `UPPERREL_FINAL`

- **Plan Topology**: Join queries eligible for `JoinScan` now produce a plan headed directly by `Custom Scan (ParadeDB Join Scan)` without being wrapped by PostgreSQL `Sort` or `Limit` nodes.
- **TopK Pushdown**: Sorting and limits are lowered into DataFusion's `SortExec(TopK)` directly within the scan, preserving early termination and eliminating PostgreSQL sort buffers.

### B. Absorption of `SELECT DISTINCT`

- **Evolution from Plan**: `JOINSCAN_MIGRATION_PLAN.md` initially envisioned `AggregateScan` handling `UPPERREL_DISTINCT`. However, doing so at `UPPERREL_DISTINCT` left upper `Sort` and `Limit` nodes in PostgreSQL, preventing TopK execution for `SELECT DISTINCT ... ORDER BY ... LIMIT ...`.
- **JoinScan DISTINCT**: `JoinScan` absorbed `DISTINCT` at `UPPERREL_FINAL`.
- **Execution Mechanism**:
  - All DISTINCT target list entries must be fast fields, score functions, or pushdown-safe expressions.
  - Deduplication runs inside DataFusion via `AggregateExec` grouping by output columns and aggregating surviving table `ctid`s (`min(ctid)`).
  - Surviving rows are late-materialized from PostgreSQL heap storage after deduplication.
  - Sorting and limits apply directly to the distinct output stream.

### C. End-to-End Target List Expression Support

- **Full Expression Pushdown**: At `UPPERREL_FINAL`, no upper PostgreSQL projection node exists to compute unhandled target list expressions. JoinScan now evaluates arbitrary expressions directly in DataFusion for both DISTINCT and non-DISTINCT queries.
- **Native Score Translation**:
  - `paradedb.score(rel.id)` within expressions is translated directly to DataFusion score columns (`make_source_score_col(source)`).
  - Expressions combining scores (e.g. `paradedb.score(doc.id) + paradedb.score(file.id) AS score`) execute natively via DataFusion arithmetic rather than failing or routing to UDF evaluation.
  - Variables inside `paradedb.score(...)` only require relation membership in the join; they do not need fast fields or column decodes from Tantivy.
- **General Expression & Constant Support**:
  - Binary arithmetic (`+`, `-`, `*`, `/`, `%`), string operations, and math functions evaluate natively in DataFusion.
  - Constants and literals (e.g. `'BM25' AS source`) evaluate via `datafusion::logical_expr::lit`.
  - Non-natively mapped functions fall back to `PgExprUdf` as long as their variable dependencies are fast fields.
- **Bug Fix**: Fixed a prior issue where non-DISTINCT target list expressions were marked `Pruned` and projected `col_N = NULL` in DataFusion, returning NULL datums in query results. They now compute exact calculated values.

### D. SubPlan & Semi/Anti Join Extraction

- Reused the AST join tree walker from `datafusion_build.rs` to extract top-level subplans into semi and anti joins via `classify_base_restrictinfo` and `wrap_with_semi_anti`.
- Eliminates custom bottom-up subplan absorbing logic from `joinscan/predicate.rs`.

### E. Clean Validation Spine

- Target list validation is centralized in `planning::resolve_target_entry_expr`:
  - Rejects aggregates and window functions.
  - Verifies non-score variable dependencies belong to join sources and are fast fields (or lateral unnests).
  - Verifies return types are Arrow-convertible.
  - Reused across both DISTINCT and non-DISTINCT validation pathways.

---

## 3. Codebase Reductions & Cleanup

- **Removed `set_join_pathlist` infrastructure**:
  - Deleted `register_join_pathlist` and `JoinPathlistHookArgs` from `hook.rs`.
  - Removed `set_join_pathlist` registration in `lib.rs`.
- **Eliminated Dead Planning & Translation Routines**:
  - Removed `add_missing_search_operators_to_tlist` in `utils.rs`.
  - Removed `deparse_planner_expr_or_raw` in `deparse.rs`.
  - Removed `RelNode::has_absorbed_search_clauses` in `build.rs`.
  - Removed ~400 lines of obsolete bottom-up clause distribution and restriction handling in `joinscan/predicate.rs`.
  - Cleaned up obsolete comments referencing bottom-up join pathlist exploration across `joinscan/mod.rs`, `planning.rs`, and `hook.rs`.
- **Net Impact**: Net reduction of over 950 lines of code (`5,338 insertions(+), 6,287 deletions(-)`).

---

## 4. Test Suite Impact

- 65 test output files updated across `pg_search/tests/pg_regress/expected/`.
- Verified recovery of `JoinScan` execution and clean plans across:
  - `joinscan_sortby_score.out`: 12 regressed queries restored to JoinScan with native score sum evaluation.
  - `joinscan_cross_table_or.out`: Constant expressions (`'BM25' AS source`) restored to JoinScan.
  - `join_distinct.out` & `join_distinct_expr.out`: Full DISTINCT coverage with TopK and late materialization.
  - `join_orderby_expression.out` & `join_order_by_alias_expression.out`: Direct evaluation of sort expressions.
- Eliminated all instances of `WARNING: JoinScan not used: expressions in target list without DISTINCT are not supported`.
