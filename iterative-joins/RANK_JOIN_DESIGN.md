# Rank Join Design (`RankJoinExec`)

## 1. Motivation & Overview

When a join query has a `LIMIT K` and an `ORDER BY` whose scoring expression combines columns or search scores from _multiple joined tables_ — for example, `SELECT a.id, b.id FROM a JOIN b ON a.key = b.key WHERE a.body @@@ 'rust' AND b.title @@@ 'database' ORDER BY pdb.score(a.id) + pdb.score(b.id) DESC LIMIT 10` — neither table alone determines the final sort order.

Today, `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`) executes multi-table scored joins in a single pass using `HashJoinExec` (and, for disjunctive or join-level search predicates, predicate tagging via `ScanMode::Tagged` in `pg_search/src/scan/batch_scanner.rs`) followed by `SortExec(TopK)` or `SegmentedTopKExec`. While that guarantees a single sequential `DocId`-order pass over each table, it must score every matching document on the build side (and under `ScanMode::Tagged`, across both sides) because it cannot use Tantivy's Block-Max WAND pruning.

`RankJoinExec` consumes two lazily-sorted inputs (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`) in descending score order under a _monotonic scoring function_ $f(s_L, s_R)$. By pushing Top-K windowing directly into each input's `PgSearchScanPlan`, `RankJoinExec` allows Tantivy's Block-Max WAND collector to skip non-competitive posting blocks on _both_ joined inputs and stops as soon as the top $K$ results are guaranteed to outscore any unseen combination.

<details>
<summary>Research background & literature references (TA/NRA, HRJN*, PBRJ, Block-Max WAND)</summary>

`RankJoinExec` is grounded in four results from database and IR research:

1. _NRA / Threshold Algorithm (TA) — Fagin, Lotem, & Naor (PODS 2001, "Optimal Aggregation Algorithms for Middleware")_:
   - Introduced the stopping threshold $T$ over sorted streams combined by a monotonic function $f$: if every input $i$ has been read down to frontier score $s_i^{\text{bottom}}$, no unseen object can achieve a combined score exceeding $T$, so any buffered candidate with score $\ge T$ can be emitted safely.

2. _HRJN & Score-Guided HRJN — Ilyas, Aref, & Elmagarmid (VLDB 2003, "Supporting Top-k Join Queries in Relational Databases"; SIGMOD 2004, "Rank-aware Query Optimization"; ACM Computing Surveys 2008)_:
   - Generalized TA/NRA from 1:1 object-ID joins to arbitrary $m:n$ relational equi-joins:
     - _HRJN (Hash Rank Join)_: Maintains symmetric in-memory hash tables ($H_L, H_R$) of tuples seen so far from each ranked input alongside a priority queue of joined candidates ordered by $f(s_L, s_R)$, bounding unseen join matches by the corner threshold:
       $$T = \max\bigl(f(s_L^{\text{bottom}}, s_R^{\text{top}}),\; f(s_L^{\text{top}}, s_R^{\text{bottom}})\bigr)$$
     - _Score-Guided HRJN ($\text{HRJN}^*$, VLDB 2003 §4.2)_: Rather than alternating between inputs in round-robin order (which performs poorly when score distributions or weights are skewed), $\text{HRJN}^*$ compares the individual upper bounds $U_L = f(s_L^{\text{bottom}}, s_R^{\text{top}})$ and $U_R = f(s_L^{\text{top}}, s_R^{\text{bottom}})$ and always advances the input with the larger $U_i$ ($\arg\max_i U_i$), because $T = \max(U_L, U_R)$ cannot decrease unless the input realizing the maximum is advanced.

3. _PBRJ (Pull-Bound Rank Join) — Schnaitter & Polyzotis (SIGMOD 2008 / ACM TODS 2010, "Evaluating Rank Joins with Optimal Cost")_:
   - Proved the instance-optimality properties of pull-bound strategies that advance $\arg\max_i U_i$ and analyzed block/windowed pulling from underlying index scans.

4. _MaxScore / WAND / Block-Max WAND — Turtle & Flood (1995), Broder et al. (2003), Ding & Suel (SIGIR 2011)_:
   - Tantivy's `Weight::index_max_score`, `Weight::max_score` (per-segment), and `PruningScorer` allow `RankJoinExec` and `LazySortMode` (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` §4) to skip non-competitive segments in $O(1)$ and push cross-join score thresholds ($\theta_L = \theta_K - s_R^{\text{top}}$) back down into Tantivy's posting-list collectors.

</details>

---

## 2. Architecture & Composability

### 2.1 Composable Binary Operator Model (Supporting $N \ge 2$ Tables)

Because `RankJoinExec` emits joined rows in strict descending order of its combined score $f(s_L, s_R)$, _its output is itself a lazily-sorted stream_.

Designing `RankJoinExec` as a _binary_ `ExecutionPlan` (`left`, `right`) where either child can be a `PgSearchScanPlan` in `LazySortMode` (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`) or another `RankJoinExec` provides:

- _Direct 2-table execution_: A single `RankJoinExec` sits directly over two `PgSearchScanPlan` leaves in `LazySortMode`.
- _Multi-table joins ($N \ge 3$)_: A 3-table join `A JOIN B ON a.x = b.x JOIN C ON b.y = c.y ORDER BY s_a + s_b + s_c DESC LIMIT K` is represented as `RankJoinExec(RankJoinExec(A, B), C)`. The lower join hashes on `x` (emitting stream $AB$ ordered by $s_{ab} = s_a + s_b$ `DESC`), and the upper join hashes on `y` (combining $s_{ab} + s_c$ `DESC`).
- _Composition with `IterativeLookupJoinExec` (`iterative-joins/ITERATIVE_TOPK_JOIN_DESIGN.md`)_: When a query joins scored tables $(A, B)$ with an unscored lookup table $C$ (`ORDER BY pdb.score(a.id) + pdb.score(b.id) DESC LIMIT K`), `RankJoinExec(A, B)` serves directly as the _left (outer)_ child of `IterativeLookupJoinExec(..., C)`.

### 2.2 Deferred Visibility & Dynamic Score Thresholds

Just like `IterativeLookupJoinExec`, `RankJoinExec` is pull-driven and order-preserving:

- `RankJoinExec` emits candidates from its internal max-heap in descending score order whenever `candidate.score >= T` (where $T$ is the upper bound on any unseen match).
- If `VisibilityFilterExec` (or an upstream `IterativeLookupJoinExec`) rejects an emitted row, the top-level `Limit(K)` simply calls `Stream::poll_next` again, causing `RankJoinExec` to pull further from its inputs and lower $T$ until $K$ visible rows survive.

<details>
<summary>Verified vs. unverified candidates for dynamic score pruning & future visibility placement</summary>

- While the emission condition (`candidate.score >= T`) is always sound regardless of downstream filtering, pushing a _pruning threshold_ $\theta_L = \theta_K - s_R^{\text{top}}$ down to the input scans via `DynamicFilterPhysicalExpr` to permanently skip rows with $s_L < \theta_L$ requires that $\theta_K$ be a _verified_ lower bound on the final $K$-th output row.
- If selective operators (`VisibilityFilterExec`, `FilterExec`, or `IterativeLookupJoinExec`) sit above `RankJoinExec`, an unverified $K$-th match inside `RankJoinExec`'s internal candidate heap might later be discarded by MVCC visibility or the lookup join.
- Therefore, within each `LazySortMode` window pull, Tantivy's `TopDocs` collector uses its window-local $(offset + local\_limit)$-th threshold for Block-Max WAND pruning, while any cross-window hard cutoff $\theta_K$ must only be updated from _post-filter verified survivors_ (or enabled when no selective operators sit between `RankJoinExec` and `Limit(K)`).
- _Future Work_: Placing `VisibilityFilterExec` _below_ `RankJoinExec` ensures every tuple entering $H_L$ and $H_R$ is already MVCC-visible, unlocking immediate cross-join score threshold pushdown from the $K$-th heap element at the cost of checking visibility on rows before they match the join.

</details>

---

## 3. Component Design (`RankJoinExec`)

### 3.1 Named Types, Signatures & Trait Implementations

- `RankJoinExec` implements DataFusion's `ExecutionPlan` and `DisplayAs` traits:
  - Constructed via `RankJoinExec::try_new(left, right, on, filter, score_expr, score_sort_options, left_sort_expr, right_sort_expr, projection, mode, fetch) -> Result<Self>`.
  - `ExecutionPlan::required_input_ordering` returns `[Some(left_sort_expr), Some(right_sort_expr)]`.
  - `ExecutionPlan::required_input_distribution` mirrors `HashJoinExec` based on `PartitionMode`:
    - `PartitionMode::CollectLeft`: `[Distribution::SinglePartition, Distribution::UnspecifiedDistribution]`
    - `PartitionMode::Partitioned`: `[Distribution::HashPartitioned(left_keys), Distribution::HashPartitioned(right_keys)]`
  - `ExecutionPlan::properties` registers the output ordering `[PhysicalSortExpr { expr: score_col, options: score_sort_options }]` in `PlanProperties::eq_properties`.
- `RankJoinStream` implements `futures::Stream<Item = Result<RecordBatch>>` and `RecordBatchStream` for a single output partition of `RankJoinExec::execute(partition, context) -> Result<SendableRecordBatchStream>`.
- `RankedInputCursor` wraps a child `SendableRecordBatchStream` inside `RankJoinStream`, stepping through buffered `RecordBatch` rows in memory (`top_score`, `bottom_score`, `is_exhausted`, `advance_next`) and only polling the underlying stream for the next deeper Tantivy window when the current batch is exhausted.

### 3.2 Execution Loop & Stopping Threshold $T$

Inside `RankJoinExec::execute(partition, context)`:

1. _Initialize Top Scores_: Pull the first batch from the left and right `RankedInputCursor`s. If either side is empty, an `Inner` join terminates immediately. Otherwise, record the best scores $s_L^{\text{top}} = \text{score}(L[0])$ and $s_R^{\text{top}} = \text{score}(R[0])$.
2. _Track Upper Bounds ($U_L, U_R, T$)_:
   - The best score any future match using a new left row can achieve is $U_L = f(s_L^{\text{bottom}}, s_R^{\text{top}})$ (or $-\infty$ if `left` is exhausted).
   - The best score any future match using a new right row can achieve is $U_R = f(s_L^{\text{top}}, s_R^{\text{bottom}})$ (or $-\infty$ if `right` is exhausted).
   - The global upper bound on any unseen match is $T = \max(U_L, U_R)$.
3. _Advance the Input Holding Up $T$_:
   - While the candidate max-heap does not have enough ready rows with `score >= T`:
     - Pick `left` if $U_L \ge U_R$, else `right`.
     - Advance that cursor by a row (or small slice), insert into its hash table ($H_L$ or $H_R$), probe the opposite hash table, evaluate $f(s_L, s_R)$ on any matches passing residual `JoinFilter`, push matches onto the candidate max-heap, and update $s_i^{\text{bottom}}$ and $T$.
4. _Emit Ready Candidates_: Pop all candidates from the max-heap with `score >= T` in descending score order and emit them as a `RecordBatch`.

<details>
<summary>Progression of Tantivy max_score integration (v1 to Phase 3)</summary>

1. _v1 (Empirical Bounds from Window 1)_: Uses row `0` of the first `LazySortMode` batch on each side for $s_L^{\text{top}}$ and $s_R^{\text{top}}$, and the current cursor row for $s_L^{\text{bottom}}$ and $s_R^{\text{bottom}}$. Requires zero changes to Tantivy's scorer traits.
2. _Phase 2 (`Weight::index_max_score` & `Weight::max_score` — Proposal A in `tantivy-0`)_: `SearchIndexReader::search_top_k_in_segments` uses `Weight::max_score(segment_reader, 1.0)` to skip segments whose max score is `0.0` or below the current threshold in $O(1)$, and `Query::index_max_score` provides plan-time bounds for `RankJoinRule`.
3. _Phase 3 (Cross-Join Threshold Pushdown & Proposal B `PruningScorer::max_score`)_: Once $K$ verified results exist with $K$-th score $\theta_K$, `RankJoinExec` computes the minimum viable partner scores $\theta_L, \theta_R$ and passes them into `LazySortMode` / `PruningScorer::set_threshold` to accelerate Block-Max WAND pruning.

</details>

---

## 4. Optimization & Plan-Time Heuristics (`iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`)

See `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` for full details on:

- When `RankJoinRule` chooses `RankJoinExec` over One-Pass `HashJoinExec` and `IterativeLookupJoinExec`.
- The v1 heuristics (`ScanMode::Standard`, `Query::index_max_score` balance check, and the $\sqrt{K \cdot D}$ window depth gate against $\min(M_L, M_R)$).
- Monotonicity verification and optimizer rule ordering in `pg_search/src/postgres/customscan/joinscan/scan_state.rs`.

---

## 5. MPP Execution & Partitioning (`datafusion-distributed`)

`RankJoinExec` requires _no new scan output modes_ on `PgSearchScanPlan`; `PgSearchScanPlan` uses only its two existing modes (`ExecutionState::Shared` for unpartitioned + work-stealing, and `ExecutionState::RangePartitioned`).

Instead, `RankJoinExec` uses DataFusion's standard `PartitionMode` (`CollectLeft` vs. `Partitioned`), directly paralleling `HashJoinExec`:

1. _`PartitionMode::Partitioned` with Range Co-Partitioning (1 Worker Stage, 0 Shuffles)_:
   When both inputs are `ExecutionState::RangePartitioned` with matching split points on the join keys, both inputs already satisfy `RankJoinExec`'s partitioning and `required_input_ordering()`. Every worker task runs `RankJoinExec` on its disjoint key range in a single pull-driven stage.
2. _`PartitionMode::CollectLeft` (Broadcast — 2 Worker Stages)_:
   When inputs use `ExecutionState::Shared` and the left side is broadcast, `RankJoinExec`'s `required_input_ordering()` causes `EnforceSorting` to place `SortPreservingMergeExec([s_a DESC])` above `NetworkBroadcastExec` so each consumer task receives the broadcast stream in sorted order while scanning its own work-stolen segments of `b` locally.
3. _`PartitionMode::Partitioned` with Hash Shuffle (`NetworkShuffleExec`)_:
   When neither input is range-partitioned and `PartitionMode::Partitioned` is used over `ExecutionState::Shared`, `EnforceDistribution` inserts `RepartitionExec(Hash)` (`NetworkShuffleExec`) on both inputs and `EnforceSorting` merges the incoming per-producer sorted streams into `RankJoinExec`.
4. _Protobuf Codec_: `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`) adds `TAG_RANK_JOIN` to serialize `RankJoinExec`.

<details>
<summary>Producer pacing consideration under NetworkShuffleExec</summary>

Because producer stages below `NetworkShuffleExec` in `pg_search/src/postgres/customscan/mpp/exec_worker.rs` are push-driven (`run_worker_fragment`), an unpaced producer stage would immediately poll subsequent windows from `LazySortMode` until `shm_mq` fills up. Because `RankJoinExec` is only chosen when a small fraction of the table is needed, even a small number of shuffled batches is lightweight, and we can optionally gate `query_count > 1` in producer `LazySortMode` scans on a consumer demand counter in `ParallelScanState`.

</details>
