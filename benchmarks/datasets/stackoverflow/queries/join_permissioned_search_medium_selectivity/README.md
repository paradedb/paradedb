# Permissioned Search, Medium Selectivity (Score Sort)

- **Join**: Single Feature (BM25 Score)
- **Description**: A Full Text Search (BM25) drives the ranking, restricted by a medium-selectivity foreign filter on `users` (`u.about_me ||| 'software'`). On the 20M dataset, the foreign filter matches 68,087 users (`3.01%`), passing `RankJoinRule` Gate 3 (`>= paradedb.rank_join_unscored_selectivity_threshold = 0.025` and `>= paradedb.rank_join_min_rows = 10,000`) near the crossover where `InList` pushdown exceeds `paradedb.hash_join_inlist_pushdown_max_distinct_values` (`20,000`), so `rank_join*.sql` plan `RankJoinExec` (`NRJN`) with a selectivity-scaled `SA` window on `stackoverflow_posts`.

## Query Info (statistics from 20M dataset):

- `stackoverflow_posts.title ||| 'how using get create'`: ~2.03M matches (~10.1%)
- `users.about_me ||| 'software'`: 68,087 / 2,261,291 users (~3.01%), yielding 112,342 joined post matches

## Native PostgreSQL Baseline Omission

See `../join_permissioned_search_low_selectivity/README.md` for why `postgres.sql` is omitted across the `join_permissioned_search_*` tiers.
