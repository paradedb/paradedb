# Synonyms, Two Terms (Count)

- **Description**: `count(*)` for two terms, one of which a ten-word rule expands. No scoring, so the cost is the expansion and the extra postings. The `no_synonyms` variant is the same match without the `pdb.expand` subquery and is the baseline. The `synonyms_serial` variant is the `synonyms` query with no parallel workers, so the two show what the workers add. The `postgres` and `postgres_no_synonyms` variants are the same pair on native Postgres, with a GIN index on a stored `tsvector` (`stackoverflow_posts_fts`), and `ts_rewrite` for the rule. The `postgres_rewritten` variant has the rewritten `tsquery` as a literal, as when the application calls `ts_rewrite` before the search. With the subquery the planner cannot see the rewritten query, so it underestimates the rows and plans no parallel workers.

## Query Info (statistics from 1m dataset; larger datasets may have different values):

- 'exception' selectivity on stackoverflow_posts.body: ~3.2%
- 'python' selectivity on stackoverflow_posts.body: ~2.1%

The native variants use the `simple` text search configuration, which does not stem, as `pdb.unicode_words`. The two tokenizers split a few words differently, so the row counts of the two sides differ by about 1%.

On a ref before the feature the `synonyms` variant fails, which a backfill run allows.
