# Synonyms, Two Terms (Count)

- **Description**: `count(*)` for two terms, one of which a ten-word rule expands. No scoring, so the cost is the expansion and the extra postings. The `no_synonyms` variant is the same match without the `pdb.expand` subquery and is the baseline. The `synonyms_serial` variant is the `synonyms` query with no parallel workers, so the two show what the workers add.

## Query Info (statistics from 1m dataset; larger datasets may have different values):

- 'exception' selectivity on stackoverflow_posts.body: ~3.2%
- 'python' selectivity on stackoverflow_posts.body: ~2.1%

On a ref before the feature the `synonyms` variant fails, which a backfill run allows.
