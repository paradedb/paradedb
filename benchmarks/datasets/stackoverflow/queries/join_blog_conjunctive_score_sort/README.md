# Blog Query 1b: 3-Table Conjunctive Search by Composite Score (with Display Columns)

- **Join**: `users` → `stackoverflow_posts` → `comments`
- **Description**: Conjunctive (`AND`) full-text search across all three normalized tables ordered by composite BM25 relevance score (`pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id)`), projecting real display columns (`users.display_name`, `stackoverflow_posts.title`, `comments.score`) to exercise deferred string decoding (`TantivyDecodeExec`) and late heap materialization.
