# Foreign Filter, Local Sort

- **Join**: Single Feature (Fast Field)
- **Description**: A standard join where the user filters by a property of the parent table (users), but sorts by a deterministic "fast field" on the child table (stackoverflow_posts). The challenge is balancing the selectivity of the foreign filter against the sort order of the local table.

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- reputation > 100 selectivity on users.reputation: ~82% (active users are overrepresented at smaller sizes; likely lower for larger datasets)
- 'error' selectivity on stackoverflow_posts.title: ~1%
