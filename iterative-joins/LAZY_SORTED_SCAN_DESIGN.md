# Lazy Sorted & Point-Lookup Scan Mode Design (`PgSearchScanPlan`)

## 1. Motivation & Overview

Ranked and Top-$K$ query execution in `pg_search` requires two complementary access primitives on a Tantivy index (`Fagin, Lotem, & Naor, PODS 2001 / JCSS 2003`):

1. _Sorted Access (`SA` — Iterative Top-$K$ Windowing)_:
   Stream matching documents in descending score order (`ORDER BY pdb.score(id) DESC`) or fast-field / vector distance order (`ORDER BY fast_field ASC/DESC`, `ORDER BY embedding <=> '[...]'`) using progressively deeper Top-$K$ windows `[offset .. offset + local_limit)`.
   - Used by `BaseScan` Top-$K$ (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`) and `RankJoinExec` (`iterative-joins/RANK_JOIN_DESIGN.md`).
   - Allows Tantivy's `TopDocs` collector (Block-Max WAND for BM25 scores, fast-field heap pruning for columnar fields, and HNSW/exact vector collectors) to skip non-competitive blocks without materializing the full postings list.

2. _Random Access (`RA` — On-Demand Batched Point Lookup)_:
   Fetch specific documents by join key (`col IN (k_1, ..., k_m)` $\rightarrow$ Tantivy `TermSetQuery`) when requested by a parent `RankJoinExec` via a pushed-down `DynamicFilterPhysicalExpr` (`RankLookupPhysicalExpr`), and evaluate the scan's scoring predicate (if any) on those documents.
   - Used by `RankJoinExec` (`iterative-joins/RANK_JOIN_DESIGN.md`) whenever an unmatched high-scoring tuple from the opposite join input survives to the emission threshold (`CA` / `MPro` lazy probing):
     - In _conjunctive_ joins (`ScanMode::Standard`), `RA` resolves whether the partner exists with a lower positive score outside the current `SA` window or is an orphan (allowing the stopping threshold $T$ to drop to $f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$).
     - In _disjunctive_ joins (`ScanMode::Tagged`), `RA` fetches the partner even if it did not match this scan's BM25 predicate (treating non-matching rows as $0.0$-scored documents).
     - In _single-sided_ ordered joins (`NRJN`, `iterative-joins/RANK_JOIN_DESIGN.md` §3.3), the unscored inner child disables `SA` and executes _only_ `RA` point-lookup batches driven by the outer sorted input.

This document specifies how `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) implements both `SA` and `RA` within a single stream while preserving its existing `ExecutionState` modes.

---

## 2. Reusing `PgSearchScanPlan` & Existing `ExecutionState` Modes

Both `SA` (`LazySortConfig`) and `RA` (`RankLookupPhysicalExpr` in `dynamic_filters`) are attached directly to `PgSearchScanPlan` (`pg_search/src/scan/execution_plan.rs`) rather than introducing separate scan operators.

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
- `BaseScan`'s `TopKScanExecState` (`pg_search/src/postgres/customscan/basescan/exec_methods/top_k.rs`), which drives `PgSearchScanPlan` via a synchronous Tokio stream poll (`poll_next_sync`, matching `ColumnarExecState` in `pg_search/src/postgres/customscan/basescan/exec_methods/fast_fields/columnar.rs`), giving the shared implementation immediate test coverage from the existing `BaseScan` Top-K test suite.

</details>

---

## 3. Component Design

### 3.1 Configuration (`LazySortConfig`) & Plan Properties

`LazySortConfig` (implementing `Clone` and `Debug`) encapsulates the parameters needed by `PgSearchScanPlan::with_lazy_sort(self, config: LazySortConfig) -> Self`:

- `orderby_info: Option<Vec<OrderByInfo>>`: The Tantivy sort specification (`None` represents unordered Top-$K$ via `search_top_k_unordered_in_segments` in `BaseScan`, or `RA`-only point-lookup mode when `initial_window == 0.0`).
- `ordering: Option<LexOrdering>`: The physical output ordering registered on `PlanProperties::eq_properties` via `EquivalenceProperties::add_ordering`, so DataFusion's `EnforceSorting` and parent operators (`SortPreservingMergeExec`, `RankJoinExec`) know each partition's `SA` output is sorted.
- `initial_window: f64`: Initial `SA` window size (`0.0` when the scan operates in `RA`-only point-lookup mode for an unscored join input).
- `shared_threshold: SharedThresholdMode`: Whether to consult the shared parallel threshold in `ParallelScanState` on the first pull.

### 3.2 Point-Lookup Dynamic Filter (`RankLookupPhysicalExpr`)

When `RankJoinExec` (`iterative-joins/RANK_JOIN_DESIGN.md`) can resolve unmatched keys against a child via Random Access, `RankJoinExec::gather_filters_for_pushdown` pushes a `DynamicFilterPhysicalExpr` to that child. Because it is wrapped in DataFusion's standard `DynamicFilterPhysicalExpr`, DataFusion's `FilterPushdown` pass automatically propagates it through intermediate unary operators (`ProjectionExec`, `CooperativeExec`) and remaps the join-key `Column` indices to match `PgSearchScanPlan`'s schema.

Inside `DynamicFilterPhysicalExpr`, `RankJoinStream` publishes updates using `RankLookupPhysicalExpr`:

```rust
pub struct RankLookupPhysicalExpr {
    /// Join-key column expression(s) in the target child's schema.
    /// Automatically remapped by `DynamicFilterPhysicalExpr::with_new_children`.
    cols: Vec<Arc<dyn PhysicalExpr>>,
    /// Batch of composite or single-column join key literals to resolve via `TermSetQuery`.
    keys: Vec<Vec<ScalarValue>>,
}
```

### 3.3 Unified `SA + RA` Execution Stream (`PgSearchScanPlan::execute`)

Inside `PgSearchScanPlan::execute(target_partition, context) -> Result<SendableRecordBatchStream>`, a single stream instance services an arbitrary interleaved sequence of `SA` window pulls and `RA` point-lookup requests without re-initializing `Scanner`, `FFHelper`, or `VisibilityChecker`:

1. _Document Deduplication (`emitted_docs: HashSet<DocAddress>`)_:
   - When `lazy_sort` is active alongside a `RankLookupPhysicalExpr`, a document may be fetched early by an `RA` point lookup and later encountered when the `SA` window expands (or vice versa).
   - The stream maintains `emitted_docs: HashSet<DocAddress>`. Both `SA` window pulls and `RA` point-lookup pulls skip any `DocAddress` already present in `emitted_docs` and insert every newly emitted `DocAddress`. Every document in the partition is guaranteed to be emitted at most once.

2. _Servicing Pending `RA` Point Lookups_:
   - At the top of each stream loop iteration, the stream inspects its pushed-down `DynamicFilterPhysicalExpr`s for a `RankLookupPhysicalExpr`.
   - When present:
     - Resets the `DynamicFilterPhysicalExpr` back to `lit(true)` (or to a sentinel on `SA` exhaustion) and converts `RankLookupPhysicalExpr` (`cols IN (keys)`) into a `ConstScoreQuery(TermSetQuery, 0.0)`.
     - Intersects the lookup query with the scan's required base filter via `SearchIndexReader::search_ra_lookup`:
       - In `ScanMode::Standard`: `resolved_query AND TermSetQuery(cols IN keys)` (only documents matching both the table's predicate and the join key are returned, scored by `resolved_query`).
       - In `ScanMode::Tagged`: `base_query AND TermSetQuery(cols IN keys)`. `Scanner::populate_scores_for_batch` evaluates `active_tag_scorers` on the matched `DocId`s, setting `__tag_i = true` and accumulating the tag's BM25 score onto `0.0` if the document matched the tag query, or setting `__tag_i = false` and keeping score `0.0` if it did not.
     - Filters out any `DocAddress` already in `emitted_docs`, records newly matched `DocAddress`es, materializes the batch, and yields it (or yields an empty 0-row `RecordBatch` if 0 new documents matched, signaling immediate completion to `RankJoinStream`).
     - Does _not_ advance the `SA` window `offset`.

3. _Servicing `SA` Iterative Window Pulls_:
   - When no `RA` request is pending:
     - If `SA` is disabled (`initial_window == 0.0`, i.e. `RA`-only mode) or `sa_exhausted` is already `true`, updates the direct parent `DynamicFilterPhysicalExpr` (`dynamic_filters.first()`) to `lit(false)` when kept alive for future `RA` requests (or `ScalarValue::Boolean(None)` when neither `SA` nor `RA` remains) and yields an empty 0-row `RecordBatch` (or terminates with `None` when not kept alive for `RA`).
     - Executes `search_top_k_in_segments` (or `search_top_k_segments_with_range_filter`) for `window = [offset .. offset + local_limit)`:
       - In `ScanMode::Standard`, searches `resolved_query`.
       - In `ScanMode::Tagged`, searches the positive-score tag query (`SearchIndexReader::with_tagged_sa_queries`), so `SA` only visits documents with positive tag scores in descending score order; non-matching documents (`score = 0.0`) are never scanned by `SA` and are only fetched on demand if requested by `RA`.
     - Filters out any `DocAddress` already in `emitted_docs`, records newly emitted `DocAddress`es, materializes the window in Top-$K$ order, and yields the `RecordBatch`.

---

## 4. Tantivy `max_score` & Dynamic Threshold Pushdown

Lazy Sorted Mode leverages Tantivy's Max Score API (`Weight::index_max_score`, `Weight::max_score(segment_reader, boost)`, and `PruningScorer::max_score`):

1. _Per-Segment Skipping_: Before executing a Top-$K$ collector on a segment, `SearchIndexReader::search_top_k_in_segments` checks `Weight::max_score(segment_reader, 1.0)` (for `OrderByFeature::Score`) or fast-field `Column::min_value` / `Column::max_value` (for `OrderByFeature::Field`). Segments that cannot beat the current Top-$K$ threshold or have `max_score == 0.0` are skipped in $O(1)$.
2. _Plan-Time & Initial Upper Bounds_: `Query::index_max_score(&searcher)` provides an $O(1)$ upper bound on an input stream's top score at plan time (`iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`).
3. _Dynamic Threshold Pushdown_: Within a single window pull, Tantivy's `TopDocs` collector uses its window-local $(offset + local\_limit)$-th threshold for Block-Max WAND pruning. Across windows or join inputs, parent operators publish a lower-bound score threshold $\theta_i$ only from _verified survivors_ that have passed all downstream filters.

---

## 5. MPP Serialization (`PgSearchPhysicalExtensionCodec`)

In `pg_search/src/scan/physical_codec.rs`, `PgSearchScanPlan`'s protobuf dispatch payload in `PgSearchPhysicalExtensionCodec` (`PhysicalExtensionCodec::try_encode` / `try_decode`) serializes `Option<LazySortConfig>`, and `DeduplicatingProtoConverter` preserves shared `Arc<DynamicFilterPhysicalExpr>` identity between `RankJoinExec` and `PgSearchScanPlan` within each worker task.
