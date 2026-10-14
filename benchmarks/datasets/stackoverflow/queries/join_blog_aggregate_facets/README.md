# Blog Query 3: Relational Terms Aggregation over Fan-Out Join

- **Join**: `stackoverflow_posts` → `badges`
- **Description**: Computes the top 10 author badges (`GROUP BY b.name ORDER BY badge_count DESC LIMIT 10`) across posts matching `p.title ||| 'python'`. Uses `p.title` so that the native Postgres baseline (`postgres.sql`) can use `stackoverflow_posts_title_fts_idx` (`GIN` on `to_tsvector('english', title)`).
- **Highlights**:
  - Relational equivalent of an Elasticsearch Terms Aggregation facet over a high-fan-out 1-to-many relationship.
  - Groups directly on compact dictionary term ordinals (`AggregateExec: mode=Partial`), decodes strings only after partial aggregation (`TantivyDecodeExec: decode=[name]`), pushes `SortExec: TopK(fetch=10)` into the distributed aggregation plan, and executes with zero heap access when the visibility map is all-visible.
