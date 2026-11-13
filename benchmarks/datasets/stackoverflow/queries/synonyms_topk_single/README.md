# Synonyms, Single Term (Score Sort)

- **Description**: Top 10 by BM25 score for one term that a ten-word rule expands. The `no_synonyms` variant is the same match without the `pdb.expand` subquery, so the two variants differ only in the expansion. The rule words are common on purpose, so this is the costly case. The `synonyms_serial` variant is the `synonyms` query with no parallel workers, so the two show what the workers add.

## Query Info (statistics from 1m dataset; larger datasets may have different values):

- 'exception' selectivity on stackoverflow_posts.body: ~3.2%
- the ten rule words together: ~20% of bodies

On a ref before the feature the `synonyms` variant fails, which a backfill run allows.
