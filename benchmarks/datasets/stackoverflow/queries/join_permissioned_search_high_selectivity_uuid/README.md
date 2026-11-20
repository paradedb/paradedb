# Permissioned Search, High Selectivity (Score Sort, UUID Join Key)

- Join: Single Feature (BM25 Score) over unpartitioned full-entropy `UUID` keys (`stackoverflow_posts.owner_user_uuid = users.user_uuid`)
- Description: Mirrors `join_permissioned_search_high_selectivity` with deterministic `md5(...)::uuid` join keys to exercise string/UUID dynamic filter pushdown (`TermSetQuery` / `StrColumn` dictionary lookups) rather than the integer fast-field path. On the 20M dataset, `u.about_me ||| 'security'` matches 4,577 users (`0.20%`), staying below both `paradedb.rank_join_min_rows` and `paradedb.rank_join_unscored_selectivity_threshold`, so `RankJoinRule` rewrites the join into `HashJoinExec(fetch=10)` over a `lazy_sort` probe scan with dynamic `InList` pushdown.

## Query Info (statistics from 20M dataset):

- `stackoverflow_posts.title ||| 'how using get create'`: ~2.03M matches (~10.1%)
- `users.about_me ||| 'security'`: 4,577 / 2,261,291 users (~0.20%), yielding 6,506 joined post matches
- Under database convention, this narrow filter represents high selectivity (filters out ~99.8% of `users`).

## Variant Omissions

- `range_partitioned.sql` and `rank_join_range_partitioned.sql` are omitted because the BM25 indexes are not range-partitioned on the UUID columns.
- `rank_join.sql` is omitted because `RankJoinRule` intentionally declines `RankJoinExec` at `0.20%` unscored selectivity and rewrites `hash_partitioned.sql` directly into a `lazy_sort` probe join.
- See `../join_permissioned_search_low_selectivity/README.md` for why `postgres.sql` is omitted across the `join_permissioned_search_*` tiers.
