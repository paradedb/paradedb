# Range filter score handling, Partitioned vs Unpartitioned (Single Table, BM25)

Tests whether issue #6674 (an unboosted range adds 1.0 to the score only on segments it partly
covers) explains the slow `id` range Top K on the partitioned index. Single-worker pairs, same
search term and range as `top_k_score_desc_partition_range_filter`, written as one `pdb.range`
clause:

- `single_*`: unboosted, so the range still adds 1.0 or 0.0 depending on the segment.
- `boost2_*`: `::pdb.boost(2)`, so the range adds 2.0 on every segment.
- `const0_*`: `::pdb.const(0)`, so the range adds 0.0 on every segment.
