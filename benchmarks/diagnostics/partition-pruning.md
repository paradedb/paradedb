# Partition pruning experiment

This experiment follows [Stu's benchmark suggestion](https://github.com/paradedb/paradedb/issues/5738#issuecomment-5481933811).
It runs on `6078-partition-by-benchmarks`; it does not publish benchmark baselines,
post PR comments, or send notifications.

## What is compared

- Main: `cd3451c19143d0106c6e7a2cb5656245df8d423f`.
- #6346: `ea32cde9a4e0ecc1bc7ac25a6afb51af34cdbbcc`.
- Benchmark definitions and runner: the experiment workflow commit, identical for both builds.

Main is an ancestor of the pinned PR commit. The workflow checks that the
experiment branch has no changes outside benchmarks and workflows. The normal
release builds are uninstrumented. A third library logs candidate decisions and
`DeferredScorer` creation, and is used only for separate diagnostic queries.

The actual Stack Overflow heap snapshots are restored at 1M and 20M rows. The
20M job depends on successful completion of the 1M job. Each job compares three
posts-index layouts, with the other index definitions held constant:

| Index fixture       | Posts `partition_by`             |
| ------------------- | -------------------------------- |
| `bm25`              | `id,owner_user_id`               |
| `bm25_pruning`      | `id,owner_user_id,creation_date` |
| `bm25_pruning_date` | `creation_date`                  |

The cluster target is 48 segments for every layout. The actual segment inventory
is saved: the target alone is not evidence of physical partitioning. Each layout
is built once with main; both binaries then reuse those exact physical indexes.
The experiment verifies relation identities and segment inventories across
restarts. No vacuum or index rebuild runs between the paired measurements.
Build time and index size are saved by the normal benchmark runner for each
layout; they are not a comparison of build speed between the two binaries.

## Queries and measurements

Date bounds come from the restored dataset's distribution, then remain fixed
across both builds and all layouts. The suite includes a narrow date/text count,
a date aggregate, date Top K, a full-date-range control, and a text-only control.
The matched count is checked explicitly so a zero-result query cannot masquerade
as a useful narrow-range workload. NULL date counts and a year histogram are saved.

Existing semi-join, aggregate-count join, and distinct-parent join queries run
with range partitioning explicitly enabled and disabled. Top K queries use an
additional ID ordering key to make results reproducible. This can affect their
plans compared with the unmodified benchmark; inspect the saved plans before
comparing these timings to historical CI results.

For each layout, run main, PR, PR, main, with 30 measured samples per query per
round. The existing Rust benchmark runner controls cache clearing and warm-up.
Its reported duration is server execution plus planning time. Cold samples are
saved separately and excluded from the mean and median. This yields 60 measured
samples per query per build. Full result values must agree across all builds
and layouts. Executed plans with buffers and timing are captured separately.

The trace library subsequently runs fully consumed serial projection queries.
It records actual candidate checks and deferred scorer creation, with segment
IDs and process IDs. It checks that rejected segments never reach that scorer
path, that the narrow date-only case actually rejects segments, and that the
no-date control does not activate query pruning. These diagnostics demonstrate
the serial projection path; they are not counts for every timed aggregate,
parallel task, or alternative Tantivy collector. Raw plans and traces remain
available to distinguish observed behavior from inferred attribution.

## Running and reading results

Dispatch `benchmark-pg_search-queries.yml` on the experiment branch with
`dataset=stackoverflow`, `partition_pruning_experiment=true`, and
`publish_baseline=false`. The ordinary publishing job and its notification job
are skipped. The experiment jobs only upload artifacts and write job summaries.

Artifacts `partition-pruning-date-1m` and `partition-pruning-date-20m` contain source SHAs,
library checksums, index DDL and identities, dataset distribution, resolved query
SQL, every raw timing, full results, executed plans, diagnostic patches and
traces, and `comparison.md` / `comparison.json`. A negative timing percentage
means the PR is faster. Failure artifacts retain partial results and server logs.

Compare main versus PR within each layout to measure the code change. Compare
layouts within each build to measure layout effects. Inspect join plans for
changed execution strategies when a partition column is added or removed.
Do not attribute either effect solely from a query filename or a lower runtime.

## Unchanged-query experiment: evidence for the usage guide

Select `pruning_suite=unchanged` along with `partition_pruning_experiment=true`.
This first pass runs at **1M only**. It keeps the same pinned builds, cluster
settings, actual heap snapshot, 48-segment target and same-index ABBA comparisons.
The ordinary Rust benchmark runner still performs all timed executions.

It copies 19 existing SQL files byte for byte, verifying them against pinned
main and preserving their original SET statements, filters, ordering and LIMIT.
In particular, a file named `hash_partitioned` does not explicitly disable range
joins in this revision: the artifact records the actual executed plan rather
than relabeling the query or changing its settings.

| Arm | Only index option changed | Question |
| --- | --- | --- |
| Original | None | Reference workload |
| Posts date | `id,owner_user_id,creation_date` | Does the existing date predicate benefit? |
| Posts type | `id,owner_user_id,post_type_id` | Does low-cardinality filtering help or create imbalance? |
| Users reputation | `id,reputation` | Do existing reputation filters improve joins? |
| Comments score | `post_id,score` | Does the comment predicate improve distinct-parent joins? |
| Comments name | `post_id,user_display_name` | Can raw string paging bounds prune, including runtime subquery binding? |

The queries are `filtered_highcard`, `filtered_lowcard`, all three string paging
queries, numeric high/low-cardinality top-k, count-filter and grouped-filter
aggregate scans, and the original hash/range variants of permissioned search,
foreign-filter/local-sort, distinct-parent, semi-filter and aggregate-count joins.
This is a focused pass, not all 105 original suite files. The sort/group-only and
unrelated-filter cases are controls for costs outside the intended beneficiaries.

Correctness and attribution are explicitly separate from timings:

- Timed SQL is never given an additional ordering key. Its outputs are preserved.
- Separate correctness companions add the projected unique ID as an ordering
  key for LIMIT queries. Their complete projected results, including scores, are
  compared across all builds and layouts. Unordered aggregate results are compared
  as multisets. Row counts are also checked against the original queries.
- A companion can have a different plan. Its equality does not prove that every
  tied/unordered execution of the original SQL selected equivalent rows. Original
  output artifacts remain available for that audit; row counts alone are not called
  a full correctness proof. Differences in companions are saved and fail the job
  after collecting the other measurements, rather than being silently ignored.
- The trace build executes the original SQL separately, with its original settings
  and LIMIT. Logs cover candidate checks and DeferredScorer creation only. A query
  emitting no trace events is not evidence that no other pruning mechanism ran.
- `comparison.json` measures main versus PR within each layout;
  `layout-comparison.json` measures each layout versus the original within each
  build. They answer different questions. All raw samples and plans are preserved.

Artifact: `partition-pruning-unchanged-1m`. This experiment only uploads artifacts
and a job summary; it does not update PRs, published baselines or Slack.

### Follow-up experiments, not yet run by this mode

Promote a claim in the guide to a measured recommendation only after its relevant
experiment is complete and the plans, results and repeated timings support it:

| Claim | Next controlled experiment |
| --- | --- |
| A chosen layout improves the real workload | Extend promising arms to all unchanged queries, then confirm at 20M |
| A selective filter pays for proof overhead | Sweep actual qualifying fractions near 0%, 0.1%, 1%, 10%, 50%, 100%; include text-only control |
| Multiple columns are worth adding | Compare individually understood columns with their combination; test column order |
| A segment count is appropriate | Rebuild at 8, 16, 48, 96 with the same workers and queries; record actual segment sizes |
| Results are stable across construction | Repeat independent index builds, not only queries on one build |
| Boolean and NULL cases retain correctness | Dedicated AND/OR/NOT, NULL and exact-boundary queries with full-result oracles |
| The layout remains useful during writes | Measure after inserts/updates/deletes and maintenance; distinguish M2 behavior from future M3 |

Do not launch the entire product of these dimensions. Use the first pass to
identify useful or surprising cases, then vary one factor at a time.

### Previous run limit

Run `35845195017` completed the first date experiment at 1M. Its 20M job failed
in the date-only layout's second round (PR), executing the range-enabled
aggregate-count join. The server reported an MPP transport receiver detaching
before EOF. The root cause is not established; this is not a completed 20M
performance comparison, and rerunning without investigating would not validate it.
The new unchanged-query first pass retains the join keys and does not automatically
start a 20M job.
