# Lazy Sorted Scan Mode Design (`PgSearchScanPlan`)

## 1. Motivation & Overview

Three execution paths in `pg_search` need to consume rows from a Tantivy index in sorted order (`ORDER BY pdb.score(id) DESC`, `ORDER BY fast_field ASC/DESC`, or `ORDER BY embedding <=> '[...]'`) with a `LIMIT K`, where downstream filtering or join probing may discard some returned rows:

1. `BaseScan` Top-K (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`): Pulls a sorted window from Tantivy, checks PostgreSQL heap MVCC visibility and subplan quals, and re-queries a deeper window if fewer than `K` tuples survive.
2. `IterativeLookupJoinExec` (`iterative-joins/ITERATIVE_TOPK_JOIN_DESIGN.md`): Pulls sorted batches from an outer table and probes an inner table via repeatable `TermSetQuery` lookups until `K` joined and MVCC-visible rows are produced.
3. `RankJoinExec` (`iterative-joins/RANK_JOIN_DESIGN.md`): Consumes two sorted input streams in score-guided order under a monotonic scoring function $f(s_L, s_R)$ until the top `K` joined results clear the stopping threshold $T$.

Because Tantivy's Top-K collectors (Block-Max WAND for BM25 scores, fast-field heap pruning for columnar fields, and HNSW/exact vector collectors) are heavily optimized for bounded `limit + offset` queries, iteratively requesting progressively larger windows `[offset .. offset + local_limit)` on demand is much cheaper than materializing and sorting an entire table up front.

This document specifies _Lazy Sorted Mode_ (`LazySortConfig`) on `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) as the shared physical scan primitive used by all three paths.

---

## 2. Reusing `PgSearchScanPlan` & Existing `ExecutionState` Modes

Lazy sorted execution is represented as an optional `LazySortConfig` attached to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) rather than a separate `ExecutionPlan` node or a new scan output mode.

`PgSearchScanPlan` keeps only its two existing `ExecutionState` modes:

1. `ExecutionState::Shared` (unpartitioned / work-stealing across tasks via `ParallelScanState`).
2. `ExecutionState::RangePartitioned` (range-partitioned across tasks via `RangePartitioningRule` split points).

<details>
<summary>Existing infrastructure preserved by reusing PgSearchScanPlan</summary>

Reusing `PgSearchScanPlan` and its two existing execution modes preserves all current integrations with:

- `pg_search_scan_desired_task_count` and `pg_search_scan_scale_up_leaf_node` (`DistributedLeafExec`)
- `visit_scan_nodes`, `stamp_parallel_state`, and `CtidResolver` (`VisibilityCtidResolverRule`)
- `DeferredPlacementRule` (`pg_search/src/scan/deferred_placement_rule.rs`)
- `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`)
- `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`), which will drive `PgSearchScanPlan` via a synchronous Tokio stream poll (`poll_next_sync`, matching `ColumnarExecState` in `pg_search/src/postgres/customscan/basescan/exec_methods/fast_fields/columnar.rs`), giving the shared implementation immediate test coverage from the existing `BaseScan` Top-K test suite.

</details>

---

## 3. Component Design

### 3.1 Configuration (`LazySortConfig`) & Plan Properties

`LazySortConfig` (implementing `Clone` and `Debug`) encapsulates the parameters needed by `PgSearchScanPlan::with_lazy_sort(self, config: LazySortConfig) -> Self`:

- The Tantivy sort specification (`Option<Vec<OrderByInfo>>`, where `None` represents unordered Top-K via `search_top_k_unordered_in_segments` in `BaseScan`).
- The physical output ordering (`Option<LexOrdering>`) registered on `PlanProperties::eq_properties` via `EquivalenceProperties::add_ordering`, so DataFusion's `EnforceSorting` and parent operators (`SortPreservingMergeExec`, `IterativeLookupJoinExec`, `RankJoinExec`) know each partition's output is already sorted.
- Window sizing parameters: `initial_window`, multiplicative retry growth factor (`paradedb.topk_retry_scale_factor`), and maximum window size cap (`paradedb.max_topk_chunk_size`).
- Whether to consult the shared parallel threshold in `ParallelScanState` on the first pull.

### 3.2 Execution Stream (`PgSearchScanPlan::execute`)

Inside `PgSearchScanPlan::execute(target_partition, context) -> Result<SendableRecordBatchStream>`:

1. _Segment Selection_:
   - `ExecutionState::Shared`: Queries all candidate segments in serial mode, or claims segments lazily via `ParallelScanState::checkout_segment_for_source` on the first pull in parallel/MPP mode and caches the claimed `Vec<SegmentId>` so subsequent window re-queries (`offset > 0`) search the exact same segment set.
   - `ExecutionState::RangePartitioned`: Queries the partition's `PartitionSegments` (`included` and `partially_included`) constrained by `RangeBoundaries::partition_bounds(target_partition)`, evaluating the unconstrained weight on `included` segments and `unconstrained_weight.and_query(range_query)` on `partially_included` segments.

2. _Iterative Window Pulling_:
   - On the first `Stream::poll_next`, requests `local_limit = initial_window` rows at `offset = 0` via `SearchIndexReader::search_top_k_in_segments` (or `search_top_k_unordered_in_segments`).
   - On subsequent pulls, advances `offset` by the previous `local_limit` and scales `local_limit` by `retry_scale_factor` (capped at `max_chunk_size`).
   - Marks the stream exhausted as soon as Tantivy returns fewer than `local_limit` documents.
   - Materializes the returned `Vec<(SearchIndexScore, DocAddress)>` into a `RecordBatch` in exact Top-K result order, using `for_each_segment` (`pg_search/src/index/fast_fields_helper.rs`) to fetch fast fields per segment in `DocId` order and scatter them back to their Top-K row indices.

### 3.3 Backing `BaseScan`'s `TopKScanExecState`

In `pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`, `TopKScanExecState` constructs a `PgSearchScanPlan` configured with `LazySortConfig` and `[WhichFastField::Ctid, WhichFastField::Score(...)]` (with `VisibilityChecker::new_noop()`), pulling the next `RecordBatch` via `poll_next_sync` whenever `found < limit` after heap visibility pruning.

---

## 4. Tantivy `max_score` & Dynamic Threshold Pushdown

Lazy Sorted Mode leverages Tantivy's Max Score API (`Weight::index_max_score`, `Weight::max_score(segment_reader, boost)`, and future `PruningScorer::max_score`):

1. _Per-Segment Skipping_: Before executing a Top-K collector on a segment, `SearchIndexReader::search_top_k_in_segments` checks `Weight::max_score(segment_reader, 1.0)` (for `OrderByFeature::Score`) or fast-field `Column::min_value` / `Column::max_value` (for `OrderByFeature::Field`). Segments that cannot beat the current Top-K threshold or have `max_score == 0.0` are skipped in $O(1)$.
2. _Plan-Time & Initial Upper Bounds_: `Query::index_max_score(&searcher)` provides an $O(1)$ upper bound on an input stream's top score at plan time (`iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`) and before documents are fetched.
3. _Dynamic Threshold Pushdown_: Within a single window pull, Tantivy's `TopDocs` collector uses its window-local $(offset + local\_limit)$-th threshold for Block-Max WAND pruning. Across windows or join inputs, parent operators can publish a lower-bound score threshold $\theta_i$ only from _verified survivors_ that have passed all downstream filters.

<details>
<summary>Why cross-window pruning thresholds require verified survivors & future work on filter placement</summary>

- _Within a single window pull_: Tantivy's `TopDocs` collector prunes blocks against the $(offset + local\_limit)$-th best score in that window. If downstream filters (`VisibilityFilterExec` or a join) reject rows from that window, the next pull re-queries with a larger `offset + local_limit`, so no valid document is permanently lost.
- _Across windows or across join inputs_: Whenever selective operators sit _above_ the producer of a threshold $\theta_i$ — such as `VisibilityFilterExec` (checking heap MVCC visibility), a residual `FilterExec`, or an upstream `IterativeLookupJoinExec` — a hard cutoff $\theta_i$ that permanently skips documents or terminates a lazy-sorted stream early _must only be derived from $K$ rows that have survived all downstream filters_. Using an unverified $K$-th candidate before visibility/join filtering could prune the $(K+1)$-th candidate needed if one of the top $K$ is later filtered out.
- _Future Work (Adaptive Filter & Visibility Placement)_: Moving `VisibilityFilterExec` (or cheap selective filters) _below_ `RankJoinExec` / `IterativeLookupJoinExec` on specific inputs makes every candidate entering the join already verified with respect to that input's visibility. That unlocks immediate cross-join Block-Max WAND threshold pushdown from the join's internal candidate heap, at the cost of checking heap visibility on rows that may not match the join. Evaluating adaptive placement of `VisibilityFilterExec` (above vs. below the join) is tracked as follow-up work.

</details>

---

## 5. MPP Serialization (`PgSearchPhysicalExtensionCodec`)

In `pg_search/src/scan/physical_codec.rs`, `PgSearchScanPlan`'s protobuf dispatch payload in `PgSearchPhysicalExtensionCodec` (`PhysicalExtensionCodec::try_encode` / `try_decode`) serializes `Option<LazySortConfig>` so workers decode and execute `LazySortMode` under either `ExecutionState::Shared` or `ExecutionState::RangePartitioned`.
