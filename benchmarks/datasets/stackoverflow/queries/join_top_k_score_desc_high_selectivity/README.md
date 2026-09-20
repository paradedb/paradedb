# Top-k by score, restricted by a join

- **Join**: stackoverflow_posts -> users
- **Description**: This is a join that is blockmax-wand-eligible and uses the scores
  from a single table to drive the sort

## Query Info (statistics from 100k dataset; larger datasets may have different values):

- 'the' chosen deliberately to introduce a very large pool of candidates for the
  topk (12.5K in the 100K dataset), to ensure we see the effect of the threshold
  tightening.
