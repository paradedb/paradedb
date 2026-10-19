# TopK Score DESC with a Heap Filter and a NOT (Single Table, BM25)

- **Description**: BM25 score ordered DESC and LIMIT 10 with a heap filter on the title length and a NOT over a title search.
  The heap filter is a condition the index cannot answer, so it is checked against the table row.
  This tests Block-Max WAND under a heap filter: only the documents that can still enter the
  top 10 should be checked.

## Query Info:

- 'use' selectivity on stackoverflow_posts.body: ~27.71% (5.55M matches, 20M dataset).
- The selectivity of the heap filter has not been measured.
