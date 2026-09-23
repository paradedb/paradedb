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

Artifacts `partition-pruning-1m` and `partition-pruning-20m` contain source SHAs,
library checksums, index DDL and identities, dataset distribution, resolved query
SQL, every raw timing, full results, executed plans, diagnostic patches and
traces, and `comparison.md` / `comparison.json`. A negative timing percentage
means the PR is faster. Failure artifacts retain partial results and server logs.

Compare main versus PR within each layout to measure the code change. Compare
layouts within each build to measure layout effects. Inspect join plans for
changed execution strategies when a partition column is added or removed.
Do not attribute either effect solely from a query filename or a lower runtime.
