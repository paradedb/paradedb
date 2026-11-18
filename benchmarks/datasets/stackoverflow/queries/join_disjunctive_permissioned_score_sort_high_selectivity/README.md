# Permissioned Disjunctive Search, High Selectivity (Score Sort)

- **Join**: stackoverflow_posts -> comments
- **Description**: Disjunctive (`OR`) full-text search across `stackoverflow_posts` and `comments` ordered by combined BM25 relevance (`pdb.score(p.id) + pdb.score(c.id) DESC, c.id ASC`), restricted by a high-selectivity `InitPlan`-parameterized base filter on `stackoverflow_posts` (`p.score >= (SELECT 50) AND p.score < (SELECT 51)`). Models a multi-table search under a selective row-level security / tenant policy where the filtered base table matches `< paradedb.rank_join_min_rows` (10,000) rows and the joined result has fewer rows than `LIMIT 20` (so an unbounded rank join cannot early-terminate by score threshold alone and must instead rely on post-`InitPlan` row estimation or selective lookup pruning).

## Query Info (statistics from 20M dataset):

- `stackoverflow_posts.title ||| 'welcome'`: 113 matches
- `comments.text ||| 'welcome'`: 81,326 matches (~0.28%)
- `stackoverflow_posts.score` in `[50, 51)`: 2,043 / 20,020,129 posts (~0.010%, spanning all 32 `id,owner_user_id`-partitioned segments), yielding 2 joined matches (`< LIMIT 20`)

## Native PostgreSQL Variant

`postgres_union_agg.sql` uses the canonical 2-branch `UNION ALL` + `GROUP BY` GIN + B-Tree rewrite (`SET enable_indexonlyscan TO off`); see `../join_disjunctive_score_sort_low_selectivity/README.md` for details.

## Rank Join Variants

`hash_partitioned.sql` and `range_partitioned.sql` set `paradedb.enable_rank_join TO off` to benchmark the `HashJoinExec` baseline, while `rank_join.sql` and `rank_join_range_partitioned.sql` set `paradedb.enable_rank_join TO on` (where post-`InitPlan` row estimation and `ScanMode::Disjunctive` `base_query` selectivity gating decline `RankJoinExec` at `2,043` matching posts / `0.010%`, keeping both variants on `HashJoinExec`).
