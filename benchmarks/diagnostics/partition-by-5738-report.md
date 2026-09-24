# #5738 partitioning benchmark report

This is the local evidence report for Stu's request in
[#5738](https://github.com/paradedb/paradedb/issues/5738). It is intentionally
kept in the benchmark branch; no review comment was posted to #6288 or any
other PR page.

## Reproduction environment

The comparison was run in GitHub Actions on the `c8gd.metal-24xl` ARM64 runner
with the `ubuntu24-full-arm64` image. PostgreSQL 18 was used with the normal
benchmark cluster setup, the actual Stack Overflow 1M heap snapshot, and
`paradedb.global_target_segment_count = 48`.

The exact production commits were:

```text
before #6346: c3479d9a1fd78fd85676524a7ead9820513ad67a
merged #6346: fa8d800a282cea684f3dda48daa1ab23cdf76634
```

The benchmark definitions and diagnostic runner were from experiment commit
`7d3ad010838853b0fefecaf7ed1c8179e84d0b35`. Each layout was built once and
then reused by both binaries. The runner used ABBA ordering with 30 measured
hot samples per round; cold samples were excluded from the reported means and
medians. Correctness companions matched for all 19 queries and all six
layouts.

CI run: [35967678542](https://github.com/paradedb/paradedb/actions/runs/35967678542).

## Layouts and measured scope

The unchanged SQL files were copied byte-for-byte from the baseline commit.
Only one index option changed at a time:

| Layout | Changed index option |
| --- | --- |
| original | existing join-key partitioning |
| posts_date | add `creation_date` |
| posts_type | add `post_type_id` |
| users_reputation | add `reputation` |
| comments_score | add `score` |
| comments_name | add `user_display_name` |

The 19-query subset includes direct filters, paging predicates, aggregates,
semi-joins, permissioned joins, local-sort joins, distinct-parent joins, and
both hash- and range-partitioned variants. This is evidence for layout
selection, not yet a replacement for all 105 files in the official suite.

## Evidence for useful additional columns

### `users.reputation` and a local-sort join

The query filters `u.reputation > 100`, searches post titles, joins posts to
users, and returns the newest 20 posts.

Adding `reputation` to the users index rejected 10 of 48 user segments before
scorer creation. The layout timing changed as follows:

| Build | Original | `id,reputation` | Change |
| --- | ---: | ---: | ---: |
| Before #6346 | 20.419 ms | 17.703 ms | -13.30% |
| #6346 | 20.242 ms | 17.947 ms | -11.34% |

The DataFusion plan still spends most of its time in the join:

```text
users PgSearchScan:  about 6.0 ms
posts PgSearchScan:  about 5.1 ms
HashJoinExec:        about 12.1 ms
  build_time:        about 11.7 ms
  join_time:         about 0.4 ms
```

The 15-execution system-wide `perf stat` sample changed by this layout as
follows on the pre-#6346 build:

| Counter | Original | `id,reputation` | Change |
| --- | ---: | ---: | ---: |
| cycles | 2.599B | 2.522B | -2.96% |
| instructions | 5.429B | 5.164B | -4.88% |
| branches | 1.033B | 971M | -5.94% |
| branch misses | 19.13M | 17.45M | -8.78% |
| cache misses | 29.34M | 34.91M | +18.98% |

The merged #6346 build shows the same reduction in instructions, branches,
and branch misses, although the cache-miss counter increases. These counters
are system-wide sample totals, not function attribution; the separate call
stack run is retained for that purpose.

### `posts.post_type_id` and a direct filter

The query filters `post_type_id < 3` together with two text predicates. Adding
`post_type_id` rejected 13 of 48 segments and created scorers for 35 segments.

| Build | Original | `post_type_id` | Change |
| --- | ---: | ---: | ---: |
| Before #6346 | 3.402 ms | 2.950 ms | -13.28% |
| #6346 | 3.403 ms | 2.822 ms | -17.09% |

The merged-build `perf stat` sample changed by -1.90% in cycles, -2.55% in
instructions, -1.56% in branch misses, and -2.49% in cache misses. This is the
cleanest direct evidence that rejecting segments reduced search work.

### Date layout: evidence is mixed

The existing date-filter query rejected 7 of 48 segments after adding
`creation_date`, but its timing did not improve consistently:

| Build | Original | `creation_date` | Change |
| --- | ---: | ---: | ---: |
| Before #6346 | 3.456 ms | 3.468 ms | +0.37% |
| #6346 | 3.460 ms | 3.425 ms | -1.03% |

The query is a small Top-K scan, so the saved scorer work is comparable to
planning, heap-fetch, and early-termination costs. The evidence does not
justify claiming a general date-layout speedup.

## Evidence for layouts that do not help

### Irrelevant `reputation` on an aggregate query

The query is:

```sql
SELECT COUNT(*)
FROM stackoverflow_posts p
JOIN comments c ON p.id = c.post_id
WHERE p.body ||| 'code';
```

It does not use the users index or `reputation`, so no relevant user segments
are rejected.

| Build | Original | Users `reputation` | Change |
| --- | ---: | ---: | ---: |
| Before #6346 | 65.297 ms | 65.434 ms | +0.21% |
| #6346 | 65.450 ms | 65.396 ms | -0.08% |

The plan is dominated by posts/comments scans and a partitioned hash join:

```text
posts scan:        about 6.5–7.7 ms per worker
comments scan:     about 7.9–8.9 ms per worker
HashJoinExec:      about 9.5–11.4 ms per worker
count aggregation: about 40–54 microseconds per worker
```

The perf counters were effectively unchanged: cycles changed by +0.04% on the
pre-#6346 build and +1.24% on #6346; instructions changed by less than 0.05%
on both. Adding a column that the query does not constrain cannot reduce this
work.

### Irrelevant `score` on a semi-join

The semi-join uses `users` and `stackoverflow_posts`; it does not read the
comments index. Adding `score` to `comments.partition_by` therefore produced
zero score-based rejected segments.

The layout timing was effectively neutral on the pre-#6346 build (-0.15%).
The #6346 result changed by -5.05%, but the cross-build difference was still
9.28% on the original layout and 3.92% on the score layout, so this is not
evidence that score pruning helped this query.

The plan remains dominated by the right-semi hash join and MPP execution:

```text
RightSemi HashJoinExec: about 1.0–3.0 ms per worker
posts scan:            mostly no rows pruned by the score layout
MPP first frame:       about 23–25 ms
```

The system-wide perf layout delta was inconsistent: cycles increased 2.76%
on the pre-#6346 build but decreased 0.93% on #6346. That is why this layout
should not be recommended for this query family.

## Current advice supported by the measurements

The evidence supports adding benchmark layouts for `reputation`,
`post_type_id`, and possibly `score` only alongside queries that actually
constrain those fields. It does not support adding every visible column to a
partition key.

The measured rule is:

```text
Add a partition column when the workload frequently constrains it and the
rejected segments remove meaningful work from an expensive downstream join,
sort, or scan.
```

Projection-only, grouping-only, unrelated, or weakly selective columns must
remain controls. A speedup must be attributed to pruning only when the plan,
rejected-segment count, and CPU/row-work evidence agree; a changed join plan is
a separate confounding factor.

## Work still required for #5738

This report does not claim that the official 105-query suite has been changed.
The following are still separate implementation tasks:

1. Promote only the evidence-supported layouts into the official benchmark
   fixtures and run the complete unchanged suite at 1M and a selected larger
   size.
2. Add a substantial local changelog draft linking the partitioning guide.
3. Prepare a separate code change that changes the default value of
   `paradedb.enable_range_partitioned_join` and removes redundant `SET ... TO
   on` statements from benchmark and regression cases while preserving explicit
   `off` controls.
4. Run regression tests and compare plans with the default changed.

The call-stack profiling run for the four representative cases is CI run
[36012492217](https://github.com/paradedb/paradedb/actions/runs/36012492217).
It is still running; its `perf report` artifacts are required before making
function-level claims about the CPU bottleneck.
