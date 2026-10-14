# Blog Query 2 (High Selectivity): Universal Disjunctive Search Across 3-Table Hierarchy (Score Sort)

- **Join**: `users` → `stackoverflow_posts` → `comments`
- **Description**: Disjunctive (`OR`) search for `'ocaml'` across all three normalized tables (`users.about_me ||| 'ocaml' OR stackoverflow_posts.title ||| 'ocaml' OR comments.text ||| 'ocaml'`), ranked by composite relevance score (`pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id)`), projecting real display columns (`users.display_name`, `stackoverflow_posts.title`, `comments.score`).
- **Highlights**:
  - Relational equivalent of an "all-fields" search on a denormalized Elasticsearch document.
  - Exercises single-pass predicate tagging, lazy `ctid` resolution (`VisibilityFilterExec`), and deferred string decoding (`TantivyDecodeExec`).

## Query Info (statistics from 20m dataset):

- `'ocaml'` selectivity:
  - `comments.text ||| 'ocaml'`: ~2.2K matches (~0.008%, ~2.2K joined rows)
  - `stackoverflow_posts.title ||| 'ocaml'`: ~1.6K matches (~0.008%, fanning out to ~1.8K joined rows)
  - `users.about_me ||| 'ocaml'`: ~137 matches (~0.006%, fanning out to ~14.0K joined rows)
  - Combined disjunctive (`OR`) joined selectivity: ~0.06% (~17.0K / 28.45M joined rows).
    Under database convention, this narrow filter represents high selectivity (filters out ~99.94% of joined rows).

## Native PostgreSQL Variants

A naive PostgreSQL query with a cross-table `OR` filter (`u.about_me @@ q OR p.title @@ q OR c.text @@ q`) cannot use per-table GIN indexes: PostgreSQL executes a full 3-table `Parallel Seq Scan` + `Parallel Hash Join` across all 28.45M joined rows (~120s) and suffers from `NULL` score propagation when `users.about_me` (81.5% `NULL`) or `stackoverflow_posts.title` (61.5% `NULL`) is `NULL` (benchmarked once in `join_blog_disjunctive_score_sort_medium_selectivity/postgres.sql` to avoid adding 40 minutes to CI). We also benchmark two GIN + B-Tree rewrites:

- **`postgres_union_agg.sql` (100% exact recall)**:
  - Rewrites the cross-table `OR` into a 3-branch `UNION ALL` where each branch drives from one table's GIN index, computes `ts_rank()` inside an optimization-fenced subquery (`WITH _ AS (SELECT 1)`), and joins the other two tables via B-Tree indexes.
  - Deduplicates and sums partial scores across branches via `GROUP BY u_id, display_name, p_id, title, c_id, comment_score` (`SUM(part_score)`).
  - **Tradeoff**: Guarantees 100% exact recall across all match combinations, but must materialize and hash-aggregate all qualifying joined triples (~17K rows at high selectivity) before sorting.
- **`postgres_early_topk.sql` (20/20 recall on this dataset)**:
  - Exploits `ts_rank() >= 0` and the `1:N` hierarchy (`users → stackoverflow_posts → comments`) to sort each branch's GIN-filtered driving stream first (`LIMIT 100` lookahead window per parallel worker) before expanding child B-Tree joins via `LATERAL` + `Incremental Sort ... LIMIT 20`.
  - Branch 1 (`users` only) halts after inspecting 2 users; Branch 2 (`posts LEFT JOIN users`) halts after inspecting ~15 posts; Branch 3 (`comments`) uses an in-memory `Parallel Hash Left Join` against the GIN-filtered `posts LEFT JOIN users` set and only probes `stackoverflow_posts_pkey` / `users_pkey` for the final 20 rows.
  - **Tradeoff**: Avoids `1:N` join fanout and random B-Tree probes, but relies on two assumptions that could affect recall on adversarial data: (1) the top 100 parent rows in Branches 1 and 2 join to at least 20 child rows, and (2) `(1,0,1)` triples (where `users` and `comments` match, but the intermediate bridge `stackoverflow_posts.title` does not) are scored as `max(u_score, c_score)` rather than `u_score + c_score`. On this dataset, all 20 top results are 3-way `(1,1,1)` matches, yielding 20/20 exact recall.
