# JoinScan Migration Plan: Moving from `set_join_pathlist` to `create_upper_paths_hook`

## 1. Executive Summary

This document proposes an architectural and implementation plan to migrate ParadeDB's `JoinScan` from PostgreSQL's `set_join_pathlist_hook` to `create_upper_paths_hook`.

### Motivation

`set_join_pathlist_hook` is called bottom-up for every candidate pair of relations (`outerrel`, `innerrel`) during join tree exploration. This forced `JoinScan` to:

- Reconstruct the join tree across permutations and planner alternative join types (`JOIN_UNIQUE_*`, `JOIN_RIGHT_*`).
- Reverse-engineer PostgreSQL's join tree search order.
- Maintain complex and fragile logic in `pg_search/src/postgres/customscan/joinscan/planning.rs` and `pg_search/src/postgres/customscan/joinscan/mod.rs` (~2,500+ lines of code).

In contrast, `create_upper_paths_hook` is called after scan and join path exploration is complete. `AggregateScan` already uses this hook to reconstruct the join tree directly in one pass from the query AST (`root->parse->jointree`) via `datafusion_build.rs`. Moving `JoinScan` to `create_upper_paths_hook` allows both scans to share the exact same parse-tree join extraction logic, eliminating the bottom-up join structure parsing in `JoinScan`.

---

## 2. Target Stage: `UPPERREL_FINAL`

In PostgreSQL's `grouping_planner`, upper relation stages run in sequence:

1. `UPPERREL_GROUP_AGG`: GROUP BY and aggregate functions (**owned by `AggregateScan`**).
2. `UPPERREL_WINDOW`: Window functions.
3. `UPPERREL_DISTINCT`: `SELECT DISTINCT` (**owned by `AggregateScan`**).
4. `UPPERREL_ORDERED`: ORDER BY sort paths.
5. `UPPERREL_FINAL`: Final relation processing (LIMIT, OFFSET, projections, rowmarks).

### Why `UPPERREL_FINAL` for `JoinScan`?

- **Guaranteed invocation**: Unlike `UPPERREL_ORDERED`, which only runs if `parse->sortClause != NULL`, `UPPERREL_FINAL` is called for every query that passes through `grouping_planner` (including queries with `LIMIT` but no `ORDER BY`).
- **Complete context**: At `UPPERREL_FINAL`, `final_rel->reltarget` defines the exact final output projection, `root->parse->sortClause` provides the ordering requirements, and `extra` (`FinalPathExtraData`) provides limit and offset information.
- **Direct TopK execution**: `JoinScan` already computes TopK natively in DataFusion (`SortExec(fetch=K)`). By placing the `CustomPath` directly on `final_rel`, `JoinScan` becomes the root node of the plan, executing the entire query pipeline without requiring redundant PostgreSQL `Sort` or `Limit` nodes above it.

### Scan Stage Partitioning

| CustomScan          | Stage(s)                                  | Criteria                                                                                                          |
| :------------------ | :---------------------------------------- | :---------------------------------------------------------------------------------------------------------------- |
| **`AggregateScan`** | `UPPERREL_GROUP_AGG`, `UPPERREL_DISTINCT` | Queries with aggregates, `GROUP BY`, `pdb.agg()`, or `DISTINCT`.                                                  |
| **`JoinScan`**      | `UPPERREL_FINAL`                          | Join queries without aggregates/grouping, requiring BM25 search predicates and late materialization with `LIMIT`. |

---

## 3. SubPlan Handling

`AggregateScan`'s `datafusion_build.rs` already lifts top-level subplans into semi/anti joins via `classify_base_restrictinfo` and `wrap_with_semi_anti`. By sharing this parse-tree extraction machinery, `JoinScan` automatically inherits support for top-level SubPlans without additional bottom-up logic.

The secondary `register_subplan_join_pathlist` hook (which handles un-flattened `OR`-nested SubPlans on base relations at `set_rel_pathlist_hook`) will be left intact initially to avoid regressions on those specific patterns.

---

## 4. Phased Implementation Plan

### Phase 1: Shared Parse-Tree Join Extraction

Extract the join extraction and validation machinery out of `pg_search/src/postgres/customscan/aggregatescan/datafusion_build.rs` into a shared module (e.g. `pg_search/src/postgres/customscan/join_build.rs`):

- `collect_join_sources`: Extract participating RTIs and index metadata from `input_rel.relids`.
- `build_relnode_from_fromexpr`: Walk `root->parse->jointree` (`FromExpr`, `JoinExpr`) to build the initial `RelNode` skeleton.
- `check_join_path_predicates`: Extract equi-join keys and search clauses from the lower path's `RestrictInfo`s.
- `inject_equi_keys`: Distribute equi-join keys to the proper join levels in `RelNode`.
- `populate_required_fields`: Verify and register fast-field requirements for join keys, filters, and projections.

`aggregatescan/datafusion_build.rs` will re-export or reference these shared functions, ensuring `AggregateScan` functionality remains unaffected.

### Phase 2: Hook Infrastructure & Dispatch

1. **Update `JoinScan::Args`**:
   Change `type Args = CreateUpperPathsHookArgs` in `JoinScan` (`pg_search/src/postgres/customscan/joinscan/mod.rs`).
2. **Refactor `paradedb_upper_paths_callback`** (`pg_search/src/postgres/customscan/hook.rs`):
   Instead of hardcoding stage and GUC checks in the generic callback, delegate stage and GUC filtering to each scan:
   - `AggregateScan` checks `stage == UPPERREL_GROUP_AGG || stage == UPPERREL_DISTINCT` and `enable_aggregate_custom_scan()`.
   - `JoinScan` checks `stage == UPPERREL_FINAL` and `enable_join_custom_scan()`.
3. **Update registration in `pg_search/src/lib.rs`**:
   Register both scans with `register_upper_path`:
   ```rust
   customscan::register_upper_path(customscan::aggregatescan::AggregateScan);
   customscan::register_upper_path(customscan::joinscan::JoinScan);
   ```

### Phase 3: Implement `JoinScan` at `UPPERREL_FINAL`

1. **`JoinScan::create_custom_path`**:
   - **Eligibility Gates**:
     - Check `stage == UPPERREL_FINAL` and `enable_join_custom_scan()`.
     - Reject queries with aggregates (`parse->hasAggs || parse->groupClause != NULL`).
     - Reject queries with `DISTINCT` (handled by `AggregateScan`).
     - Reject queries without `LIMIT` (JoinScan requirement for late materialization).
   - **Join Tree Extraction**:
     - Call shared `extract_join_tree_from_parse(root, &sources, path_info)`.
   - **Output Projection & Late Materialization**:
     - Inspect `final_rel->reltarget` to map requested columns to `OutputColumnInfo` (`Var`, `Score`, `Unnested`, `Pruned`).
     - Ensure `ctid` is requested for relations requiring heap tuple materialization.
     - Populate required fast fields using `populate_required_fields`.
   - **Sort & Limit Handling**:
     - Extract `sortClause` from `root->parse` and validate that ORDER BY columns exist as fast fields.
     - Extract `LimitOffset` from `root` / `extra`.
     - Build `JoinCSClause` with TopK pushdown.
   - **Costing & Path Creation**:
     - Calculate cost for fast-field scans, joins, DataFusion TopK, and random heap fetches for the limited row count.
     - Build `CustomPath` attached to `final_rel`.
2. **`JoinScan::plan_custom_path`**:
   - Set `scanrelid = 0`.
   - Set `scan.plan.targetlist` and `custom_scan_tlist` using `final_rel->reltarget`.
   - Splice multi-table clauses and heap condition clauses into `custom_exprs` so `setrefs.c` converts their Vars into `INDEX_VAR` references.

### Phase 4: Deregister `JoinScan` from `set_join_pathlist` & Clean Up Legacy Code

1. Remove `customscan::register_join_pathlist(customscan::joinscan::JoinScan)` from `pg_search/src/lib.rs`.
2. Remove unused bottom-up join planning routines in `pg_search/src/postgres/customscan/joinscan/planning.rs` and `pg_search/src/postgres/customscan/joinscan/mod.rs`.
3. Keep `register_subplan_join_pathlist` in `set_rel_pathlist_hook` intact.

### Phase 5: Verification & Regression Testing

1. Verify clean `cargo check -p pg_search`.
2. Run regression tests (requesting the user to run them, per rules).
3. Validate EXPLAIN plans and result correctness across:
   - INNER, SEMI, ANTI joins
   - Multi-table joins (3+ tables)
   - `ORDER BY` + `LIMIT`
   - `LIMIT` without `ORDER BY`
   - Lateral unnest queries
   - Score sorting (`pdb.score`)
   - Parallel / MPP execution
