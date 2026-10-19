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

A naive PostgreSQL query with a cross-table `OR` filter (`u.about_me @@ q OR p.title @@ q OR c.text @@ q`) cannot use per-table GIN indexes: PostgreSQL executes a full 3-table `Parallel Seq Scan` + `Parallel Hash Join` across all 28.45M joined rows (~120s) and suffers from `NULL` score propagation when `users.about_me` (81.5% `NULL`) or `stackoverflow_posts.title` (61.5% `NULL`) is `NULL`. We instead benchmark two GIN + B-Tree rewrites:

- **`postgres_union_agg.sql` (~73.0 s, 100% exact recall)**:
  - Rewrites the cross-table `OR` into a 3-branch `UNION ALL` where each branch drives from one table's GIN index, computes `ts_rank()` inside an optimization-fenced subquery (`WITH _ AS (SELECT 1)`), and joins the other two tables via B-Tree indexes.
  - Deduplicates and sums partial scores across branches via `GROUP BY u_id, p_id, c_id` (`SUM(part_score)`).
  - **Tradeoff**: Guarantees 100% exact recall across all match combinations, but must execute 2.79M B-Tree probes into `stackoverflow_posts_pkey` (for the 2.97M matching comments), expand 36K matching users into 1.85M joined rows, and hash-aggregate ~4.72M joined triples before sorting.
- **`postgres_early_topk.sql` (~20.2 s, 20/20 recall on this dataset)**:
  - Exploits `ts_rank() >= 0` and the `1:N` hierarchy (`users → stackoverflow_posts → comments`) to sort each branch's GIN-filtered driving stream first (`LIMIT 100` lookahead window per parallel worker) before expanding child B-Tree joins via `LATERAL` + `Incremental Sort ... LIMIT 20`.
  - Branch 1 (`users` only) halts after inspecting 2 users; Branch 2 (`posts LEFT JOIN users`) halts after inspecting 28 posts; Branch 3 (`comments`) uses an in-memory `Parallel Hash Left Join` against the GIN-filtered `posts LEFT JOIN users` set and only probes `stackoverflow_posts_pkey` / `users_pkey` 20 times instead of 2.97M times. At low selectivity, the remaining ~20s runtime is dominated by scanning and evaluating `to_tsvector()` + `ts_rank()` across the 2.97M matching `comments` heap rows (768K heap pages = 6.0 GB).
  - **Tradeoff**: Avoids `1:N` join fanout and 2.97M random B-Tree probes, but relies on two assumptions that could affect recall on adversarial data: (1) the top 100 parent rows in Branches 1 and 2 join to at least 20 child rows, and (2) `(1,0,1)` triples (where `users` and `comments` match, but the intermediate bridge `stackoverflow_posts.title` does not) are scored as `max(u_score, c_score)` rather than `u_score + c_score`. On this dataset, all 20 top results are 3-way `(1,1,1)` matches whose 3-table scores strictly dominate 2-table matches, yielding 20/20 exact recall.
