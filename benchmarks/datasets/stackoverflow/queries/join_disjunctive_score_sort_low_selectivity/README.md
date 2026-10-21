# Disjunctive Search across Normalized Hierarchy, Low Selectivity (Score Sort)

- **Join**: users → stackoverflow_posts → comments
- **Description**: Disjunctive (OR) full-text search across normalized relational tables with relevance score ranking.
  This pattern represents the normalized relational equivalent of searching across
  a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
  users can execute broad full-text searches across normalized table boundaries
  without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.
- **Note**: We benchmark the "score sort" variant separately from the "local sort" variant to track
  the impact of scoring and top-K sorting on disjunctive join scans.
- **TODO**: Implement Block-Max WAND (BMW) scoring optimization for join scans: [https://github.com/paradedb/paradedb/issues/5301]

## Query Info (statistics from 20m dataset):

- 'code' selectivity:
  - comments.text ||| 'code': ~2.88M matches (~9.94%, ~2.83M joined rows)
  - stackoverflow_posts.title ||| 'code': ~147K matches (~0.74%, fanning out to ~293K joined rows)
  - users.about_me ||| 'code': ~29K matches (~1.27%, fanning out to ~1.85M joined rows)
  - Combined disjunctive (`OR`) joined selectivity: ~16.58% (~4.72M / 28.45M joined rows).
    Under database convention, this wide filter represents low selectivity (unselective filter with a large candidate pool).

## Native PostgreSQL Variants

A naive PostgreSQL query with a cross-table `OR` filter (`u.about_me @@ q OR p.title @@ q OR c.text @@ q`) cannot use per-table GIN indexes: PostgreSQL executes a full 3-table `Parallel Seq Scan` + `Parallel Hash Join` across all 28.45M joined rows (~120s) and suffers from `NULL` score propagation when `users.about_me` (81.5% `NULL`) or `stackoverflow_posts.title` (61.5% `NULL`) is `NULL`. We instead benchmark two GIN + B-Tree rewrites (both set `enable_indexonlyscan = off` so ParadeDB's `bm25` index-only scan cost estimate does not hijack unpredicated B-Tree join branches):

- **`postgres_union_agg.sql` — Canonical 3-branch `UNION ALL` + `GROUP BY` (~73.0 s, 100% exact recall)**:
  - Rewrites the cross-table `OR` into a 3-branch `UNION ALL` where each branch drives from one table's GIN index and joins the other two tables via B-Tree foreign-key/primary-key indexes.
  - Each branch computes `ts_rank()` only on its GIN-filtered driving table inside an optimization-fenced subquery (`WITH _ AS (SELECT 1)`). This fence prevents `pull_up_subqueries` from flattening the branch, enabling `Parallel Append` and avoiding redundant `to_tsvector()` evaluations across the `1:N` join fanout.
  - Because `(users.id, stackoverflow_posts.id, comments.id)` is a unique 3-tuple per branch, `GROUP BY u_id, p_id, c_id` with `SUM(part_score)` deduplicates and sums the partial scores of multi-table matches while avoiding `NULL` score propagation.
  - **Tradeoff**: Guarantees 100% exact recall across all match combinations, but at low selectivity must execute 2.79M B-Tree probes into `stackoverflow_posts_pkey` (for the 2.97M matching comments), expand 36K matching users into 1.85M joined rows, and materialize + hash-aggregate ~4.72M joined triples before sorting.
- **`postgres_early_topk.sql` — Hierarchy-aware early-terminating 3-branch Top-K (~20.2 s, 20/20 recall on this dataset)**:
  - Exploits `ts_rank() >= 0` and the `1:N` schema hierarchy (`users → stackoverflow_posts → comments`) so each branch sorts its GIN-filtered driving stream first (`LIMIT 100` lookahead window per parallel worker) before expanding child B-Tree joins via `LATERAL` + `Incremental Sort ... LIMIT 20`:
    1. **Branch 1** (covers `(1,0,0)` user-only matches): Sorts matching users by `u_score DESC LIMIT 100` (top-N heapsort in parallel workers) and expands `posts → comments` via `LATERAL` + `Incremental Sort LIMIT 20`. At low selectivity, halts after inspecting the top 2 users instead of fanning out across 36K users (1.85M joined rows).
    2. **Branch 2** (covers `(0,1,0)` and `(1,1,0)` post + optional user matches): `Parallel Hash Left Join`s GIN-matching posts with GIN-matching users, sorts by `up_score DESC LIMIT 100`, and expands `users + comments` via `LATERAL` + `Incremental Sort LIMIT 20`. At low selectivity, halts after inspecting ~28 posts.
    3. **Branch 3** (covers comment matches `(0,0,1)`, `(0,1,1)`, `(1,1,1)`): `Parallel Hash Left Join`s GIN-matching comments with the GIN-filtered `(posts LEFT JOIN users)` hash table on `c.post_id = p_m.id`, sorts by `upc_score DESC, c.id DESC LIMIT 100` inside each parallel worker, and only probes `stackoverflow_posts_pkey` / `users_pkey` via `LATERAL` for the final 20 rows (avoiding 2.97M random B-Tree lookups). At low selectivity, the remaining ~20s runtime is dominated by scanning and evaluating `to_tsvector()` + `ts_rank()` across the 2.97M matching `comments` heap rows (768K heap pages = 6.0 GB).
  - **Why inner `LIMIT 100` is required**: Without an inner `LIMIT` inside each branch's parallel subquery, parallel workers under `Gather Merge` sort all matching rows and fill their 64 KB `shm_mq` tuple queues when the outer `Incremental Sort + Limit 20` stops early, blocking worker exit and starving subsequent `Append` branches of parallel workers (`max_parallel_workers = 8`).
  - **Tradeoff**: Avoids `1:N` join fanout and millions of random B-Tree probes, but relies on two assumptions that could affect recall on adversarial data: (1) the top 100 parent rows in Branches 1 and 2 join to at least 20 child rows, and (2) because Branch 3's hash table only includes posts matching the query, `(1,0,1)` triples (where `users` and `comments` match, but the intermediate bridge `stackoverflow_posts.title` does not) are scored as `max(u_score, c_score)` rather than `u_score + c_score`. On this dataset, all 20 top results are 3-way `(1,1,1)` matches whose 3-table scores strictly dominate 2-table matches, yielding 20/20 exact recall across all three selectivity tiers.
