# Synonyms, Single Term (Score Sort)

- **Description**: Top 10 by BM25 score for one term that a ten-word synonym set expands. The set is attached to the index by `after_create_index.sql`, and the `no_synonyms` variant turns expansion off with `paradedb.enable_synonyms`, so both variants run the same code path and differ only in the expansion. The set words are common on purpose, so this is the costly case.

## Query Info (statistics from 1m dataset; larger datasets may have different values):

- 'exception' selectivity on stackoverflow_posts.body: ~3.2%
- the ten set words together: ~20% of bodies

The `paradedb.enable_synonyms` setting does not exist on refs before the feature, so both variants fail there.
