# Top-k by score, restricted by a join (low selectivity)

- **Join**: stackoverflow_posts -> users
- **Description**: This is a join that is blockmax-wand-eligible and uses the scores
  from a single table to drive the sort

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'code developer' chosen deliberately to introduce a very large pool of candidates for the
  topk (~13.5% selectivity on the 20M dataset, ~2.7M joined matches; ~13.5K in the 100K dataset),
  to ensure we see the effect of the threshold tightening. Under database convention, this large
  candidate pool represents low selectivity (unselective filter).
