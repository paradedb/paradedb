# Top-K Join Optimization & Plan-Time Heuristics Design

## 1. Overview

When a join query has an `ORDER BY ... LIMIT K`, `JoinScan` (`pg_search/src/postgres/customscan/joinscan/`) has three possible ways to execute it:

1. _One-Pass `HashJoinExec`_ (our existing strategy, paired with `SegmentedTopKExec` or `SortExec(TopK)`).
2. _`RankJoinExec`_ (`iterative-joins/RANK_JOIN_DESIGN.md`): Pulls top-scoring windows from _both_ inputs in parallel and joins them in memory until the top `K` matches are found.
3. _`IterativeLookupJoinExec`_ (`iterative-joins/ITERATIVE_TOPK_JOIN_DESIGN.md`): Pulls a top-scoring window from _one_ outer input and probes the other input by join key (`TermSetQuery`).

The core trade-off between these strategies is _guaranteed single-pass scanning_ versus _skipping work with Tantivy's Top-K collector_:

- `HashJoinExec` scans each table in a single pass in `DocId` order. It never re-queries Tantivy, and it can push join keys from a small build table into a large probe table. However, because it scans in `DocId` order rather than score order, it must score _every_ matching document on the build side (and when predicate tagging via `ScanMode::Tagged` is used, on both sides).
- `RankJoinExec` and `IterativeLookupJoinExec` use `LazySortMode` (`iterative-joins/LAZY_SORTED_SCAN_DESIGN.md`), which runs Tantivy's Top-K collector (Block-Max WAND). This skips scoring 90–99%+ of matching documents when only a small window is needed. However, if the first window does not produce `K` surviving join matches, they must re-query Tantivy with progressively larger windows.

---

## 2. When Each Strategy Wins

At a high level, the planner chooses between the three strategies based on _how many documents match on each side_ and _which tables contribute to the `ORDER BY` score_:

- _Use One-Pass `HashJoinExec` when at least one side has few matches, or `K` is large_:
  If either table matches only a small number of documents (say, a few thousand rows), `HashJoinExec` builds a small hash table on that side and pushes an `IN (...)` filter (`TermSetQuery`) into the other side. Both tables finish quickly in one pass, so lazy windowing is unnecessary.
- _Use `RankJoinExec` when both sides have many matches and both contribute comparably to the score_:
  For queries like `ORDER BY pdb.score(a.id) + pdb.score(b.id) DESC LIMIT 10` where both `a` and `b` match hundreds of thousands of documents, `HashJoinExec` would have to score all of them. `RankJoinExec` lets _both_ scans use Block-Max WAND and stops after reading a small prefix of each.
- _Use `IterativeLookupJoinExec` when only one side drives the score (or heavily dominates it)_:
  For queries like `ORDER BY pdb.score(a.id) DESC LIMIT 10` (where `b` is unscored, or `b`'s max score is tiny compared to `a`'s), pulling top-scoring rows from `a` and doing direct join-key lookups into `b` avoids scanning `b` by score altogether.

<details>
<summary>Why heavily lopsided scores hurt RankJoinExec and favor IterativeLookupJoinExec (example & proof)</summary>

Suppose `ORDER BY 100 * pdb.score(a.id) + pdb.score(b.id) DESC LIMIT 10`, where `a`'s weighted score ranges from `0` to `100` ($s_L^{\text{top}} = 100$) and `b`'s score ranges from `0` to `1` ($s_R^{\text{top}} = 1$).

`RankJoinExec` can only stop when its upper-bound threshold $T = \max(U_L, U_R)$ drops below the 10th best matched score $\theta_{10}$, where:

- $U_L = s_L^{\text{bottom}} + s_R^{\text{top}}$ (best possible score using an unseen row from `a` and the best row from `b`)
- $U_R = s_L^{\text{top}} + s_R^{\text{bottom}}$ (best possible score using the best row from `a` and an unseen row from `b`)

Suppose the 10th best join match has score $\theta_{10} = 95$.

- On side `a`, $U_L = s_L^{\text{bottom}} + 1 \le 95$ as soon as $s_L^{\text{bottom}} \le 94$, which might happen after pulling just 20 rows from `a`.
- On side `b`, however, $U_R = 100 + s_R^{\text{bottom}}$. Because $s_R^{\text{bottom}} \ge 0$, $U_R$ can _never_ drop below `100` (let alone `95`) until `b` is completely exhausted! Any unseen row at the very bottom of `b` could still theoretically join with row `0` of `a` (score `100`) to beat `95`.

Thus, when one table's score range is smaller than the top-$K$ score drop on the other table ($s_R^{\text{top}} < s_L^{\text{top}} - \theta_K$), a symmetric `RankJoinExec` is forced to scan 100% of the weaker-scored table. Driving from `a` and probing `b` by join key (`IterativeLookupJoinExec`) avoids scanning `b`.

</details>

---

## 3. Initial v1 Heuristics (`RankJoinExec` vs. One-Pass Joins Only)

In v1, before `IterativeLookupJoinExec` is implemented and without accurate join-selectivity statistics, `RankJoinRule` chooses between `RankJoinExec` and One-Pass `HashJoinExec` using only information already present on `PgSearchScanPlan` and `SearchIndexReader` at plan time:

- $K$: `SortExec::fetch()` (`limit + offset`).
- $M_L, M_R$: Estimated matching documents on each side (`ScanInfo::estimate`, computed by `SearchIndexReader::estimate_docs`).
- $D_L, D_R$: Total documents in each index (`SearchIndexReader::total_docs`).
- $s_L^{\max}, s_L^{\min}$ and $s_R^{\max}, s_R^{\min}$: Static score or field bounds (`Query::index_max_score` for BM25 scores, or `Column::min_value` / `Column::max_value` for fast fields).
- `ScanMode`: `ScanMode::Standard` vs. `ScanMode::Tagged`.

`RankJoinRule` rewrites an inner `HashJoinExec` into `RankJoinExec` only when all three of the following gates pass:

### Gate 1: Both Scans Can Use Tantivy's Top-K Collector

1. Both inputs use `ScanMode::Standard` (not `ScanMode::Tagged`, which computes tag scores after scanning and cannot be ordered inside Tantivy's `TopDocs` collector).
2. Neither input's search query is an unselective constant-score query (`AllQuery` or `ConstScoreQuery`), since flat scores cannot be pruned by Block-Max WAND.

### Gate 2: Both Tables Contribute Comparably to the Score

Because `IterativeLookupJoinExec` is not in v1, single-table or heavily lopsided score queries should stay on One-Pass `HashJoinExec` + `SegmentedTopKExec` (which already pushes single-table score thresholds down to the probe scan) rather than forcing `RankJoinExec` to scan an unscored/weakly-scored side to completion.

Using `Query::index_max_score` (or fast-field min/max bounds), `RankJoinRule` checks how much each side can change the combined score $f(s_L, s_R)$:

- $\Delta_L^{\max} = |f(s_L^{\max}, s_R^{\max}) - f(s_L^{\min}, s_R^{\max})|$
- $\Delta_R^{\max} = |f(s_L^{\max}, s_R^{\max}) - f(s_L^{\max}, s_R^{\min})|$

Both $\Delta_L^{\max}$ and $\Delta_R^{\max}$ must be positive, and the smaller one must be at least $\tau_{\text{balance}}$ (e.g. `20%`) of the larger one.

### Gate 3: Both Inputs Have Many Matches Relative to the Expected Window Depth

Unlike `IterativeLookupJoinExec`, `RankJoinExec` does not look up partner rows by key — it pulls top-scoring rows from $L$ and $R$ independently and waits for their join keys to collide in its hash tables.

Over a join key domain of size $D = \max(D_L, D_R)$, two independent sorted streams need to pull roughly $W_{\text{est}} = c \cdot \sqrt{K \cdot D}$ rows each before $K$ join keys collide (where $c \ge 1$ is a small multiplier for MVCC visibility and score threshold clearing, e.g. $c = 2$). Meanwhile, One-Pass `HashJoinExec` only has to build a hash table on the smaller input $\min(M_L, M_R)$ and push a `TermSetQuery` into the larger input.

Therefore, `RankJoinRule` requires:

1. `LIMIT K` is small: $K \le \text{rank\_join\_max\_k}$ (e.g. `1,000`).
2. Both inputs match a non-trivial number of documents: $\min(M_L, M_R) \ge M_{\min}$ (e.g. `10,000`).
3. The estimated `RankJoinExec` pull depth is a small fraction (e.g. $\eta = 10\%$) of the smaller input's matching rows:
   $$c \cdot \sqrt{K \cdot \max(D_L, D_R)} \le \eta \cdot \min(M_L, M_R)$$

When `RankJoinExec` is chosen, `RankJoinRule` also sets `LazySortConfig::initial_window` on both scans to $\sqrt{K \cdot \max(D_L, D_R)}$ (clamped to `[K * scale_factor, max_initial_window]`) so the very first window pull is large enough to produce $K$ matches without repeated Tantivy re-queries.

<details>
<summary>Derivation of the sqrt(K * D) window depth and worked examples</summary>

#### Why Independent Ranked Streams Need $\sim \sqrt{K \cdot D}$ Rows

Suppose tables $L$ and $R$ join on a key with domain size $D = \max(D_L, D_R)$, and the score orderings of $L$ and $R$ are independent of the join key. After pulling $W$ rows from $L$ and $W$ rows from $R$, there are $W^2$ pairs of rows, each colliding on the join key with probability $1 / D$. The expected number of join matches in memory is:

$$\mathbb{E}[\text{matches}] \approx \frac{W^2}{D}$$

Setting $\mathbb{E}[\text{matches}] \ge K$ gives $W \ge \sqrt{K \cdot D}$ (the birthday-paradox / hypergeometric intersection bound from Ilyas et al., VLDB 2003).

#### Worked Examples (with $c = 2, \eta = 0.1$)

1. _Both predicates match many rows (`RankJoinExec` wins)_:
   - $D_L = D_R = 1,000,000$, $M_L = 400,000$, $M_R = 500,000$, $K = 10$.
   - $W_{\text{est}} = 2 \cdot \sqrt{10 \times 1,000,000} \approx 6,324$ rows.
   - Since $6,324 \le 0.1 \times 400,000 = 40,000$, `RankJoinExec` is selected and scores $\sim 6\text{K}$ docs per side via Block-Max WAND instead of $900\text{K}$ docs in `HashJoinExec`.

2. _One predicate is selective (`HashJoinExec` wins)_:
   - $D_L = D_R = 1,000,000$, $M_L = 15,000$, $M_R = 500,000$, $K = 10$.
   - $W_{\text{est}} \approx 6,324 > 0.1 \times 15,000 = 1,500$.
   - `RankJoinRule` declines; `HashJoinExec` builds on the $15,000$ rows of $L$ and pushes a `TermSetQuery` into $R$.

3. _Small dimension table joining a large fact table (`HashJoinExec` wins)_:
   - $D_L = M_L = 10,000$, $D_R = 10,000,000$, $M_R = 2,000,000$, $K = 10$.
   - $W_{\text{est}} = 2 \cdot \sqrt{10 \times 10,000,000} = 20,000 > 0.1 \times 10,000 = 1,000$.
   - `RankJoinRule` declines because hashing the $10,000$-row table $L$ in one pass is cheaper than pulling $20,000$ rows from $R$.

</details>

---

## 4. Monotonicity Check & Optimizer Rule Ordering

### 4.1 Recognizing Monotonic Scoring Expressions

`RankJoinRule` checks that the `SortExec` expression is monotonic in each input by walking the `PhysicalExpr` tree and tracking whether each subexpression is `Increasing` or `Decreasing`:

| Expression Node                            | Allowed When                                                                                  | Effect on Child Direction                                                                     |
| :----------------------------------------- | :-------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------- |
| Leaf column / `pdb.score` / distance       | Belongs to one join input and maps to a Tantivy `OrderByInfo` (or child `RankJoinExec` score) | `Increasing` $\rightarrow$ `DESC`, `Decreasing` $\rightarrow$ `ASC` (when top sort is `DESC`) |
| `a + b`                                    | Always                                                                                        | Preserves direction of both `a` and `b`                                                       |
| `a - b`                                    | Always                                                                                        | Preserves `a`, flips `b`                                                                      |
| `c * a` or `a / c` (constant $c \neq 0$)   | Always                                                                                        | Preserves if $c > 0$, flips if $c < 0$                                                        |
| `a * b`                                    | Both `a` and `b` are non-negative ($\ge 0$, e.g. `pdb.score`, distance, `GREATEST(x, 0)`)     | Preserves direction of both `a` and `b`                                                       |
| `GREATEST(a, b, ...)` / `LEAST(a, b, ...)` | Always                                                                                        | Preserves direction of all arguments                                                          |
| `ln(a)`, `sqrt(a)`, `exp(a)`               | Always on valid domain                                                                        | Preserves direction of `a`                                                                    |

### 4.2 Rule Ordering in `build_base_session` (`pg_search/src/postgres/customscan/joinscan/scan_state.rs`)

1. `RangePartitioningRule` & `LateMaterializationRule`
2. `RangeCoPartitionedJoinRule` (flips eligible `CollectLeft` hash joins to `Partitioned` when range co-partitioned)
3. `RankJoinRule` (rewrites eligible two-sided scored `HashJoinExec`s into `RankJoinExec`, preserving `HashJoinExec::mode()`)
4. `IterativeLookupJoinRule` (when implemented: rewrites remaining single-sided or lopsided scored `HashJoinExec`s into `IterativeLookupJoinExec`)
5. `EnsureRequirements` (`EnforceDistribution` and `EnforceSorting`, which insert `SortPreservingMergeExec` / `RepartitionExec` for `RankJoinExec` inputs when needed)
6. `DeferredPlacementRule` & `DeferredAggregateRule` (keep string decoding deferred to the final `K` rows above the join)
7. `SegmentedTopKRule` (applies to remaining One-Pass `HashJoinExec` plans)
8. `DistributedPhysicalOptimizerRule` (slices the plan into MPP stages)
