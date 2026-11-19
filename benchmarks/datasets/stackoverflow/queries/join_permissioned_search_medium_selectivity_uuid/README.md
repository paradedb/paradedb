# Permissioned Search, Medium Selectivity (Score Sort, UUID Join Key)

- Join: Single Feature (BM25 Score) over unpartitioned full-entropy `UUID` keys (`stackoverflow_posts.owner_user_uuid = users.user_uuid`)
- Description: Mirrors `join_permissioned_search_medium_selectivity` with deterministic `md5(...)::uuid` join keys. On the 20M dataset, `u.about_me ||| 'software'` matches 68,087 users (`3.01%`), below `paradedb.rank_join_unscored_selectivity_threshold` (`0.08`) and above `paradedb.hash_join_inlist_pushdown_max_distinct_values`, exercising medium-cardinality UUID join execution (`HashTableLookupExpr` / dynamic filter pushdown and partitioned hash join behavior at default broadcast thresholds).

## Query Info (statistics from 20M dataset):

- `stackoverflow_posts.title ||| 'how using get create'`: ~2.03M matches (~10.1%)
- `users.about_me ||| 'software'`: 68,087 / 2,261,291 users (~3.01%), yielding 112,342 joined post matches

## Variant Omissions

- `range_partitioned.sql` and `rank_join_range_partitioned.sql` are omitted because the BM25 indexes are not range-partitioned on the UUID columns.
- See `../join_permissioned_search_low_selectivity/README.md` for why `postgres.sql` is omitted across the `join_permissioned_search_*` tiers.
