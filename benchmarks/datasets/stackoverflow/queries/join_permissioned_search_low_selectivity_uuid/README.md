# Permissioned Search, Low Selectivity (Score Sort, UUID Join Key)

- Join: Single Feature (BM25 Score) over unpartitioned full-entropy `UUID` keys (`stackoverflow_posts.owner_user_uuid = users.user_uuid`)
- Description: Mirrors `join_permissioned_search_low_selectivity` with deterministic `md5(...)::uuid` join keys. Because `u.reputation > 100` matches 35.0% of `users` on 20M, `rank_join.sql` plans `RankJoinExec` (`NRJN`) and probes `users` via Random Access (`RA`) against the high-cardinality `users.user_uuid` FST term dictionary, while `hash_partitioned.sql` benchmarks large-cardinality UUID hash join execution at default broadcast thresholds.

## Query Info (statistics from 20M dataset; 100k dataset has ~82% on `reputation > 100` due to active-user sampling skew):

- `stackoverflow_posts.title ||| 'how using get create'`: ~2.03M matches (~10.1%)
- `users.reputation > 100`: 791,228 / 2,261,291 users (~35.0%), yielding 1,463,595 joined post matches
- Under database convention, this broad filter represents low selectivity (large candidate pool).

## Variant Omissions

- `range_partitioned.sql` and `rank_join_range_partitioned.sql` are omitted because the BM25 indexes are not range-partitioned on the UUID columns.
- See `../join_permissioned_search_low_selectivity/README.md` for why `postgres.sql` is omitted across the `join_permissioned_search_*` tiers.
