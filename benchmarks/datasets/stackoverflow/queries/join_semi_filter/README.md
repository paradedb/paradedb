# Semi-Join Filter (Term Set)

- **Join**: Single Feature (Fast Field)
- **Description**: The sort is simple (e.g., by Title), but the filter involves a "List" logic where the valid IDs are derived from a complex subquery or list. This was previously implemented using a term_set, effectively pushing a semi-join down to the search index.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'java' selectivity on users.about_me: ~5%
- 'David John Alex' selectivity on users.display_name: ~2%
- combined selectivity on users (about_me ||| 'java' AND display_name ||| 'David John Alex'): <1%
