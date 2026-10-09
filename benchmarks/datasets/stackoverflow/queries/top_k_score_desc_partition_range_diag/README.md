# Diagnostics for the partitioned `id` range filter (Single Table, BM25)

Single-worker pairs on `stackoverflow_posts` (partitioned) and `stackoverflow_posts_unpartitioned`,
each changing one thing from `top_k_score_desc_partition_range_filter`:

- `count_*`: `count(*)` instead of Top K by score (matching and filtering without scoring).
- `narrow_*`: `id BETWEEN 20000000 AND 20500000` (covers partitioned segments partly, not fully).
- `open_*`: `id >= 20000000` (covers more segments).
- `javascript_*`: a more selective term with the same range.
- `estimates_*`: the original query with `paradedb.explain_recursive_estimates` on, for the plan output.
