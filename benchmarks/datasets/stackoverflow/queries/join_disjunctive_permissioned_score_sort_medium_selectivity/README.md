# Permissioned Disjunctive Search, Medium Selectivity (Score Sort)

- **Join**: stackoverflow_posts -> comments
- **Description**: Disjunctive (`OR`) full-text search across `stackoverflow_posts` and `comments` ordered by combined BM25 relevance (`pdb.score(p.id) + pdb.score(c.id) DESC, c.id ASC`), restricted by an `InitPlan`-parameterized base filter on `stackoverflow_posts` (`p.view_count >= (SELECT 42) AND p.view_count < (SELECT 43)`). Models a multi-table search under a row-level security / tenant filter that exceeds both `paradedb.rank_join_min_rows` (`10,000`) and `paradedb.hash_join_inlist_pushdown_max_distinct_values` (`20,000`), while staying selective (`0.20% < paradedb.rank_join_unscored_selectivity_threshold = 0.08`) where parallel `HashJoinExec` outperforms `RankJoinExec` (which otherwise amplifies `SA` misses on `comments`).

## Query Info (statistics from 20M dataset):

- `stackoverflow_posts.title ||| 'welcome'`: 113 matches
- `comments.text ||| 'welcome'`: 81,326 matches (~0.28%)
- `stackoverflow_posts.view_count` in `[42, 43)`: 39,575 / 20,020,129 posts (~0.20%, spanning all 32 `id,owner_user_id`-partitioned segments), yielding 235 joined matches

## Native PostgreSQL Variant

`postgres_union_agg.sql` uses the canonical 2-branch `UNION ALL` + `GROUP BY` GIN + B-Tree rewrite (`SET enable_indexonlyscan TO off`); see `../join_disjunctive_score_sort_low_selectivity/README.md` for details.

## Rank Join Variants

`hash_partitioned.sql` and `range_partitioned.sql` set `paradedb.enable_rank_join TO off` to benchmark the `HashJoinExec` baseline, while `rank_join.sql` and `rank_join_range_partitioned.sql` set `paradedb.enable_rank_join TO on` (where `ScanMode::Disjunctive` `base_query` selectivity gating declines `RankJoinExec` at `0.20% < paradedb.rank_join_unscored_selectivity_threshold = 0.08`, keeping both variants on `HashJoinExec`).
