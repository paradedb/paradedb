# Permissioned Search (Score Sort)

- **Join**: Single Feature (BM25 Score)
- **Description**: A Full Text Search (BM25) drives the ranking, but the result set must be restricted by a JOIN (e.g., checking permissions or document isolation). The score comes purely from the stackoverflow_posts table, but the validity of the row depends on the users table.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'how using get create' selectivity on stackoverflow_posts.title: ~10%
- reputation > 100 selectivity on users.reputation: ~82% (active users are overrepresented at smaller sizes; likely lower for larger datasets)
