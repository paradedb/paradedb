# Conjunctive Search across Normalized Hierarchy (Score Sort)

- **Join**: users → stackoverflow_posts → comments
- **Description**: Conjunctive (AND) full-text search across normalized relational tables with relevance score ranking.
  This pattern represents the normalized relational equivalent of searching across
  a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
  users can execute broad full-text searches across normalized table boundaries
  without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.
- **Note**: We benchmark the "score sort" variant to track the impact of scoring and top-K sorting on
  conjunctive join scans.
- **TODO**: Implement Block-Max WAND (BMW) scoring optimization for join scans: [https://github.com/paradedb/paradedb/issues/5301]

## Query Info (statistics from 20m dataset):

- selectivity:
  - comments.text ||| 'question': ~1.9m matches (~6.7%)
  - stackoverflow_posts.title ||| 'error': ~290k matches (~1.5%)
  - users.about_me ||| 'java': ~39k matches (~1.7%)
