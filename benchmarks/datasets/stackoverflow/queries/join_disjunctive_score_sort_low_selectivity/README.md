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
