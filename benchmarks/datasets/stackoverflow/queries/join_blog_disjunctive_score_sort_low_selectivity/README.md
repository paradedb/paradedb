# Blog Query 2 (Low Selectivity): Universal Disjunctive Search Across 3-Table Hierarchy (Score Sort)

- **Join**: `users` → `stackoverflow_posts` → `comments`
- **Description**: Disjunctive (`OR`) search for `'code'` across all three normalized tables (`users.about_me ||| 'code' OR stackoverflow_posts.title ||| 'code' OR comments.text ||| 'code'`), ranked by composite relevance score (`pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id)`), projecting real display columns (`users.display_name`, `stackoverflow_posts.title`, `comments.score`).
- **Highlights**:
  - Relational equivalent of an "all-fields" search on a denormalized Elasticsearch document.
  - Exercises single-pass predicate tagging, lazy `ctid` resolution (`VisibilityFilterExec`), and deferred string decoding (`TantivyDecodeExec`).

## Query Info (statistics from 20m dataset):

- `'code'` selectivity:
  - `comments.text ||| 'code'`: ~2.88M matches (~9.94%, ~2.83M joined rows)
  - `stackoverflow_posts.title ||| 'code'`: ~147K matches (~0.74%, fanning out to ~293K joined rows)
  - `users.about_me ||| 'code'`: ~29K matches (~1.27%, fanning out to ~1.85M joined rows)
  - Combined disjunctive (`OR`) joined selectivity: ~16.58% (~4.72M / 28.45M joined rows).
    Under database convention, this wide filter represents low selectivity (unselective filter with a large candidate pool).

## Native PostgreSQL Variants

See `join_blog_disjunctive_score_sort_medium_selectivity/README.md` for the full explanation of the three native PostgreSQL baselines (`postgres.sql`, `postgres_union_agg.sql`, and `postgres_early_topk.sql`), the `1 : N : M` hierarchy decomposition, and their recall/performance tradeoffs. Tier-specific behavior at low selectivity (`'code'`):

- **`postgres_union_agg.sql` (100% exact recall)**:
  - Must expand 36K matching users into 1.85M joined rows, join 2.97M matching comments against `stackoverflow_posts` and `users` (with the planner switching several join legs from B-Tree `Nested Loop` joins to `Parallel Hash Join` + `Parallel Seq Scan` across `stackoverflow_posts`, `comments`, and `users`), and hash-aggregate ~4.72M joined triples before sorting (~14.2s in CI).
- **`postgres_early_topk.sql` (20/20 recall on this dataset, up to 1 `float4` ULP tie-break order)**:
  - Branch 1 (`users`) halts after expanding the top 2 users (avoiding the 1.85M-row fanout across 36K matching users); Branch 2 (`stackoverflow_posts`) halts after inspecting ~28 posts; Branch 3 (`comments`) probes `stackoverflow_posts_pkey` / `users_pkey` only 20 times instead of 2.97M times.
  - The remaining ~16.5s CI runtime is dominated by scanning and evaluating `to_tsvector()` + `ts_rank()` across the 2.97M matching `comments` heap rows (768K heap pages = 6.0 GB).
  - All 20 top results are 3-way `(1, 1, 1)` matches (with 1 tied `(1, 1, 1)` row at the 20th-place tie boundary `score = 0.21952924` differing from `postgres_union_agg.sql` by 1 `float4` ULP due to non-associative `float4` addition order).
