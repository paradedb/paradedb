# Disjunctive Search across Normalized Hierarchy, Low Selectivity (Score Sort)

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

- 'code' selectivity:
  - comments.text ||| 'code': ~2.88M matches (~9.94%, ~2.83M joined rows)
  - stackoverflow_posts.title ||| 'code': ~147K matches (~0.74%, fanning out to ~293K joined rows)
  - users.about_me ||| 'code': ~29K matches (~1.27%, fanning out to ~1.85M joined rows)
  - Combined disjunctive (`OR`) joined selectivity: ~16.58% (~4.72M / 28.45M joined rows).
    Under database convention, this wide filter represents low selectivity (unselective filter with a large candidate pool).

## Native PostgreSQL Variant

A naive PostgreSQL query with a cross-table `OR` filter (`u.about_me @@ q OR p.title @@ q OR c.text @@ q`) cannot use per-table GIN indexes: PostgreSQL executes a full 3-table `Parallel Seq Scan` + `Parallel Hash Join` across all 28.45M joined rows (~120s) and suffers from `NULL` score propagation when `users.about_me` (81.5% `NULL`) or `stackoverflow_posts.title` (61.5% `NULL`) is `NULL`. We instead benchmark a GIN + B-Tree rewrite (`enable_indexonlyscan = off` is set so ParadeDB's `bm25` index-only scan cost estimate does not hijack unpredicated B-Tree join branches):

- **`postgres_union_agg.sql`: Canonical 3-branch `UNION ALL` + `GROUP BY` (~73.0 s, 100% exact recall)**:
  - Rewrites the cross-table `OR` into a 3-branch `UNION ALL` where each branch drives from one table's GIN index and joins the other two tables via B-Tree foreign-key/primary-key indexes.
  - Each branch computes `ts_rank()` only on its GIN-filtered driving table inside an optimization-fenced subquery (`WITH _ AS (SELECT 1)`). This fence prevents `pull_up_subqueries` from flattening the branch, enabling `Parallel Append` and avoiding redundant `to_tsvector()` evaluations across the `1:N` join fanout.
  - Because `(users.id, stackoverflow_posts.id, comments.id)` is a unique 3-tuple per branch, `GROUP BY u_id, p_id, c_id` with `SUM(part_score)` deduplicates and sums the partial scores of multi-table matches while avoiding `NULL` score propagation.
  - **Tradeoff**: Guarantees 100% exact recall across all match combinations, but at low selectivity must execute 2.79M B-Tree probes into `stackoverflow_posts_pkey` (for the 2.97M matching comments), expand 36K matching users into 1.85M joined rows, and materialize + hash-aggregate ~4.72M joined triples before sorting.
