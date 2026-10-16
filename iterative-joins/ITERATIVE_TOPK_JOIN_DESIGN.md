# Iterative Top-K Lookup Join Design (`IterativeLookupJoinExec`)

## 1. Motivation & Overview

In `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`), DataFusion's `HashJoinExec` and `NestedLoopJoinExec` require fully buffering the build side of the join before probing. When a join query has a `LIMIT K` and an `ORDER BY` that depends only on one side of the join (e.g., `ORDER BY pdb.score(a.id) DESC`, `ORDER BY a.fast_field ASC`, `ORDER BY a.embedding <=> '[...]'`, or a multi-table score already combined by a child `RankJoinExec`), materializing an entire unscored join input is often unnecessary work.

`IterativeLookupJoinExec` executes these queries lazily:

1. The _outer (left)_ child produces batches in the target sort order (`LazySortMode` on `PgSearchScanPlan`, defined in `iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`, or a child `RankJoinExec` defined in `iterative-joins/RANK_JOIN_DESIGN.md`).
2. For each sorted outer batch, `IterativeLookupJoinExec` pushes the batch's join keys into its _inner (right)_ child via `DynamicFilterPhysicalExpr` (`InListExpr` $\rightarrow$ Tantivy `TermSetQuery`), re-executes the inner child for that batch, and emits matching joined rows while strictly preserving the outer sort order.
3. If fewer than `K` joined rows survive downstream MVCC visibility checking (`VisibilityFilterExec`), the next pull automatically requests the next sorted window from the outer child.

<details>
<summary>Research background (Batched NRJN)</summary>

In the rank-aware query processing literature (Ilyas, Aref, & Elmagarmid, VLDB 2003 _"Supporting Top-k Join Queries in Relational Databases"_ and SIGMOD 2004 _"Rank-aware Query Optimization"_), this operator is known as _NRJN (Nested-Loops Rank Join)_. Rather than probing the inner index one tuple at a time, `IterativeLookupJoinExec` adapts NRJN to Tantivy's inverted and columnar indexes by probing the inner relation one outer `RecordBatch` window at a time via `TermSetQuery`.

</details>

---

## 2. Architecture & Data Flow

### 2.1 Operator Pipeline

For an eligible query `SELECT ... FROM a JOIN b ON a.k = b.k ORDER BY a.sort_col DESC LIMIT K`, the physical plan pipeline is:

- Top-level limit/merge: `GlobalLimitExec(fetch=K)` (serial) or `SortPreservingMergeExec(fetch=K)` (MPP)
- Late materialization: `TantivyDecodeExec` / `TantivyFetchExec` (resolving deferred columns for surviving rows)
- MVCC check: `VisibilityFilterExec` (checking heap visibility for both `a.ctid` and `b.ctid`)
- Join operator: `IterativeLookupJoinExec` (`JoinType::Inner`, preserving left ordering)
  - Left (outer) child: `PgSearchScanPlan` in `LazySortMode` (or `RankJoinExec`)
  - Right (inner) child: inner subplan rooted at `PgSearchScanPlan` on `b` receiving a `DynamicFilterPhysicalExpr`

### 2.2 Deferred Visibility After the Join

Because DataFusion execution is pull-based, keeping `VisibilityFilterExec` above `IterativeLookupJoinExec` and below `GlobalLimitExec(fetch=K)` works naturally:

- If `IterativeLookupJoinExec` produces fewer than `K` joined rows from the first outer window, or if `VisibilityFilterExec` filters out dead tuples from either table, the limit node simply calls `Stream::poll_next` again.
- That pull cascades through `VisibilityFilterExec` $\rightarrow$ `IterativeLookupJoinExec` $\rightarrow$ the outer sorted stream, advancing to the next window without ever checking heap visibility on outer rows that failed to match the inner table.

<details>
<summary>Future work on adaptive VisibilityFilterExec & filter placement</summary>

When the outer child is a `RankJoinExec` (`iterative-joins/RANK_JOIN_DESIGN.md`) or uses dynamic score-threshold pushdown (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` §4), any candidate that has not yet passed `IterativeLookupJoinExec` and `VisibilityFilterExec` is _unverified_ (it may be rejected by the inner lookup or MVCC check). We plan to experiment with adaptive placement of `VisibilityFilterExec` and selective predicates (e.g., pushing visibility checks below the join when join hit rate is high or dead-tuple fraction is significant, vs. keeping them above the join when the join itself is selective).

</details>

---

## 3. Component Design

### 3.1 `IterativeLookupJoinExec`

`IterativeLookupJoinExec` is a physical operator in `pg_search/src/postgres/customscan/joinscan/` (or `pg_search/src/scan/`) implementing DataFusion's `ExecutionPlan` and `DisplayAs` traits:

- _Supported Join Type_: `JoinType::Inner` only (initial implementation).
- _Children_:
  - `left` (outer): The order-driving input — a `PgSearchScanPlan` in `LazySortMode` (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`), a `RankJoinExec` (`iterative-joins/RANK_JOIN_DESIGN.md`), or another `IterativeLookupJoinExec`.
  - `right` (inner): A repeatable subplan rooted at a single `PgSearchScanPlan`, optionally wrapped in stateless unary nodes (`ProjectionExec`, `FilterExec`, `TantivyFetchExec`, `TantivyDecodeExec`).
- _Order Preservation_: `ExecutionPlan::maintains_input_order` returns `[true, false]`, and `ExecutionPlan::properties` propagates the `left` child's `LexOrdering` into `PlanProperties::eq_properties`.
- _Partitioning Modes (`IterativeJoinPartitionMode`)_:
  - `IterativeJoinPartitionMode::Partitioned`: Both `left` and `right` are range co-partitioned on the join keys (`Partitioning::Range`). Partition `i` pulls from `left.execute(i, ctx)` and probes `right.execute(i, ctx)`.
  - `IterativeJoinPartitionMode::ReplicatedInner`: `left` has $N$ partitions and `right` is an unpartitioned local scan where `right.execute(0, ctx)` searches all candidate segments of the inner index.
  - `ExecutionPlan::required_input_distribution` returns `[Distribution::UnspecifiedDistribution, Distribution::UnspecifiedDistribution]` in both modes so `EnforceDistribution` never inserts a shuffle under the join.

#### Per-Batch Execution Loop (`IterativeLookupJoinExec::execute`)

1. Pull the next non-empty `outer_batch: RecordBatch` from the `left` stream (or terminate on `None`), and filter out outer rows with `NULL` in any equi-join key.
2. Deduplicate the non-null join keys, build an `InListExpr` for the right equi-join key columns, and call `DynamicFilterPhysicalExpr::update`.
3. Call `right.execute(inner_partition, Arc::clone(&context))` (`partition` in `Partitioned` mode, `0` in `ReplicatedInner` mode) and collect the returned inner batches into an in-memory lookup table keyed by the right join keys.
4. Probe the collected inner rows in _outer row order_ (`0..outer_batch.num_rows()`), applying any residual `JoinFilter`, so the output `RecordBatch` strictly preserves the outer `LexOrdering`.
5. If 0 rows matched this outer window, loop immediately to step 1; otherwise yield the joined `RecordBatch`.

### 3.2 Repeatable Inner `PgSearchScanPlan`

To allow `right.execute(...)` to be called once per outer batch, `PgSearchScanPlan` supports repeatable execution via `PgSearchScanPlan::with_repeatable(self, repeatable: bool) -> Self` (cloning `ScanState` rather than transitioning to `ExecutionState::Consumed`).

<details>
<summary>Details on repeatable inner scan state & InListPushdown::Skip handling</summary>

1. _Cloning `ScanState` across `execute()` calls_:
   - Because `PgSearchScanPlan::execute` already clones the `SearchIndexReader` and dynamic filters inside its stream generator before invoking `try_dynamic_filter_pushdown`, each `execute()` call reads the newly updated `DynamicFilterPhysicalExpr` and compiles a fresh `TermSetQuery` without mutating the base `ScanState`.
   - In `IterativeJoinPartitionMode::ReplicatedInner` under MPP, the inner scan's `parallel_state` is `None` (or ignored via a replicated flag) so every worker searches all candidate segments of the inner index rather than claiming from a shared once-only segment counter.
2. _Retaining `PreFilter` when `InListPushdown::Skip` triggers_:
   - In `pg_search/src/scan/pre_filter.rs`, `try_convert_in_list_to_query` returns `InListPushdown::Skip` when term density exceeds `term_set_bitset_max_density_multi()` and rewrites the dynamic filter to `lit(true)`.
   - For iterative inner scans (or in general when `Skip` triggers), `try_dynamic_filter_pushdown` must either force `TermSetQuery` construction or retain the `PreFilter` rather than rewriting the filter to `lit(true)` on small segments.

</details>

---

## 4. Optimization & Plan-Time Heuristics (`iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`)

See `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` for full details on:

- When the planner selects `IterativeLookupJoinExec` over One-Pass `HashJoinExec` and `RankJoinExec` (single-sided or heavily lopsided score ordering, inner `TermSetQuery` pushdown support, and small `LIMIT K`).
- Rule ordering in `build_base_session` (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`) and composition with `RankJoinRule` and `DeferredPlacementRule`.

---

## 5. MPP Execution & Serialization (`datafusion-distributed`)

In both `IterativeJoinPartitionMode::Partitioned` and `IterativeJoinPartitionMode::ReplicatedInner`, `IterativeLookupJoinExec` and its inner child run inside the _same worker stage_ as the outer child (zero network shuffles below the join):

- _Leader stage_: `SortPreservingMergeExec(fetch=K)` over `NetworkCoalesceExec`.
- _Worker stage_: Each worker task `i` runs `LocalLimitExec(fetch=K)` $\rightarrow$ `VisibilityFilterExec` $\rightarrow$ `IterativeLookupJoinExec` (partition `i`), with `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`) serializing `TAG_ITERATIVE_LOOKUP_JOIN` and sharing the `Arc<DynamicFilterPhysicalExpr>` between the join and inner scan via `DeduplicatingProtoConverter`.
