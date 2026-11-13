# Permissioned Search, Low Selectivity (Score Sort)

- **Join**: Single Feature (BM25 Score)
- **Description**: A Full Text Search (BM25) drives the ranking, restricted by a low-selectivity (broad) foreign filter on `users` (`u.reputation > 100`). Because the foreign filter passes `RankJoinRule` Gate 3 (`>= 2.5%` unscored selectivity and `>= 10,000` rows), `rank_join` plans `RankJoinExec` (`NRJN`) and probes `users` via Random Access (`RA`).

## Query Info (statistics from 20M dataset; 100k dataset has ~82% on `reputation > 100` due to active-user sampling skew):

- `stackoverflow_posts.title ||| 'how using get create'`: ~2.03M matches (~10.1%)
- `users.reputation > 100`: 791,228 / 2,261,291 users (~35.0%), yielding 1,463,595 joined post matches
- Under database convention, this broad filter represents low selectivity (large candidate pool).

## Native PostgreSQL Baseline Omission

`postgres.sql` is omitted across the `join_permissioned_search_*` tiers:

- Equivalent disjunctive `to_tsvector('english', p.title) @@ to_tsquery('english', 'how | using | get | create')` matches 1,286,163 posts and requires a `Parallel Bitmap Heap Scan` + `ts_rank()` evaluation across all 1.29M heap tuples before sorting (~26.7 s per iteration even under selective `users` filters, because PostgreSQL drives from the GIN scan on `stackoverflow_posts`).
- Note: an earlier `postgres.sql` used `plainto_tsquery('english', 'how using get create')`, which PostgreSQL parses as the conjunction `'use' & 'get' & 'creat'` (496 posts instead of 1.29M), benchmarking a different query.
