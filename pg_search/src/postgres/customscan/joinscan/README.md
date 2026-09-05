# JoinScan

JoinScan intercepts PostgreSQL join planning and replaces the standard executor with a DataFusion-based pipeline that operates entirely on Tantivy's columnar fast fields. The core strategy is **late materialization**: execute the join using only index data, apply sorting and limits, then access the PostgreSQL heap only for the final K result rows.

## Physical Plan

For a typical `SELECT ... FROM files JOIN documents ... ORDER BY title LIMIT K`:

```txt
ProjectionExec
  TantivyDecodeExec                   ← decodes term ordinals to strings for final K rows only
    TantivyFetchExec                  ← resolves doc addresses to term ordinals for those rows
      SegmentedTopKExec               ← global threshold pruning + final sort + LIMIT K
        HashJoinExec                  ← join on fast fields
          PgSearchScan (documents)    ← BM25 search
          PgSearchScan (files)        ← lazy scan, deferred columns, receives dynamic filters
```

When a viable PostgreSQL parallel launch is available, JoinScan uses Massively Parallel Processing (MPP) via `datafusion-distributed` to parallelize queries. DataFusion's in-process multithreading is bypassed because PostgreSQL has already launched independent parallel worker processes. For a viable MPP launch, `DistributedPlanner` slices the physical plan into network stages (`DistributedExec`); logical tasks are assigned round-robin across the workers that attached, so one worker may host multiple tasks.

[`SegmentedTopKExec`][topk-exec] publishes dynamic filter thresholds that are pushed down through the join to the probe-side scan, pruning rows at the scanner level. It also performs the final materialized sort and LIMIT, so the lookup above it only fetches and decodes K rows (not K×segments).

## How It Works

### 1. Activation

JoinScan fires when all conditions are met: LIMIT present, equi-join keys exist, all columns are fast fields, all tables have ParadeDB indexes, and at least one `@@@` predicate. See [`create_custom_path()`][activation] for the full checklist.

### 2. Planning

The planner hook builds a [`JoinCSClause`][joincsc] — a serializable IR capturing the [`RelNode`][relnode] join tree, predicates, ORDER BY, and LIMIT. This is stored in `CustomScan.custom_private` and deserialized at execution time.

- [`build.rs`](build.rs) — `RelNode`, `JoinCSClause`, `JoinSource`
- [`planning.rs`](planning.rs) — cost estimation, field validation
- [`predicate.rs`](predicate.rs) — Postgres expression translation
- [`privdat.rs`](privdat.rs) — serialization

### 3. Physical Plan Construction

[`scan_state.rs`](scan_state.rs) builds a DataFusion logical plan from the `JoinCSClause`, then runs [physical optimization][optimizer-rules]:

1. **[`RangePartitioningRule`](range_partitioning_rule.rs)** — coordinates split points across joins for MPP range partitioning, sampling both sides of the join and injecting the merged sample into both `PgSearchTableProvider`s
2. **`LateMaterializationRule`** — injects [`TantivyDecodeExec`][decode-exec] over [`TantivyFetchExec`][fetch-exec] to defer string materialization. With `paradedb.defer_column_fetch = off` the scan resolves term ordinals itself and only the decode node is injected
3. **[`RangeCoPartitionedJoinRule`](range_partitioning_rule.rs)** — flips a `CollectLeft` inner hash join to `Partitioned` mode when both sides declare compatible `Partitioning::Range` layouts, so MPP joins partition pairs task-locally instead of broadcasting the build side
4. **[`SegmentedTopKRule`][topk-rule]** — injects [`SegmentedTopKExec`][topk-exec] for Top K on deferred columns, removes the now-redundant `SortExec(TopK)` and transfers ownership of its already pushed-down `DynamicFilterPhysicalExpr` into the injected node, [wraps blocking nodes][wrap-blocking] with [`FilterPassthroughExec`][filter-passthrough]

When MPP is eligible, `DistributedPlanner` builds an MPP execution tree (`DistributedExec`), slicing it into isolated tasks. Plans with fewer than two producer tasks, or launches with fewer than two attached workers, run serially.

### 4. Deferred Columns

String columns are emitted as a [2-way `UnionArray`](../../../scan/deferred_encode.rs) (doc_address | term_ordinal) so intermediate nodes work with cheap integer ordinals instead of decoded strings. The [decision to defer](../../../scan/table_provider.rs) is made in [`configure_deferred_outputs()`][defer-decision].

The lookup has two halves with different access patterns, so they are separate nodes. [`TantivyFetchExec`][fetch-exec] reads the fast-field column (doc_address → term_ordinal, and packed ctid → real ctid); it wants doc order, which a join above the scan no longer keeps. [`TantivyDecodeExec`][decode-exec] reads the segment dictionary (term_ordinal → string); it is random access either way, and ordinals are much narrower than strings, so it can move above joins and shuffles at the same cost per row. The planner places the two next to each other; a cost model may separate them. `paradedb.defer_column_fetch` chooses between fetching in the scan (State 1 out of the scan, in doc order) and fetching at the decode point.

### 5. Pruning Path

There are two primary pruning mechanisms for dynamic filters that are pushed down to the scan:

1. **Query-Time Pushdown (Inverted Index):** Filters that are static and known at the start of the scan (such as `InList` predicates generated from a `HashJoin` build side) are intercepted during the first `poll_next` of the scan stream. They are converted into native Tantivy queries (e.g., `TermSetQuery`) and `AND`ed into the main search query via [`try_dynamic_filter_pushdown`][try-pushdown]. This allows Tantivy to use its inverted index to filter documents _while_ executing the search, providing the highest possible pruning performance. The DataFusion expressions are then rewritten to `lit(true)` so they are not evaluated again.

2. **Pre-Filter Pushdown (Fast Fields):** For evolving thresholds, such as the global threshold from [`SegmentedTopKExec`][topk-exec], the threshold is pushed down to the scan via filter pushdown. This works because `SegmentedTopKExec` and [`PgSearchScan`][scan-plan] share an `Arc<DynamicFilterPhysicalExpr>`. The [scanner reads `current()`][scanner-next] on every batch and applies the filter _after_ the search but _before_ Arrow column materialization. For strings, it translates literals to per-segment ordinal bounds via [`try_rewrite_binary`][rewrite-binary] and filters directly against the fetched term ordinals.

### 6. Execution Result

After all input is consumed, `SegmentedTopKExec` materializes sort column values, performs the final sort, and emits exactly K rows. The lookup above it fetches and decodes deferred strings for those K rows only. JoinScanState extracts CTIDs and fetches heap tuples — the only point where the PostgreSQL heap is accessed.

### 7. MPP Execution and Parallelism

JoinScan does not use DataFusion's standard in-process multithreading. Since PostgreSQL already coordinates execution across independent backend processes via the `Gather` node, relying on thread-level parallelism inside a Postgres worker would result in `Workers * Threads` explosions, and Postgres does not support interacting with its APIs anywhere but on the `main` thread.

Instead, MPP via `datafusion-distributed` is our only mechanism for parallelizing joins. It assigns logical tasks across PostgreSQL parallel workers based on segment count:

1. **Partition Output Definition**: Because index segments are checked out atomically from shared memory, [`PgSearchScanPlan`][scan-plan] natively partitions its output by the number of segments. In [`table_provider.rs`](../../../scan/table_provider.rs), we formally expose the scan's output partition count as `min(segment_count, target_partitions)`. When the `RangePartitioningRule` has injected a range sample, the scan instead declares `Partitioning::Range` with the sample's split points, which lets DataFusion treat the two sides of a join as co-partitioned.
2. **Task Estimation**: During MPP planning, [`PgSearchScanTaskEstimator`](../../../scan/execution_plan.rs) intercepts the leaf nodes and requests exactly this `partition_count` number of tasks.
3. **Execution Routing**: For a viable MPP launch, tasks are assigned round-robin across the workers PostgreSQL attached, and each worker uses `ParallelScanState` to lazily claim segments. A one-task plan does not launch MPP workers and runs serially.

## Key Files

| File                                                       | Purpose                                                                               |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| [`mod.rs`](mod.rs)                                         | Lifecycle, [activation checks][activation], parallel support                          |
| [`build.rs`](build.rs)                                     | [`RelNode`][relnode], [`JoinCSClause`][joincsc], `JoinSource`                         |
| [`scan_state.rs`](scan_state.rs)                           | DataFusion plan building, [optimizer registration][optimizer-rules], result streaming |
| [`planning.rs`](planning.rs)                               | Cost estimation, field validation, ORDER BY extraction                                |
| [`predicate.rs`](predicate.rs)                             | Postgres expression → `JoinLevelExpr`                                                 |
| [`range_partitioning_rule.rs`](range_partitioning_rule.rs) | Rules that synchronize Join-side MPP partition boundaries and co-partition the join   |
| [`translator.rs`](../datafusion/translator.rs)             | Postgres ↔ DataFusion expression mapping                                              |
| [`explain.rs`](../datafusion/explain.rs)                   | EXPLAIN output formatting                                                             |

Execution-layer files under [`pg_search/src/scan/`](../../../scan/):

| File                                                     | Purpose                                                                                                                             |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| [`segmented_topk_exec.rs`][topk-exec]                    | [`SegmentedTopKExec`][topk-exec] — per-segment heaps, [global heap][global-heap], [`build_global_filter_expression`][global-filter] |
| [`segmented_topk_rule.rs`][topk-rule]                    | Optimizer rule, [`wrap_blocking_nodes`][wrap-blocking]                                                                              |
| [`tantivy_fetch_exec.rs`][fetch-exec]                    | Fast-field fetch: doc address → term ordinal, packed ctid → ctid                                                                    |
| [`tantivy_decode_exec.rs`][decode-exec]                  | Dictionary decode: term ordinal → string/bytes                                                                                      |
| [`filter_passthrough_exec.rs`][filter-passthrough]       | Transparent wrapper enabling filter pushdown through blocking nodes                                                                 |
| [`batch_scanner.rs`](../../../scan/batch_scanner.rs)     | [`Scanner::next()`][scanner-next] — batch iteration, pre-filter, visibility                                                         |
| [`execution_plan.rs`](../../../scan/execution_plan.rs)   | [`PgSearchScanPlan`][scan-plan] — dynamic filter integration                                                                        |
| [`pre_filter.rs`](../../../scan/pre_filter.rs)           | [`try_rewrite_binary`][rewrite-binary], [`collect_filters`][collect-filters]                                                        |
| [`deferred_encode.rs`](../../../scan/deferred_encode.rs) | 2-way UnionArray construction and unpacking                                                                                         |

## GUCs

| GUC                                      | Default | Effect                                                                |
| ---------------------------------------- | ------- | --------------------------------------------------------------------- |
| `paradedb.enable_join_custom_scan`       | `on`    | Master switch                                                         |
| `paradedb.enable_range_partitioned_join` | `false` | Range co-partitioned joins                                            |
| `paradedb.enable_segmented_topk`         | `true`  | `SegmentedTopKExec` injection                                         |
| `paradedb.defer_column_fetch`            | `true`  | Fetch term ordinals at the decode point (`on`) or in the scan (`off`) |

[activation]: mod.rs
[relnode]: build.rs
[joincsc]: build.rs
[optimizer-rules]: scan_state.rs
[topk-exec]: ../../../scan/segmented_topk_exec.rs
[global-filter]: ../../../scan/segmented_topk_exec.rs
[global-heap]: ../../../scan/segmented_topk_exec.rs
[topk-rule]: ../../../scan/segmented_topk_rule.rs
[wrap-blocking]: ../../../scan/segmented_topk_rule.rs
[filter-passthrough]: ../../../scan/filter_passthrough_exec.rs
[fetch-exec]: ../../../scan/tantivy_fetch_exec.rs
[decode-exec]: ../../../scan/tantivy_decode_exec.rs
[scan-plan]: ../../../scan/execution_plan.rs
[scanner-next]: ../../../scan/batch_scanner.rs
[rewrite-binary]: ../../../scan/pre_filter.rs
[collect-filters]: ../../../scan/pre_filter.rs
[defer-decision]: ../../../scan/table_provider.rs
[try-pushdown]: ../../../scan/pre_filter.rs
