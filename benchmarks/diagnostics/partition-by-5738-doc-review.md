# Local review of the `partition_by` guidance

This file is local working material for issue #5738. It is not a comment on
PR #6288 and is not posted to any review page. Every recommendation below is
labelled as either source evidence or a claim that still needs an experiment.

## Reproduction inputs

The measured comparison is the same one documented in
[`partition-by-5738-report.md`](partition-by-5738-report.md): GitHub Actions
`c8gd.metal-24xl`, ARM64, PostgreSQL 18, the Stack Overflow 1M snapshot,
`global_target_segment_count = 48`, and production commits
`c3479d9a1fd78fd85676524a7ead9820513ad67a` (before #6346) and
`fa8d800a282cea684f3dda48daa1ab23cdf76634` (merged #6346). The benchmark run
was [35967678542](https://github.com/paradedb/paradedb/actions/runs/35967678542).

The experiment rebuilt the same indexes with one extra `partition_by` column,
kept the timed SQL byte-for-byte unchanged, used ABBA ordering, and collected
30 hot samples per round. The full query manifest, settings, plans, trace
summaries, and perf counters are in that run's artifact. This document does
not treat an isolated `EXPLAIN ANALYZE` sample as a timing result.

## What the measurements establish

| Added column | Query that constrained it | Segment evidence | Timing evidence | Decision |
| --- | --- | ---: | ---: | --- |
| `users.reputation` | permissioned local-sort join with `u.reputation > 100` | 10 of 48 user segments rejected | 20.419 -> 17.703 ms before #6346 (-13.30%); 20.242 -> 17.947 ms on #6346 (-11.34%) | Include as a benchmark treatment for a matching predicate. |
| `posts.post_type_id` | direct low-cardinality filter `post_type_id < 3` | 13 of 48 segments rejected; 35 scorers created | 3.402 -> 2.950 ms before (-13.28%); 3.403 -> 2.822 ms on #6346 (-17.09%) | Include as a direct-pruning case. |
| `posts.creation_date` | date-filtered scan | 7 of 48 segments rejected | +0.37% before; -1.03% on #6346 | Keep as a mixed-result control; do not promise a universal win. |
| `comments.score` | semi-join that does not read `comments` | 0 score-based segments rejected | No consistent improvement | Do not use as evidence for pruning. |
| `users.reputation` | aggregate that does not read `users` | 0 relevant segments rejected | Within noise; cycles +0.04% before and +1.24% on #6346 | Do not add this layout to that query. |

This supports the practical rule: a partition column earns a benchmark case
when the query constrains that same indexed field and the rejected segments
remove measurable scan/scorer work. A column that is merely present in the
schema, projection, grouping, or another index is not evidence of pruning.

## Review of the proposed documentation

The documentation's core model is consistent with the source and the traces:
segment metadata is compared with a predicate before a scorer is created, and
segments whose ranges cannot intersect are skipped. The following statements
are therefore safe when presented as behavior, not as a performance promise:

* `partition_by` is useful for frequently filtered columns and for join keys.
* Narrower ranges can increase the fraction of rejected segments, at the cost
  of more segment metadata.
* Writes and merges can change how useful the original layout remains; this
  needs a separate write/maintenance experiment before a numeric claim.

The following statements should remain explicitly conditional until measured
for the relevant workload:

* “a query scans only the segment containing tenant 42” — only true when the
  built index actually has a non-overlapping segment for that value and the
  statistics are readable;
* “eliminates cross-worker data exchange entirely” — the plan must be checked
  for the specific join and execution mode; the measured runs still contain
  MPP coordination and a partitioned hash join;
* “2–8x CPU count is recommended” — it is a starting hypothesis, not a
  result of the 1M measurements above;
* “multiple columns divide the segment budget across dimensions” — verify the
  writer's actual layout algorithm before describing it as a geometric rule;
* “performance gradually declines as writes accumulate” — plausible, but not
  measured in this experiment.

The guide should distinguish three facts in every example:

1. the indexed `partition_by` field;
2. the query predicate that constrains that field; and
3. the observed rejected-segment count and end-to-end timing.

That prevents a reader from interpreting an index option as a guaranteed
speedup for unrelated queries.

## Default range-join change: source audit and required experiment

The current source declares
`ENABLE_RANGE_PARTITIONED_JOIN` as `GucSetting<bool>::new(false)` in
`pg_search/src/gucs.rs`, and the joinscan README documents the default as
`false`. The range-partitioning rule checks this GUC before constructing the
range plan. Existing benchmark and regression cases set it to `on` explicitly;
`join_lateral_unnest.sql` deliberately sets it to `off` as a negative control.

Changing the default is therefore a separate code change, not a benchmark-only
documentation edit. A repeatable validation must:

1. change the default and the README together;
2. remove redundant `SET ... TO on` only from cases that already require the
   feature, while retaining explicit `off` controls;
3. run the complete affected regression files and compare their expected plans;
4. rerun hash-partitioned and range-partitioned benchmark variants with the
   same index snapshots; and
5. record whether an unset GUC changes any plan outside the intended range
   join cases.

No production default was changed on this experiment branch. This is an
implementation checklist, not a claim that the default change is complete.

## Remaining evidence gaps

The current run covers 19 unchanged queries and six layouts, not every one of
the 110 SQL files currently under the Stack Overflow query directory. Before
turning this into a product recommendation, run a full-suite pass at 1M and a
selected larger size, then add representative post-insert/reindex measurements.
The call-stack run [36012492217](https://github.com/paradedb/paradedb/actions/runs/36012492217)
is still collecting `perf record -g` reports; function-level CPU claims must
wait for that artifact.

