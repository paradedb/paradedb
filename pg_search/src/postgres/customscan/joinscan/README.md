# JoinScan

JoinScan intercepts PostgreSQL join planning and replaces the standard executor with a DataFusion-based pipeline that operates entirely on Tantivy's columnar fast fields. The core strategy is **late materialization**: execute the join using only index data, apply sorting and limits, then access the PostgreSQL heap only for the final K result rows.

## Physical Plan

For a typical `SELECT ... FROM files JOIN documents ... ORDER BY title LIMIT K`:

```txt
ProjectionExec
  TantivyLookupExec                   ← materializes deferred strings for final K rows only
    SegmentedTopKExec                 ← global threshold pruning + final sort + LIMIT K
      HashJoinExec                    ← join on fast fields
        PgSearchScan (documents)      ← BM25 search
        PgSearchScan (files)          ← lazy scan, deferred columns, receives dynamic filters
```

[`SegmentedTopKExec`][topk-exec] publishes dynamic filter thresholds that are pushed down through the join to the probe-side scan, pruning rows at the scanner level. It also performs the final materialized sort and LIMIT, so `TantivyLookupExec` only decodes K rows (not K×segments).

## How It Works

### 1. Activation

JoinScan fires when all conditions are met: LIMIT present, equi-join keys exist, all columns are fast fields, all tables have BM25 indexes, and at least one `@@@` predicate. See [`create_custom_path()`][activation] for the full checklist.

### 2. Planning

The planner hook builds a [`JoinCSClause`][joincsc] — a serializable IR capturing the [`RelNode`][relnode] join tree, predicates, ORDER BY, and LIMIT. This is stored in `CustomScan.custom_private` and deserialized at execution time.

- [`build.rs`](build.rs) — `RelNode`, `JoinCSClause`, `JoinSource`
- [`planning.rs`](planning.rs) — cost estimation, field validation
- [`predicate.rs`](predicate.rs) — Postgres expression translation
- [`privdat.rs`](privdat.rs) — serialization

### 3. Physical Plan Construction

[`scan_state.rs`](scan_state.rs) builds a DataFusion logical plan from the `JoinCSClause`, then runs [physical optimization][optimizer-rules]:

1. **[`SortMergeJoinEnforcer`][smj-enforcer]** — converts HashJoin to SortMergeJoin when inputs are pre-sorted
2. **FilterPushdown (Post)** — pushes dynamic filters through the join
3. **`LateMaterializationRule`** — injects [`TantivyLookupExec`][lookup-exec] to defer string materialization
4. **[`SegmentedTopKRule`][topk-rule]** — injects [`SegmentedTopKExec`][topk-exec] for Top K on deferred columns, removes the now-redundant `SortExec(TopK)`, [wraps blocking nodes][wrap-blocking] with [`FilterPassthroughExec`][filter-passthrough]
5. **FilterPushdown (Post) — [second pass][second-pushdown]** — pushes `SegmentedTopKExec`'s `DynamicFilterPhysicalExpr` down to the scan

### 4. Deferred Columns

String columns are emitted as a [3-way `UnionArray`](../../scan/deferred_encode.rs) (doc_address | term_ordinal | materialized) so intermediate nodes work with cheap integer ordinals instead of decoded strings. The [decision to defer](../../scan/table_provider.rs) is made in [`configure_deferred_outputs()`][defer-decision].

### 5. Pruning Path

There are two primary pruning mechanisms for dynamic filters that are pushed down to the scan:

1. **Query-Time Pushdown (Inverted Index):** Filters that are static and known at the start of the scan (such as `InList` predicates generated from a `HashJoin` build side) are intercepted during the first `poll_next` of the scan stream. They are converted into native Tantivy queries (e.g., `TermSetQuery`) and `AND`ed into the main search query via [`try_dynamic_filter_pushdown`][try-pushdown]. This allows Tantivy to use its inverted index to filter documents _while_ executing the search, providing the highest possible pruning performance. The DataFusion expressions are then rewritten to `lit(true)` so they are not evaluated again.

2. **Pre-Filter Pushdown (Fast Fields):** For evolving thresholds, such as the global threshold from [`SegmentedTopKExec`][topk-exec], the threshold is pushed down to the scan via filter pushdown. This works because `SegmentedTopKExec` and [`PgSearchScan`][scan-plan] share an `Arc<DynamicFilterPhysicalExpr>`. The [scanner reads `current()`][scanner-next] on every batch and applies the filter _after_ the search but _before_ Arrow column materialization. For strings, it translates literals to per-segment ordinal bounds via [`try_rewrite_binary`][rewrite-binary] and filters directly against the fetched term ordinals.

### 6. Execution Result

After all input is consumed, `SegmentedTopKExec` materializes sort column values, performs the final sort, and emits exactly K rows. `TantivyLookupExec` decodes deferred strings for those K rows only. JoinScanState extracts CTIDs and fetches heap tuples — the only point where the PostgreSQL heap is accessed.

## Key Files

| File                             | Purpose                                                                               |
| -------------------------------- | ------------------------------------------------------------------------------------- |
| [`mod.rs`](mod.rs)               | Lifecycle, [activation checks][activation], parallel support                          |
| [`build.rs`](build.rs)           | [`RelNode`][relnode], [`JoinCSClause`][joincsc], `JoinSource`                         |
| [`scan_state.rs`](scan_state.rs) | DataFusion plan building, [optimizer registration][optimizer-rules], result streaming |
| [`planner.rs`](planner.rs)       | [`SortMergeJoinEnforcer`][smj-enforcer], `FilterPassthroughExec` usage                |
| [`planning.rs`](planning.rs)     | Cost estimation, field validation, ORDER BY extraction                                |
| [`predicate.rs`](predicate.rs)   | Postgres expression → `JoinLevelExpr`                                                 |
| [`translator.rs`](translator.rs) | Postgres ↔ DataFusion expression mapping                                              |
| [`explain.rs`](explain.rs)       | EXPLAIN output formatting                                                             |

Execution-layer files under [`pg_search/src/scan/`](../../scan/):

| File                                                  | Purpose                                                                                                                             |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| [`segmented_topk_exec.rs`][topk-exec]                 | [`SegmentedTopKExec`][topk-exec] — per-segment heaps, [global heap][global-heap], [`build_global_filter_expression`][global-filter] |
| [`segmented_topk_rule.rs`][topk-rule]                 | Optimizer rule, [`wrap_blocking_nodes`][wrap-blocking]                                                                              |
| [`tantivy_lookup_exec.rs`][lookup-exec]               | Dictionary decode + [filter passthrough][lookup-passthrough]                                                                        |
| [`filter_passthrough_exec.rs`][filter-passthrough]    | Transparent wrapper enabling filter pushdown through blocking nodes                                                                 |
| [`batch_scanner.rs`](../../scan/batch_scanner.rs)     | [`Scanner::next()`][scanner-next] — batch iteration, pre-filter, visibility                                                         |
| [`execution_plan.rs`](../../scan/execution_plan.rs)   | [`PgSearchScanPlan`][scan-plan] — dynamic filter integration                                                                        |
| [`pre_filter.rs`](../../scan/pre_filter.rs)           | [`try_rewrite_binary`][rewrite-binary], [`collect_filters`][collect-filters]                                                        |
| [`deferred_encode.rs`](../../scan/deferred_encode.rs) | 3-way UnionArray construction and unpacking                                                                                         |

## GUCs

| GUC                                | Default | Effect                        |
| ---------------------------------- | ------- | ----------------------------- |
| `paradedb.enable_join_custom_scan` | `on`    | Master switch                 |
| `paradedb.enable_segmented_topk`   | `true`  | `SegmentedTopKExec` injection |
| `paradedb.enable_columnar_sort`    | `true`  | Enables SortMergeJoin path    |

[activation]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/postgres/customscan/joinscan/mod.rs#L317
[relnode]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/postgres/customscan/joinscan/build.rs#L575
[joincsc]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/postgres/customscan/joinscan/build.rs#L796
[optimizer-rules]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/postgres/customscan/joinscan/scan_state.rs#L176-L213
[second-pushdown]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/postgres/customscan/joinscan/scan_state.rs#L213
[smj-enforcer]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/postgres/customscan/joinscan/planner.rs#L60
[topk-exec]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/segmented_topk_exec.rs#L150
[global-filter]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/segmented_topk_exec.rs#L924
[global-heap]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/segmented_topk_exec.rs#L447
[topk-rule]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/segmented_topk_rule.rs#L63
[wrap-blocking]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/segmented_topk_rule.rs#L284
[filter-passthrough]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/filter_passthrough_exec.rs#L39
[lookup-exec]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/tantivy_lookup_exec.rs#L60
[lookup-passthrough]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/tantivy_lookup_exec.rs#L232
[scan-plan]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/execution_plan.rs#L89
[scanner-next]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/batch_scanner.rs#L259
[rewrite-binary]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/pre_filter.rs#L383
[collect-filters]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/pre_filter.rs#L254
[defer-decision]: https://github.com/paradedb/paradedb/blob/53b9d11/pg_search/src/scan/table_provider.rs#L126
[try-pushdown]: ../../scan/pre_filter.rs
