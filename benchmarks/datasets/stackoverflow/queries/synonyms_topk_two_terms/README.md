# Synonyms, Two Terms (Score Sort)

- **Description**: Top 10 by BM25 score for two terms, one of which a ten-word synonym set expands. With expansion on, the expanded term becomes a synonym group in the `Should` union, which Block-WAND prunes with a wider bound than a term. The `no_synonyms` variant turns expansion off with `paradedb.enable_synonyms` and is the baseline.

## Query Info (statistics from 1m dataset; larger datasets may have different values):

- 'python' selectivity on stackoverflow_posts.body: ~2.1%
- 'exception' selectivity on stackoverflow_posts.body: ~3.2%

The `paradedb.enable_synonyms` setting does not exist on refs before the feature, so both variants fail there.
