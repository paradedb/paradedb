# Disjunctive Search Scalar COUNT(\*) Aggregate on JOIN

- **Join**: users → stackoverflow_posts → comments
- **Description**: Scalar COUNT(\*) aggregate over a disjunctive (OR) full-text search across normalized relational tables.
  This pattern represents the normalized relational equivalent of searching across
  a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
  users can execute broad full-text searches across normalized table boundaries
  without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.

## Query Info (statistics from 20m dataset):

- 'python' selectivity:
  - comments.text ||| 'python': ~220k matches
  - stackoverflow_posts.title ||| 'python': ~170k matches
  - users.about_me ||| 'python': ~24k matches
