# `join_blog_scoped_score_sort`

## Purpose

Canonical 2-table scoped relevance Top-K join used in the Join Blog Post (`join-blog/outline.md`):
filters `stackoverflow_posts` (`p.title ||| 'python error'`) and `users` (`u.about_me ||| 'software developer'`), joins on `p.owner_user_id = u.id`, and returns the top 20 posts ordered by `pdb.score(p.id) DESC, p.id DESC`.

## Variants

- `postgres.sql`: Native PostgreSQL baseline using GIN full-text indexes (`to_tsvector` / `to_tsquery`) and `ts_rank`.
- `hash_partitioned.sql`: ParadeDB `JoinScan` with `paradedb.enable_range_partitioned_join = off` and `paradedb.enable_rank_join = off` (hash-repartitioned join).
- `range_partitioned.sql`: ParadeDB `JoinScan` with `paradedb.enable_range_partitioned_join = on` and `paradedb.enable_rank_join = off` (0-shuffle co-partitioned join on `users.id = stackoverflow_posts.owner_user_id`).
- `rank_join.sql`: ParadeDB `JoinScan` with `paradedb.enable_rank_join = on` (`RankJoinExec`).
