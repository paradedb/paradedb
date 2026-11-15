# Permissioned Disjunctive Search, Low Selectivity (Score Sort)

- **Join**: stackoverflow_posts -> comments
- **Description**: Disjunctive (`OR`) full-text search across `stackoverflow_posts` and `comments` ordered by combined BM25 relevance (`pdb.score(p.id) + pdb.score(c.id) DESC, c.id ASC`), restricted by a low-selectivity `InitPlan`-parameterized base filter on `stackoverflow_posts` (`p.view_count >= (SELECT 42) AND p.view_count < (SELECT 150)`). Models a multi-table search under a broad row-level security / tenant filter that passes `RankJoinRule` Gate 3 (`>= paradedb.rank_join_min_rows = 10,000` across `100K`, `1M`, and `20M` datasets, and `11.23% >= paradedb.rank_join_unscored_selectivity_threshold = 0.025`), where `RankJoinExec` (`ScanMode::Disjunctive` with `RaBaseFilter::Filter` and per-leaf `SA` window scaling) early-terminates without scanning all 2.25M filtered posts and 59M comments.

## Query Info (statistics from 20M dataset):

- `stackoverflow_posts.title ||| 'welcome'`: 113 matches
- `comments.text ||| 'welcome'`: 81,326 matches (~0.28%)
- `stackoverflow_posts.view_count` in `[42, 150)`: 2,247,778 / 20,020,129 posts (~11.23%, spanning all 32 `id,owner_user_id`-partitioned segments), yielding 11,986 joined matches

## Native PostgreSQL Variant

`postgres_union_agg.sql` uses the canonical 2-branch `UNION ALL` + `GROUP BY` GIN + B-Tree rewrite (`SET enable_indexonlyscan TO off`); see `../join_disjunctive_score_sort_low_selectivity/README.md` for details.
