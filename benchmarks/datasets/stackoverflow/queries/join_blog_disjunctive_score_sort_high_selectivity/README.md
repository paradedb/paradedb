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

See `join_blog_disjunctive_score_sort_medium_selectivity/README.md` for the full explanation of the three native PostgreSQL baselines (`postgres.sql`, `postgres_union_agg.sql`, and `postgres_early_topk.sql`), the `1 : N : M` hierarchy decomposition, and their recall/performance tradeoffs. Tier-specific behavior at high selectivity (`'ocaml'`):

- **`postgres_union_agg.sql` (100% exact recall)**:
  - Must materialize and hash-aggregate all ~17K qualifying joined triples before sorting.
  - Because `users_about_me_fts_idx @@ 'ocaml'` is estimated at ~414 rows (~410 heap pages, below `min_parallel_table_scan_size = 8MB`), PostgreSQL does not generate a `Parallel Bitmap Heap Scan` path on `users` and instead falls back to a `Parallel Seq Scan` across all 2.26M `users` rows (~1.2s in CI) in order to parallelize the downstream `Nested Loop` B-Tree fanout under `Parallel Append`.
- **`postgres_early_topk.sql` (20/20 exact recall on this dataset)**:
  - Only ~3.8K total rows match across all three GIN indexes (130 users, 1,635 posts, 2,051 comments, touching ~88 MB of shared buffers across up to 6 parallel processes).
  - Branch 1 (`users`) halts after expanding the top 2 users; Branch 2 (`stackoverflow_posts`) halts after inspecting ~85 posts; Branch 3 (`comments`) sorts 2,051 matching comments in parallel and probes `stackoverflow_posts_pkey` / `users_pkey` only 20 times, finishing in ~36 ms in CI (whereas `rank_join.sql` sets `max_parallel_workers_per_gather = 0` and runs single-threaded across 96 segments in ~105 ms).
