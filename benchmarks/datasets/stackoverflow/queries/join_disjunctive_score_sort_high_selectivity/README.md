# Disjunctive Search across Normalized Hierarchy, High Selectivity (Score Sort)

- **Join**: users → stackoverflow_posts → comments
- **Description**: Disjunctive (OR) full-text search across normalized relational tables with relevance score ranking.
  This pattern represents the normalized relational equivalent of searching across
  a single flattened/denormalized document (e.g., Elasticsearch index). In ParadeDB,
  users can execute broad full-text searches across normalized table boundaries
  without paying the storage, memory, or ingestion cost of denormalizing parent/child entities.
- **Note**: We benchmark the "score sort" variant separately from the "local sort" variant to track
  the impact of scoring and top-K sorting on disjunctive join scans.

## Query Info (statistics from 20m dataset):

- 'ocaml' selectivity:
  - comments.text ||| 'ocaml': ~2.2K matches (~0.008%, ~2.2K joined rows)
  - stackoverflow_posts.title ||| 'ocaml': ~1.6K matches (~0.008%, fanning out to ~1.8K joined rows)
  - users.about_me ||| 'ocaml': ~137 matches (~0.006%, fanning out to ~14.0K joined rows)
  - Combined disjunctive (`OR`) joined selectivity: ~0.06% (~17.0K / 28.45M joined rows).
    Under database convention, this narrow filter represents high selectivity (filters out ~99.94% of joined rows).

## Native PostgreSQL Variant

See `../join_disjunctive_score_sort_low_selectivity/README.md` for a detailed explanation of the PostgreSQL GIN + B-Tree rewrite strategy. High-selectivity (`'ocaml'`) behavior on the 20M dataset:

- **`postgres_union_agg.sql` (~372 ms, 100% exact recall)**:
  - Materializes and hash-aggregates all qualifying joined triples (~17K rows at high selectivity) before sorting.
