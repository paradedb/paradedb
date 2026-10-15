# Iterative Top-K Lookup Join Design

## 1. Motivation & Overview

In `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`), DataFusion's `HashJoinExec` and `NestedLoopJoinExec` require fully buffering the build side of the join before probing. When a join query has a `LIMIT K` and an `ORDER BY` that can be satisfied by Tantivy's Top-K collector on one of the joined relations (e.g., `ORDER BY pdb.score(a.id) DESC`, `ORDER BY a.fast_field ASC`, or `ORDER BY a.embedding <=> '[...]'`), materializing an entire join input is often unnecessary work.

In `BaseScan`, `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) already uses an iterative windowing strategy over Tantivy's `search_top_k_in_segments`:

1. Request a batch of `local_limit` rows at `offset = 0` in sorted order from Tantivy.
2. Filter the returned rows (e.g., for MVCC visibility).
3. If fewer than `K` rows survive, advance `offset += local_limit`, scale `local_limit` by `topk_chunk_scale_factor`, and re-execute the Tantivy Top-K query from scratch.

Because Tantivy's Top-K collector (Block-WAND for score, fast-field heap pruning for columnar fields) is heavily optimized, slightly over-querying and occasionally re-querying a deeper window is typically much cheaper than evaluating expensive post-filters during collection or materializing the full relation.

This design introduces the same iterative strategy to DataFusion physical plans in `JoinScan` — and unifies `BaseScan`'s existing `TopKScanExecState` onto the same implementation — via two cooperating components:

1. **Lazy Sorted Mode on `PgSearchScanPlan`**: A mode on `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) whose stream yields progressively deeper sorted windows `[offset .. offset + window_size)` via `search_top_k_in_segments` (or `search_top_k_unordered_in_segments`) on each pull. Both `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) and `JoinScan`'s outer Top-K scan will use this mode, eliminating duplicate Top-K windowing, parallel segment claiming, and CTID/fast-field resolution code while giving the new mode full test coverage from existing `BaseScan` Top-K tests.
2. **`IterativeLookupJoinExec`**: A custom DataFusion `ExecutionPlan` join node for `JoinType::Inner` that pulls a sorted batch from its outer (left) child, pushes the batch's join keys into its inner (right) child via `DynamicFilterPhysicalExpr` (`InListExpr` $\rightarrow$ Tantivy `TermSetQuery`), re-executes the inner child for that batch, and emits matching joined rows while preserving the outer sort order.

---

## 2. Architecture & Data Flow

### 2.1 Operator Pipeline

For an eligible query `SELECT ... FROM a JOIN b ON a.k = b.k ORDER BY a.sort_col DESC LIMIT K`:

```text
GlobalLimitExec(fetch=K) / SortPreservingMergeExec(fetch=K)
  └── TantivyDecodeExec / TantivyFetchExec (deferred columns for surviving rows)
        └── VisibilityFilterExec (checks heap MVCC visibility for both a.ctid and b.ctid)
              └── IterativeLookupJoinExec (JoinType::Inner, preserves left ordering)
                    ├── Left (Outer): PgSearchScanPlan [LazySortMode: a.sort_col DESC]
                    └── Right (Inner): Inner Subplan (PgSearchScanPlan on b with DynamicFilterPhysicalExpr)
```

### 2.2 Why Deferred Visibility Works After the Join

`VisibilityFilterOptimizerRule` (`pg_search/src/postgres/customscan/joinscan/visibility_filter.rs`) treats `LogicalPlan::Join(Inner)` as `BarrierStatus::None` and `LogicalPlan::Sort(fetch=Some)` / `LogicalPlan::Limit` as `BarrierStatus::Full`. Consequently, `VisibilityFilterExec` is placed above the `Inner` join and below the top-level limit/merge.

Because DataFusion execution is pull-based:

- The top-level `GlobalLimitExec(fetch=K)` (or `SortPreservingMergeExec(fetch=K)` in MPP) pulls batches until `K` visible rows have been produced.
- If `IterativeLookupJoinExec` produces fewer than `K` joined rows from the first outer window, or if `VisibilityFilterExec` filters out dead tuples from either the outer or inner table, the limit node simply calls `poll_next()` again.
- That pull cascades through `VisibilityFilterExec` $\rightarrow$ `IterativeLookupJoinExec` $\rightarrow$ `PgSearchScanPlan` (outer), which advances `offset` and runs the next `search_top_k_in_segments` window.
- Keeping `VisibilityFilterExec` above `IterativeLookupJoinExec` avoids checking heap visibility on outer rows that fail to match the inner table.

---

## 3. Component Design

### 3.1 Lazy Sorted Mode on `PgSearchScanPlan` (Shared with `BaseScan` Top-K)

Rather than introducing a separate leaf `ExecutionPlan` type, lazy sorted execution is represented as an optional configuration on `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`). Reusing `PgSearchScanPlan` preserves existing integrations with:

- `ExecutionState::Shared` and `ExecutionState::RangePartitioned`
- `pg_search_scan_desired_task_count` and `pg_search_scan_scale_up_leaf_node` (`DistributedLeafExec`)
- `visit_scan_nodes`, `stamp_parallel_state`, and `CtidResolver` (`VisibilityCtidResolverRule`)
- `DeferredPlacementRule` (`pg_search/src/scan/deferred_placement_rule.rs`)
- `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`)
- `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`), which will drive `PgSearchScanPlan` via a synchronous Tokio stream poll (`poll_next_sync`, matching `ColumnarExecState` in `pg_search/src/postgres/customscan/basescan/exec_methods/fast_fields/columnar.rs`).

#### Configuration Struct

```rust
#[derive(Clone, Debug)]
pub struct LazySortConfig {
    /// Tantivy-compatible sort specification (Score, Field, or VectorDistance).
    /// `None` represents unordered Top-K (`search_top_k_unordered_in_segments` in BaseScan).
    pub order_by: Option<Vec<OrderByInfo>>,
    /// Physical ordering declared in `PlanProperties::eq_properties` (when ordered).
    pub physical_ordering: Option<LexOrdering>,
    /// Initial Top-K window size (`(limit * scale_factor) as usize` on the first pull).
    pub initial_window: usize,
    /// Multiplicative growth factor for subsequent pulls (`paradedb.topk_retry_scale_factor`).
    pub retry_scale_factor: usize,
    /// Maximum window size cap (`paradedb.max_topk_chunk_size`).
    pub max_chunk_size: usize,
    /// Whether to use the shared parallel threshold in `ParallelScanState` on the first pull.
    pub use_shared_threshold: bool,
}
```

#### Plan Properties

When `lazy_sort: Some(config)` is set on `PgSearchScanPlan`:

- If `config.physical_ordering` is `Some(ordering)`, `PlanProperties::eq_properties` registers `ordering` via `eq_props.add_ordering(...)`, so DataFusion's `EnforceSorting` and parent operators know the scan's output is already ordered.

#### Execution Stream (`execute`)

Inside `PgSearchScanPlan::execute(target_partition, context)`:

1. **Segment Selection**:
   - **`ExecutionState::Shared`**:
     - In serial mode (`parallel_state.is_null()`), query all candidate segments of the reader (`reader.segment_ids()`).
     - In parallel/MPP mode (`!parallel_state.is_null()`), claim segments lazily from `parallel_state` on the first pull (`checkout_segment_for_source`) and cache `claimed_segments: Vec<SegmentId>` in the stream state so subsequent re-queries with `offset > 0` search the exact same segment set (matching `TopKScanExecState::segments_to_query` in `pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`).
   - **`ExecutionState::RangePartitioned`**:
     - Query the partition's `PartitionSegments` (`included` and `partially_included`) constrained by `range_boundaries.partition_bounds(target_partition)`.
     - Extend `SearchIndexReader::search_top_k_in_segments` (or add `search_top_k_in_segments_with_range_filter` in `pg_search/src/index/reader/index.rs`) so that `included` segments evaluate the unconstrained base weight while `partially_included` segments evaluate `unconstrained_weight.and_query(range_query)`, matching `search_segments_with_range_filter`.
2. **Iterative Window Pulling**:
   - State tracks `offset: usize = 0`, `chunk_size: usize = 0`, `query_count: usize = 0`, and `exhausted: bool = false`.
   - On each `poll_next()` when `!exhausted`:
     - Compute `local_limit`:
       - On the first pull (`query_count == 0`): `local_limit = config.initial_window`.
       - On subsequent pulls (`query_count > 0`): update `chunk_size = (chunk_size * config.retry_scale_factor).max(config.initial_window * config.retry_scale_factor).min(config.max_chunk_size)`, and `local_limit = config.initial_window.max(chunk_size)`.
     - Increment `query_count += 1`.
     - Execute `reader.search_top_k_in_segments(...)` (when `config.order_by` is `Some`, passing `parallel_state` as `parallel_state_holding_shared_threshold` only when `query_count == 1 && config.use_shared_threshold`) or `reader.search_top_k_unordered_in_segments(...)` (when `config.order_by` is `None`).
     - If `top_k_search.results.len() < local_limit`, set `exhausted = true` (all matching docs in the segment set have been seen).
     - Advance `offset += local_limit`.
     - Materialize the returned `Vec<(SearchIndexScore, DocAddress)>` into a `RecordBatch` in exact Top-K result order, using `for_each_segment` (`pg_search/src/index/fast_fields_helper.rs`) to fetch fast fields (`Ctid`, `DeferredCtid`, `Named` fast fields) per segment in `DocId` order and scatter them back to their Top-K row indices.

#### Backing `BaseScan`'s `TopKScanExecState`

In `pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`:

- `TopKScanExecState` constructs a `PgSearchScanPlan` configured with `LazySortConfig` and `[WhichFastField::Ctid, WhichFastField::Score(...)]` (with `VisibilityChecker::new_noop()`, since `BaseScan::exec_custom_scan` performs heap tuple fetching, `check_visibility`, subplan qual evaluation, and snippet projection).
- Each time `TopKScanExecState::query()` is called (initially, or when `self.found < self.limit()` after heap visibility pruning), it pulls the next `RecordBatch` from the `PgSearchScanPlan` stream via `poll_next_sync`.
- Telemetry (`query_count`, `segment_info` for vector probes/IO stats) and optional first-query `TopKAuxiliaryCollector` (for `BaseScan` window aggregates) are surfaced from the scan stream back to `BaseScanState`.

---

### 3.2 `IterativeLookupJoinExec`

A new physical operator in `pg_search/src/postgres/customscan/joinscan/` (or `pg_search/src/scan/`) implementing `ExecutionPlan`.

#### Scope & Invariants

- **Supported Join Type**: `JoinType::Inner` only (initial implementation).
- **Children**:
  - `left` (outer): The order-driving input (either a `PgSearchScanPlan` in `LazySortMode` or another `IterativeLookupJoinExec` in a left-deep join chain).
  - `right` (inner): A re-entrant subplan rooted at a single `PgSearchScanPlan`, optionally wrapped in stateless unary nodes (`ProjectionExec`, `FilterExec`, `TantivyFetchExec`, `TantivyDecodeExec`).
- **Order Preservation**:
  - `maintains_input_order()` returns `[true, false]`.
  - `EquivalenceProperties` propagates the `left` child's output ordering into the join's `PlanProperties`.
- **Partitioning Modes**:

  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum IterativeJoinPartitionMode {
      /// Both left and right children are co-partitioned (e.g., compatible `Partitioning::Range`
      /// on the join keys). Partition `i` of the join pulls from `left.execute(i)` and probes
      /// `right.execute(i)`.
      Partitioned,
      /// Left child has N partitions (e.g., `ExecutionState::Shared` or range-partitioned on
      /// a non-join column) and right child is an unpartitioned/replicated local scan
      /// (`right.execute(0)` searches all candidate segments of the inner index).
      ReplicatedInner,
  }
  ```

  - `required_input_distribution()` returns `[UnspecifiedDistribution, UnspecifiedDistribution]` in both modes so DataFusion's `EnforceDistribution` never inserts a `RepartitionExec` / `NetworkShuffleExec` under the join.

#### Per-Batch Execution Loop

For a given `partition` in `IterativeLookupJoinExec::execute(partition, context)`:

1. Pull the next non-empty `outer_batch: RecordBatch` from `left_stream`. If `left_stream` yields `None`, terminate.
2. Evaluate the left equi-join key expressions on `outer_batch`.
   - Filter out any outer rows with `NULL` in any equi-join key (since `JoinType::Inner` cannot match `NULL` equi-keys). If no outer rows remain, loop to step 1.
3. Deduplicate the non-null join key values and construct an `InListExpr` for each equi-join key column on the right side (combining multiple keys with `BinaryExpr(..., Operator::And, ...)` for composite joins, matching `try_extract_in_list_conjunction` in `pg_search/src/scan/pre_filter.rs`).
4. Update `self.dynamic_filter.update(in_list_expr)`.
5. Determine the inner partition to execute:
   - `IterativeJoinPartitionMode::Partitioned` $\rightarrow$ `inner_partition = partition`
   - `IterativeJoinPartitionMode::ReplicatedInner` $\rightarrow$ `inner_partition = 0`
6. Call `self.right.execute(inner_partition, Arc::clone(&context))` and collect all returned inner `RecordBatch`es into a single concatenated batch (or an in-memory hash table keyed by the right join keys).
7. Probe the collected inner rows in **outer row order**:
   - Iterating outer row index `0..outer_batch.num_rows()` in order and emitting `(outer_row, matching_inner_rows)` pairs guarantees that the output `RecordBatch` strictly preserves the outer `LexOrdering`.
   - Apply any residual `JoinFilter` (`self.filter`) before emitting.
8. If the resulting batch is empty (0 matches for this outer window), loop immediately to step 1 to pull the next outer window; otherwise yield the joined `RecordBatch`.

---

### 3.3 Re-entrant Inner `PgSearchScanPlan` & `TermSetQuery` Pushdown

Two small adjustments in `pg_search/src/scan/` are required so `self.right.execute(...)` can be called once per outer batch:

1. **Repeatable `PgSearchScanPlan::execute()`**:
   - In `pg_search/src/scan/execution_plan.rs`, `execute()` currently transitions `self.state` to `ExecutionState::Consumed` via `std::mem::replace`.
   - Change `PgSearchScanPlan` (either unconditionally by cloning `ScanState` out of the mutex guard, or when marked `repeatable: bool`) so `self.state` remains intact across multiple `execute()` calls.
   - Because `execute()` already clones `reader` and `dynamic_filters` inside `stream_gen` before invoking `try_dynamic_filter_pushdown(&mut reader, &mut dynamic_filters, ...)`, each `execute()` call will read the newly updated `DynamicFilterPhysicalExpr` and compile a fresh `TermSetQuery` without mutating the base `ScanState.reader`.
   - When the inner scan is in `ReplicatedInner` mode under MPP, ensure its `parallel_state` is `None` (or ignored via `mpp_source_idx = None` / replicated flag) so every worker searches all candidate segments of the inner index rather than claiming from a shared once-only segment counter.

2. **Avoid `InListPushdown::Skip` Full-Scan Fallback on Inner Lookups**:
   - In `pg_search/src/scan/pre_filter.rs`, `try_convert_in_list_to_query` returns `InListPushdown::Skip` when `density > term_set_bitset_max_density_multi()` and rewrites the dynamic filter expression to `lit(true)`.
   - That heuristic assumes a single full scan below a `HashJoinExec` that will filter rows in its hash table. In `IterativeLookupJoinExec`, rewriting the filter to `lit(true)` on a small segment would cause every outer batch iteration to return all rows of that segment, and if no `PreFilter` is retained, could even return non-matching rows if the join were not re-checking equality.
   - Even though `IterativeLookupJoinExec` checks equality when matching outer and inner rows, we should ensure that for iterative inner scans (or in general when `Skip` triggers), `try_dynamic_filter_pushdown` either forces `TermSetQuery` construction or retains the `PreFilter` rather than rewriting the filter to `lit(true)` when skipping index pushdown.

---

## 4. Optimizer Integration & Cost Model

### 4.1 Physical Optimizer Rule (`IterativeLookupJoinRule`)

Introduce a `PhysicalOptimizerRule` behind a GUC (e.g., `paradedb.enable_iterative_lookup_join`, plus a threshold GUC or cost check):

#### Placement in Rule Pipeline (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`)

- Run **after** `RangeCoPartitionedJoinRule` and `EnsureRequirements` (`EnforceDistribution` / `EnforceSorting`), and **before** `DeferredPlacementRule`, `SegmentedTopKRule`, and `DistributedPhysicalOptimizerRule`.
- Running after `RangeCoPartitionedJoinRule` lets `IterativeLookupJoinRule` inspect whether `HashJoinExec` is already in `PartitionMode::Partitioned` (range co-partitioned on the join keys) or `PartitionMode::CollectLeft`.
- Running before `DeferredPlacementRule` means that once `SortExec(fetch=K)` is replaced by a streaming limit (`GlobalLimitExec(fetch=K)` or `SortPreservingMergeExec(fetch=K)`), `DeferredPlacementRule` automatically sees `Bound::Limit` (`Consumer::Bounds`) above `TantivyDecodeExec`, keeping string dictionary decoding deferred until after the join and visibility check (so only the final `K` rows are decoded).
- In `DeferredPlacementRule` (`pg_search/src/scan/deferred_placement_rule.rs`), teach `summarize_path`, `rebuilds_with_new_types`, and `is_unique` about `IterativeLookupJoinExec` (mirroring `HashJoinExec`, except that on the left side `out_of_order` is `true` because the outer scan emits in Top-K sort order rather than `DocAddress` order).

#### Pattern Matching & Eligibility Criteria

`IterativeLookupJoinRule` matches a `SortExec(fetch=Some(k))` (where `k == limit + offset`) and walks down through order-preserving / limit-transparent unary nodes (`TantivyDecodeExec`, `TantivyFetchExec`, `VisibilityFilterExec`, `ProjectionExec`, `CoalescePartitionsExec`) to a `HashJoinExec` (or a left-deep chain of `HashJoinExec`s):

1. **Join Type**: `HashJoinExec::join_type() == JoinType::Inner`.
2. **Sort Order on One Preserved Side**:
   - All expressions in `SortExec::expr()` resolve to columns coming from a single base `PgSearchScanPlan` (call it the **outer side**; if the `HashJoinExec` has that side on the right, swap `left` and `right` when constructing `IterativeLookupJoinExec` since `JoinType::Inner` is symmetric up to output column projection).
   - Every sort expression can be translated back to a Tantivy-supported `OrderByInfo` (`OrderByFeature::Score`, `OrderByFeature::Field` on a sortable fast field, or `OrderByFeature::VectorDistance`) up to `MAX_TOPK_FEATURES`.
3. **Inner Child Shape**:
   - The other child of `HashJoinExec` (the **inner side**) consists of a single `PgSearchScanPlan` with only stateless unary operators (`ProjectionExec`, `FilterExec`, `TantivyFetchExec`, `TantivyDecodeExec`) between the join and the scan, and no `RepartitionExec` that hashes/shuffles.
   - All right-hand equi-join keys resolve to fast fields / indexed term fields on the inner `PgSearchScanPlan` that support `InListExpr` $\rightarrow$ `TermSetQuery` pushdown.
4. **Window Sizing & Cost/Cardinality Gate**:
   - Estimate join selectivity $s_{\text{join}} \in (0, 1]$ and visibility selectivity $s_{\text{vis}} \in (0, 1]$ (or use a configurable initial multiplier / minimum chunk size, similar to `TopKScanExecState` in `basescan/exec_methods/top_k.rs` where `initial_window = clamp(k / selectivity, K, max_initial_window)`).
   - Gate the rewrite on `k` being small relative to the estimated outer/inner input cardinalities (e.g., when `k` is below a threshold such as `paradedb.iterative_lookup_join_max_k` or when estimated `initial_window` is a small fraction of the inner table's row count so a handful of `TermSetQuery` point lookups beats scanning and hashing the full inner table).

#### Plan Transformation

When eligible:

1. Convert the outer `PgSearchScanPlan` into `LazySortMode` with `LazySortConfig { order_by, physical_ordering, initial_window, scale_factor }`.
2. Mint a `DynamicFilterPhysicalExpr` (initialized to `lit(true)`) over the right join key columns and push it into the inner `PgSearchScanPlan`'s `dynamic_filters`.
3. Configure the inner `PgSearchScanPlan` as repeatable:
   - If the original `HashJoinExec` was `PartitionMode::Partitioned` with compatible `Partitioning::Range` on both inputs, set `IterativeJoinPartitionMode::Partitioned` and keep `ExecutionState::RangePartitioned` on both scans.
   - Otherwise, set `IterativeJoinPartitionMode::ReplicatedInner` and ensure the inner `PgSearchScanPlan` is unpartitioned (`repartition(1)` / `mpp_source_idx = None`) so each task probes all candidate segments of the inner index locally.
   - Strip any `RepartitionExec` / `CoalescePartitionsExec` that `EnforceDistribution` inserted solely for a `CollectLeft` or hash-partitioned `HashJoinExec`.
4. Replace `HashJoinExec` with `IterativeLookupJoinExec`.
5. Replace the top `SortExec(fetch=Some(k))` with:
   - `LocalLimitExec::new(input, k)` if the input has 1 partition, or
   - `SortPreservingMergeExec::new(sort_exprs, input).with_fetch(Some(k))` if the input has $> 1$ partitions (or precedes a `CoalescePartitionsExec`, replacing `SortExec(fetch=k) -> CoalescePartitionsExec(input)` with `SortPreservingMergeExec(fetch=k, input)`).

---

## 5. MPP Serialization & Execution (`datafusion-distributed`)

### 5.1 Single-Stage Worker Execution (Zero Network Shuffles)

In both partitioning modes (`Partitioned` and `ReplicatedInner`), `IterativeLookupJoinExec` and its entire inner child run inside the **same worker stage** as the outer `PgSearchScanPlan`:

- **Leader stage**: Runs `SortPreservingMergeExec(fetch=K)` over a `NetworkGatherExec` (or `ProjectionExec` above it).
- **Worker stage (1 stage, $P$ tasks)**: Each worker task `i` runs:

  ```text
  LocalLimitExec(fetch=K) (optional per-task bound)
    └── VisibilityFilterExec
          └── IterativeLookupJoinExec (partition i)
                ├── Left: DistributedLeafExec(PgSearchScanPlan [LazySortMode, partition i])
                └── Right: PgSearchScanPlan [partition i (Range) or partition 0 (ReplicatedInner)]
  ```

- Because `PgSearchScanPlan` is a leaf node on both sides, `pg_search_scan_scale_up_leaf_node` (`pg_search/src/scan/execution_plan.rs`) must know not to wrap the _replicated inner_ `PgSearchScanPlan` in a `DistributedLeafExec` that partitions its segments across tasks when it is in `ReplicatedInner` mode (its `partition_count()` is `1`, whereas in `Partitioned` range mode its `partition_count()` matches `ev.task_count` so scaling up assigns partition `i` to task `i` as usual).

### 5.2 Protobuf Dispatch Codec (`PgSearchPhysicalExtensionCodec`)

In `pg_search/src/scan/physical_codec.rs`:

1. Extend `PgSearchScanPlan`'s dispatch payload to serialize `Option<LazySortConfig>` and `repeatable: bool`.
2. Add a new tag `TAG_ITERATIVE_LOOKUP_JOIN` for `IterativeLookupJoinExec`:
   - Serialize `on: Vec<(PhysicalExprRef, PhysicalExprRef)>`, `filter: Option<JoinFilter>`, `projection: Option<Vec<usize>>`, `partition_mode: IterativeJoinPartitionMode`, and `dynamic_filter: Arc<dyn PhysicalExpr>` using `proto_converter`.
   - Because `StagePlanDispatchSource` (`pg_search/src/postgres/customscan/mpp/glue.rs`) encodes with `DefaultPhysicalProtoConverter` (which stamps `expression_id` on every `DynamicFilterPhysicalExpr`) and workers decode with `DeduplicatingProtoConverter` (`pg_search/src/postgres/customscan/mpp/dispatch.rs`), the decoded `IterativeLookupJoinExec` and inner `PgSearchScanPlan` on each worker automatically share the exact same `Arc<DynamicFilterPhysicalExpr>` instance.

---

## 6. Implementation Plan (Phased)

1. **Phase 1: Lazy Sorted Mode on `PgSearchScanPlan`, `BaseScan` Top-K Unification & Range-Filtered Top-K**
   - Add `LazySortConfig` and repeatable execution support to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`).
   - Refactor `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) to execute via `PgSearchScanPlan` in lazy-sorted mode, validating the implementation against the existing `BaseScan` Top-K test suite.
   - Add range-filter support to `SearchIndexReader::search_top_k_in_segments` (`pg_search/src/index/reader/index.rs`) for `ExecutionState::RangePartitioned`.
   - Ensure `pre_filter.rs` does not drop the filter without a `PreFilter` fallback when `InListPushdown::Skip` triggers on tiny segments.

2. **Phase 2: `IterativeLookupJoinExec` Operator**
   - Implement `IterativeLookupJoinExec` supporting `JoinType::Inner`, `IterativeJoinPartitionMode::{Partitioned, ReplicatedInner}`, outer-order-preserving probing, residual `JoinFilter` evaluation, and execution metrics (outer windows pulled, inner lookups executed, rows matched).

3. **Phase 3: `IterativeLookupJoinRule`, GUCs & Late Materialization Integration**
   - Add GUCs (`paradedb.enable_iterative_lookup_join`, window sizing / threshold knobs) in `pg_search/src/gucs.rs`.
   - Implement `IterativeLookupJoinRule` and register it in `build_base_session` (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`).
   - Update `DeferredPlacementRule` (`pg_search/src/scan/deferred_placement_rule.rs`) to recognize `IterativeLookupJoinExec`.

4. **Phase 4: MPP Support & Verification**
   - Add `LazySortConfig` and `TAG_ITERATIVE_LOOKUP_JOIN` serialization in `pg_search/src/scan/physical_codec.rs`.
   - Update `pg_search_scan_desired_task_count` / `pg_search_scan_scale_up_leaf_node` in `pg_search/src/scan/execution_plan.rs` for `ReplicatedInner` scans.
   - Add unit and `pg_regress` tests covering serial execution, MVCC dead-tuple re-querying, deferred string materialization, range co-partitioned MPP joins, and replicated-inner MPP joins.
