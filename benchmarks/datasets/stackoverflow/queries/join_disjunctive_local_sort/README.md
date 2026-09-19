# Disjunctive Search across Normalized Hierarchy (Local Sort)

- **Join**: users → stackoverflow_posts → comments
- **Description**: Disjunctive (OR) full-text search across normalized relational tables with local fast-field sort.
  This pattern represents the normalized relational equivalent of searching across
  a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
  users can execute broad full-text searches across normalized table boundaries
  without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.
- **Note**: We benchmark the "local sort" variant separately from the "score sort" variant because
  scoring requires evaluating relevance scores and sorting the top-K. Maintaining
  "local sort" as a distinct benchmark isolates pure join and filter evaluation performance.
- **TODO**: Implement Block-Max WAND (BMW) scoring optimization for join scans: [https://github.com/paradedb/paradedb/issues/5301]

## Query Info (statistics from 20m dataset):

- 'python' selectivity:
  - comments.text ||| 'python': ~220k matches
  - stackoverflow_posts.title ||| 'python': ~170k matches
  - users.about_me ||| 'python': ~24k matches
