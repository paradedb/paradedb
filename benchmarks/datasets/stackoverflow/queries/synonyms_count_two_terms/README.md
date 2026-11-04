# Synonyms, Two Terms (Count)

- **Description**: `count(*)` for two terms, one of which a ten-word synonym set expands. No scoring, so the cost is the extra postings only. The `no_synonyms` variant turns expansion off with `paradedb.enable_synonyms` and is the baseline.

## Query Info (statistics from 1m dataset; larger datasets may have different values):

- 'python' selectivity on stackoverflow_posts.body: ~2.1%
- 'exception' selectivity on stackoverflow_posts.body: ~3.2%

The `paradedb.enable_synonyms` setting does not exist on refs before the feature, so both variants fail there.
