# Blog Query 4: Disjunctive Relational Terms Aggregation over 1:N Join

- **Join**: `users` → `stackoverflow_posts`
- **Description**: Computes the top 10 tag combinations (`GROUP BY p.tags ORDER BY post_count DESC LIMIT 10`) across posts where either the author's bio matches `'python'` (`u.about_me ||| 'python'`) or the post title matches `'python'` (`p.title ||| 'python'`).
- **Highlights**:
  - Demonstrates disjunctive predicate tagging (`tag_0`) combined with co-partitioned `HashJoinExec` (`filter = __u_0_tag_0 OR __p_1_tag_0`) and post-join selective late materialization (`TantivyFetchExec: fetch=[tags]`).
  - Rather than fetching `p.tags` for all 20M posts during the scan, `TantivyFetchExec` sits above the join filter and fetches `p.tags` dictionary ordinals only for the ~812K posts that survive the join filter, followed by partial aggregation on 8-byte dictionary ordinals (`AggregateExec: mode=Partial`) and string decoding (`TantivyDecodeExec: decode=[tags]`).
  - Includes both the natural cross-table `OR` PostgreSQL baseline (`postgres.sql`) and a 2-branch `UNION` rewrite (`postgres_union.sql`) that uses `users_about_me_fts_idx` and `stackoverflow_posts_title_fts_idx` before deduplicating `p.id` and grouping by `tags`.
