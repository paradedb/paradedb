# Rank Join Design (`RankJoinExec`)

## 1. Motivation & Overview

When a join query has a `LIMIT K` and an `ORDER BY` expression over one or more joined tables — such as:

- _Conjunctive multi-table scoring_: `SELECT a.id, b.id FROM a JOIN b ON a.key = b.key WHERE a.body ||| 'rust' AND b.title ||| 'database' ORDER BY pdb.score(a.id) + pdb.score(b.id) DESC LIMIT 10`
- _Disjunctive multi-table scoring_: `SELECT a.id, b.id FROM a JOIN b ON a.key = b.key WHERE a.body ||| 'python' OR b.title ||| 'python' ORDER BY pdb.score(a.id) + pdb.score(b.id) DESC LIMIT 10`
- _Single-sided or lopsided scoring_: `SELECT a.id, b.id FROM a JOIN b ON a.key = b.key WHERE a.body ||| 'rust' ORDER BY pdb.score(a.id) DESC LIMIT 10`

executing the join via a one-pass `HashJoinExec` must score every matching document on the build side (and under `ScanMode::Tagged` for disjunctive queries, scan every row of every table in the database with `base_query = All`).

`RankJoinExec` unifies _Sorted Access (`SA`)_ (iterative Top-$K$ window pulls via `LazySortConfig`, `iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`) and _Random Access (`RA`)_ (lazy, batched join-key point lookups via `RankLookupPhysicalExpr` $\rightarrow$ `TermSetQuery`) into a single binary physical operator under a monotonic scoring function $f(s_L, s_R)$:

1. _Two-sided `SA + RA` (Balanced & Lopsided Conjunctive Joins)_: Pulls sorted Top-$K$ windows from both inputs (`HRJN*`), and uses lazy `RA` point lookups (`CA` / `MPro`) whenever unmatched top tuples on one side survive up to the emission threshold — eliminating the "orphan top-1" stall and dropping the stopping threshold from the corner bound $\max(f(s_L^{\text{bottom}}, s_R^{\text{top}}), f(s_L^{\text{top}}, s_R^{\text{bottom}}))$ to $f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
2. _Two-sided `SA + RA` over `ScanMode::Tagged` (Disjunctive Joins)_: A row that does not match a table's BM25 predicate is simply a document with score $0.0$ (too low to appear in the positive-score `SA` stream). Each input's `SA` stream pulls only positive-score matches, while `RA` point lookups fetch missing partners (returning $> 0$ if the partner matched its predicate, or $0.0$ if it did not).
3. _One-sided `SA` + One-sided `RA`-only (`NRJN`)_: When one child contributes $0$ to the `ORDER BY` score ($f(s_L, s_R) = s_L$ or $f(s_L, s_R) = s_R$), that child disables `SA` (`initial_window = 0.0`, `sa_exhausted = true`, $s^{\text{top}} = s^{\text{bottom}} = 0.0$) and operates purely via `RA` point lookups driven by the ordered child.

<details>
<summary>Research background & literature references (NRA/TA/CA, MPro/Upper, HRJN*/NRJN, PBRJ, Block-Max WAND)</summary>

`RankJoinExec` synthesizes five foundational results from database and IR research:

1. _`NRA`, `TA`, and `CA` (Combined Algorithm) — Fagin, Lotem, & Naor (PODS 2001 / JCSS 2003, "Optimal Aggregation Algorithms for Middleware")_:
   - Formalized top-$K$ aggregation over sources supporting _Sorted Access (`SA`)_ and _Random Access (`RA`)_, where documents missing from a ranked list implicitly have score $0$:
     - _`NRA` (No Random Access)_: Uses only `SA`; every partially-seen object has a worst-case bound $W(x)$ (unseen score $= 0$) and best-case bound $B(x)$ (unseen score $= s_i^{\text{bottom}}$). Without `RA`, `NRA` must scan deeply into sorted streams when top items on one side have low or $0$ scores on the other side.
     - _`TA` (Threshold Algorithm)_: Performs an `RA` point lookup on every seen object immediately, collapsing the stopping threshold to $\tau = f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$ at the cost of excessive random lookups when $c_{\text{RA}} \gg c_{\text{SA}}$.
     - _`CA` (Combined Algorithm)_: Performs `SA` in batches and _only triggers `RA` point lookups for the top unresolved candidates whose optimistic upper bound $B(x)$ is at the top of the priority queue and blocking termination_.

2. _`MPro` (Minimal Probing) & `Upper` — Chang & Hwang (SIGMOD 2002, "Minimal Probing: Supporting Expensive Predicates for Top-k Queries"); Bruno, Gravano, & Marian (ICDE 2002 / ACM TODS 2004, "Evaluating Top-k Queries over Web-Accessible Databases")_:
   - Proved that scheduling random probes lazily — only probing an unmatched tuple $x$ when its optimistic bound $U(x) = f(s_L(x), s_R^{\text{bottom}})$ rises to the top of the queue (exceeding both the best complete match and the unseen-unseen bound $f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$) — executes the _provably minimal set of random-access lookups_ required by any deterministic top-$K$ algorithm.

3. _`HRJN`, `HRJN*`, and `NRJN` — Ilyas, Aref, & Elmagarmid (VLDB 2003, "Supporting Top-k Join Queries in Relational Databases"; SIGMOD 2004, "Rank-aware Query Optimization"; ACM Computing Surveys 2008, "A Survey of Top-k Query Processing Techniques in Relational Database Systems")_:
   - Generalized Fagin's framework from 1:1 object-ID aggregation to arbitrary $m:n$ relational equi-joins:
     - _`HRJN` / `HRJN*` (Hash Rank Join)_: The relational join analogue of `NRA`, maintaining symmetric hash tables ($H_L, H_R$) and advancing $\arg\max_i U_i$ against the corner bound $T = \max(f(s_L^{\text{bottom}}, s_R^{\text{top}}), f(s_L^{\text{top}}, s_R^{\text{bottom}}))$.
     - _`NRJN` (Nested-Loops Rank Join)_: The relational join analogue of `RA` probing, pulling ordered tuples from an outer child and probing an index on the join attribute of the inner child.

4. _`PBRJ` (Pull/Bound Rank Join) — Schnaitter & Polyzotis (PODS 2008 / ACM TODS 2010, "Evaluating Rank Joins with Optimal Cost")_:
   - Proved instance-optimality properties under the Pull/Bound framework and demonstrated why pure `SA` corner bounds degrade severely when high-scoring tuples on one side are orphans (have no join partner) or match low-scoring partners on the other side — motivating hybrid `SA + RA` resolution of top unmatched tuples.

5. _MaxScore / WAND / Block-Max WAND — Turtle & Flood (1995), Broder et al. (2003), Ding & Suel (SIGIR 2011)_:
   - Tantivy's `Weight::index_max_score`, `Weight::max_score` (per-segment), and `PruningScorer` allow `LazySortMode` (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md` §4) to skip non-competitive segments in $O(1)$ and prune non-competitive posting blocks within each `SA` window.

</details>

---

## 2. Architecture & Composability

### 2.1 Composable Binary Operator Model (Supporting $N \ge 2$ Tables)

Because `RankJoinExec` emits joined rows in strict descending (or ascending) order of its combined score $f(s_L, s_R)$, _its output is itself a lazily-sorted stream that also accepts incoming `RA` point-lookup requests_.

Designing `RankJoinExec` as a binary `ExecutionPlan` (`left`, `right`) where either child can be a `PgSearchScanPlan` (`SA + RA` or `RA`-only) or another `RankJoinExec` provides:

- _Direct 2-table execution_: A single `RankJoinExec` sits over two `PgSearchScanPlan` leaves (either both `SA + RA` for two-sided conjunctive/disjunctive joins, or one `SA` and one `RA`-only for single-sided ordered joins).
- _Multi-table joins ($N \ge 3$)_: A 3-table join `(users ⋈ stackoverflow_posts) ⋈ comments` is represented as `RankJoinExec(RankJoinExec(users, stackoverflow_posts), comments)`:
  - For `SA`, the lower join emits stream $UP$ ordered by $s_u + s_p$ `DESC`, and the upper join combines $(s_u + s_p) + s_c$ `DESC`.
  - For `RA`, when the upper join needs to resolve a batch of `post_id` keys against its left child (the lower `RankJoinExec`), the lower `RankJoinExec` forwards `stackoverflow_posts.id IN (post_ids)` to `PgSearchScan(stackoverflow_posts)`, immediately resolves the resulting `owner_user_id` keys against `PgSearchScan(users)`, and returns the joined `(user, post)` pairs to the upper join.

### 2.2 Deferred Visibility & Dynamic Filters

`RankJoinExec` is pull-driven and order-preserving:

- `RankJoinExec` emits candidates from its internal max-heap in target sort order whenever `candidate.rank_value >= T` (or `> T` when secondary tie-breaker columns are present).
- If `VisibilityFilterExec` (or a residual `JoinFilter`) rejects an emitted row, the upstream `Limit(K)` simply calls `Stream::poll_next` again, causing `RankJoinExec` to continue advancing `SA` windows and `RA` point lookups until $K$ visible rows survive.

---

## 3. Component Design (`RankJoinExec`)

### 3.1 Named Types, Signatures & Trait Implementations

- `RankJoinExec` implements DataFusion's `ExecutionPlan` and `DisplayAs` traits:
  - `ExecutionPlan::required_input_ordering` returns `[Some(left_sort_expr), Some(right_sort_expr)]` (or `None` for an `RA`-only child whose score contribution is unused in `ScoreCombiner`).
  - `ExecutionPlan::gather_filters_for_pushdown` mints a `DynamicFilterPhysicalExpr` for each child (`left_lookup_filter`, `right_lookup_filter`) during `FilterPushdownPhase::Post`, initialized to `lit(true)`.
  - `ExecutionPlan::handle_child_pushdown_result` intercepts any incoming lookup `DynamicFilterPhysicalExpr` pushed down from a parent `RankJoinExec` (in $N \ge 3$ multi-way joins), identifies which of its own children (`left` or `right`) owns the target lookup column, and records `(side, child_filter)` in `parent_lookup_filters`.
- `RankJoinStream` implements `futures::Stream<Item = Result<RecordBatch>>` and `RecordBatchStream`.
- `RankedInputCursor` wraps a child `SendableRecordBatchStream` inside `RankJoinStream`:
  - Steps through buffered `SA` `RecordBatch` rows in memory (`top_score`, `bottom_score`, `sa_exhausted`).
  - Tracks `resolved_keys: HashSet<OwnedRow>` and `unresolved_keys: VecDeque<(f64, OwnedRow)>` (seen keys from this side ordered by score that have not yet been point-looked-up in the opposite child).
  - Supports both `poll_ensure_buffered(cx)` (advancing the child's `SA` Top-$K$ window) and `poll_ra_batch(cx)` (polling the child stream after updating its lookup `DynamicFilterPhysicalExpr` with a `RankLookupPhysicalExpr`).

### 3.2 Stopping Threshold $T$ & Lazy `CA` / `MPro` Scheduling

At any point during execution of `RankJoinStream`:

1. _Frontier & Unresolved Scores_:
   - Let $s_L^{\text{bottom}}$ (resp. $s_R^{\text{bottom}}$) be the current frontier score of `left_cursor` (resp. `right_cursor`):
     - When `sa_exhausted` is `true`, $s_i^{\text{bottom}} = 0.0$ if `ra_capable` is `true` (since unseen documents still reachable via `RA` have score $\ge 0.0$), or no further matches are possible from that side's unseen `SA` tail when `ra_capable` is `false`.
   - Let $s_L^{\text{unres}}$ (resp. $s_R^{\text{unres}}$) be the highest score among seen keys in `left` (resp. `right`) that have _not_ yet been resolved via an `RA` point lookup on the opposite side (or $s_i^{\text{top}}$ if the opposite side is not `ra_capable` and not yet `sa_exhausted`).

2. _Three-Quadrant Upper Bounds ($U_{\text{RA}, L}, U*{\text{RA}, R}, U*{\text{SA}}$)_:
   Any join pair $(l, r)$ not yet in `candidate_heap` belongs to one of three unseen quadrants:
   - _Quadrant 1 (`Seen Unresolved Left` $\times$ `Unseen Right`)_:
     $$U_{\text{RA}, L} = f(s_L^{\text{unres}},\; s_R^{\text{bottom}})$$
   - _Quadrant 2 (`Unseen Left` $\times$ `Seen Unresolved Right`)_:
     $$U_{\text{RA}, R} = f(s_L^{\text{bottom}},\; s_R^{\text{unres}})$$
   - _Quadrant 3 (`Unseen Left` $\times$ `Unseen Right`)_:
     $$U_{\text{SA}} = f(s_L^{\text{bottom}},\; s_R^{\text{bottom}})$$
   - If a child does not support `RA` point lookups, its opposite `unres` score remains fixed at $s_i^{\text{top}}$, smoothly degenerating to the classic `HRJN*` corner bound $\max(f(s_L^{\text{bottom}}, s_R^{\text{top}}), f(s_L^{\text{top}}, s_R^{\text{bottom}}))$.
   - The global stopping threshold is:
     $$T = \max\bigl(U_{\text{RA}, L},\; U_{\text{RA}, R},\; U_{\text{SA}}\bigr)$$

3. _Advancement & Lazy `RA` Resolution Loop_:
   - _Step A (Drain in-memory `SA` buffers)_: While either cursor has buffered `SA` rows in memory and `candidate_heap` does not yet have a ready batch clearing $T$, advance the cursor with the higher bound one row at a time in memory (`advance_one_row`), inserting into $H_L$ / $H_R$, probing the opposite hash table, and recording newly seen keys as unresolved if not already in `resolved_keys`.
   - _Step B (Emit ready candidates)_: If `candidate_heap.peek()` clears $T$, drain ready candidates, apply any residual `JoinFilter` (such as `tag_L OR tag_R` in `ScanMode::Tagged`), and emit the `RecordBatch`.
   - _Step C (Lazy `RA` Point Lookup when $U_{\text{RA}}$ holds up $T$)_:
     When an in-memory buffer is empty (or `sa_exhausted`) and $U_{\text{RA}, L} > U*{\text{SA}}$ (or $U*{\text{RA}, R} > U\_{\text{SA}}$) prevents emitting candidates or lowering $T$:
     - Collect a batch of top unresolved keys from `left` whose $f(s_L(k), s_R^{\text{bottom}}) > U_{\text{SA}}$ (up to `RA_LOOKUP_BATCH_SIZE = 64`), mark them in `resolved_keys`, publish a `RankLookupPhysicalExpr` to `right_lookup_filter`, and poll `right` to ingest all matching rows into $H_R$ and `candidate_heap`.
     - Symmetrically resolve blocking unresolved keys from `right` against `left`.
     - Once resolved, $s_L^{\text{unres}}$ and $s_R^{\text{unres}}$ drop, collapsing $T$ down to $U_{\text{SA}} = f(s_L^{\text{bottom}}, s_R^{\text{bottom}})$.
   - _Step D (`SA` Window Pull)_: When $T = U_{\text{SA}}$ still blocks emission, pull the next `SA` Top-$K$ window from whichever input (`left` or `right`) is not yet `sa_exhausted`.

### 3.3 Single-Sided Joins (`NRJN` Special Case)

When the `ORDER BY` score expression depends on only one side of the join (e.g., `SELECT a.id, b.id FROM a JOIN b ON a.k = b.k WHERE a.body ||| 'rust' AND b.status = 'published' ORDER BY pdb.score(a.id) DESC LIMIT K`), `RankJoinExec` executes `NRJN` directly without a separate operator:

1. _Bounds Setup_:
   - The scored child (`a`, say `left`) is configured with `LazySortConfig` (`initial_window > 0`), streaming `SA` windows in score order.
   - The unscored child (`b`, say `right`) contributes $0$ to the score combiner ($f(s_L, s_R) = s_L$, so `combiner.uses_right() == false`) and is configured with `initial_window = 0.0` (`sa_exhausted = true`, $s_R^{\text{top}} = s_R^{\text{bottom}} = 0.0$ from the start).
   - The three-quadrant bounds (§3.2) simplify automatically to:
     $$U_{\text{RA}, L} = s_L^{\text{unres}}, \quad U_{\text{RA}, R} = -\infty, \quad U_{\text{SA}} = s_L^{\text{bottom}}, \quad T = \max(s_L^{\text{unres}}, s_L^{\text{bottom}})$$
2. _Per-Window Execution Trace_:
   - `RankJoinStream` pulls the first `SA` window from `left` (`a`), buffering rows down to frontier score $s_L^{\text{bottom}}$ and recording their join keys as unresolved ($s_L^{\text{unres}} = s_L^{\text{top}}$).
   - Because $U_{\text{RA}, L} = s_L^{\text{unres}} > U_{\text{SA}} = s_L^{\text{bottom}}$, `RankJoinStream` batches the unresolved join keys from `left`'s window into `right_lookup_filter` (`RankLookupPhysicalExpr`) and polls `right`.
   - `PgSearchScanPlan(b)` executes `b.status = 'published' AND TermSetQuery(b.k IN (...))` and yields the matching `b` rows in a single pass without advancing an `SA` window.
   - `RankJoinStream` joins those `b` rows with $H_L$, pushes the matches into `candidate_heap`, and marks the keys resolved, dropping $T$ to $s_L^{\text{bottom}}$.
   - All matched rows in the window satisfy $s_L \ge s_L^{\text{bottom}} = T$ and are immediately emitted in exact `left` sort order (including any secondary tie-breaker columns).

### 3.4 Cascading `RA` Lookups in Nested `RankJoinExec` ($N \ge 3$ Joins)

When a parent `RankJoinExec` issues a `RankLookupPhysicalExpr` (`col IN (K)`) to a child `RankJoinExec`:

1. The child `RankJoinStream` detects a `RankLookupPhysicalExpr` on one of its `parent_lookup_filters` (`(target_side, filter_expr)`), resets that filter to `lit(true)`, and extracts the requested keys.
2. It forwards `col IN (K)` to whichever of its own children (`left` or `right`, say `right`) owns `target_side`, and polls `right` via `poll_ra_batch` to obtain the matching batch $B_R$.
3. It ingests $B_R$ into `right_hash_table`, extracts any join keys in $B_R$ that are not yet in `left`'s `resolved_keys`, issues an `RA` lookup for those join keys to `left` to obtain $B_L$, and ingests $B_L$ into `left_hash_table`.
4. All newly formed `(left_row, right_row)` pairs matching `col IN (K)` (passing any local `JoinFilter`) are materialized and returned immediately to the parent `RankJoinExec`, and recorded in `emitted_pairs: HashSet<(BufferedRowRef, BufferedRowRef)>` so they are not emitted a second time if also present in `candidate_heap`.

---

## 4. Optimization & Plan-Time Heuristics (`iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md`)

See `iterative-joins/TOPK_JOIN_OPTIMIZATION_DESIGN.md` for full details on:

- How `RankJoinRule` rewrites both two-sided (`SA + RA`) and single-sided (`SA` + `RA`-only) joins, as well as `ScanMode::Tagged` disjunctive joins, into `RankJoinExec`.
- Monotonicity verification and optimizer rule ordering in `pg_search/src/postgres/customscan/joinscan/scan_state.rs`.

---

## 5. MPP Execution & Partitioning (`datafusion-distributed`)

Because `RankJoinExec` relies on dynamic filters (`FilterPushdown`) to prune random-access (`RA`) point lookups on the fly, and cross-process dynamic filters are currently disabled in `datafusion-distributed` (`with_distributed_remote_dynamic_filters(false)`), `RankJoinRule` is currently gated off when `mpp_is_active()` is true (the inverse of `RangePartitioningRule` / `RangeCoPartitionedJoinRule`).

Once dynamic filter support under MPP is enabled, `RankJoinExec` will use DataFusion's standard `PartitionMode` (`Partitioned` vs. `CollectLeft`):

1. _`PartitionMode::Partitioned` with Range Co-Partitioning (1 Worker Stage, 0 Shuffles)_:
   When both inputs are `ExecutionState::RangePartitioned` with matching split points on the join keys, every worker task runs `RankJoinExec` (`SA + RA`) locally on its disjoint key range in a single stage.
2. _`PartitionMode::CollectLeft` / Replicated Broadcast Scan in a Single Worker Stage_:
   When one input is range-partitioned (e.g., `stackoverflow_posts` and `comments` co-partitioned on `post_id`) and a smaller lookup/join table (e.g., `users` on `owner_user_id`) is `CollectLeft`, each worker task runs both its local partition and `SA + RA` lookups inside the same stage without an intermediate network shuffle.
3. _Protobuf Codec_: `PgSearchPhysicalExtensionCodec` (`pg_search/src/scan/physical_codec.rs`) serializes `RankJoinExec` (`TAG_RANK_JOIN`) and `RankLookupPhysicalExpr` (`TAG_RANK_LOOKUP_EXPR`), preserving shared `Arc<DynamicFilterPhysicalExpr>` pointers via `DeduplicatingProtoConverter`.
