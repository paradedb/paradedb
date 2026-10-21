# Disjunctive Search across Normalized Hierarchy, Medium Selectivity (Score Sort)

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

- 'python' selectivity:
  - comments.text ||| 'python': ~221K matches (~0.76%, ~216K joined rows)
  - stackoverflow_posts.title ||| 'python': ~170K matches (~0.85%, fanning out to ~333K joined rows)
  - users.about_me ||| 'python': ~24K matches (~1.07%, fanning out to ~1.05M joined rows)
  - Combined disjunctive (`OR`) joined selectivity: ~5.27% (~1.50M / 28.45M joined rows).

## Native PostgreSQL Variants

See `../join_disjunctive_score_sort_low_selectivity/README.md` for a detailed explanation of both PostgreSQL GIN + B-Tree rewrite strategies and their recall tradeoffs. Medium-selectivity (`'python'`) behavior on the 20M dataset:

- **`postgres_union_agg.sql` (~22.4 s, 100% exact recall)**:
  - Expands the `1:N` B-Tree join fanout (e.g., 23K matching users fanning out to ~1.05M comments) and hash-aggregates ~1.53M joined triples before sorting.
- **`postgres_early_topk.sql` (~2.2 s warm / ~12.4 s cold, 20/20 recall on this dataset)**:
  - Branch 1 (`users` only) halts after inspecting 2 users (avoiding the 1.05M-row fanout); Branch 2 (`posts LEFT JOIN users`) halts after inspecting 16 posts; Branch 3 (`comments`) uses an in-memory `Parallel Hash Left Join` against the GIN-filtered `posts LEFT JOIN users` set and only probes `stackoverflow_posts_pkey` / `users_pkey` 20 times (avoiding 207K random B-Tree probes and keeping the working set within `shared_buffers = 4GB`).
  - On this dataset, 3,356 three-way `(1,1,1)` matches exist whose 3-table scores strictly dominate 2-table matches, yielding 20/20 exact recall.
