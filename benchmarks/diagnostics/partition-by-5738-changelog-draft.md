# Local changelog draft for #5738

`partition_by` can reduce search work when a query filters a column that is
also used to partition the index. ParadeDB compares that predicate with each
segment's recorded bounds and can reject a segment before creating its scorer.
In the 1M Stack Overflow experiment, adding `post_type_id` rejected 13 of 48
segments and reduced the unchanged low-cardinality filter from 3.403 ms to
2.822 ms on the #6346 build. Adding `users.reputation` rejected 10 of 48
segments and reduced the unchanged permissioned local-sort join from 20.242 ms
to 17.947 ms on that build.

These are measured examples, not guarantees. A partition column that the query
does not constrain produced no relevant rejected segments in the aggregate and
semi-join controls. The full guidance should therefore explain how to select a
frequently filtered or joined column, how `target_segment_count` affects the
trade-off, and why writes and merges require a separate maintenance
measurement.

Reference: the local evidence report at
`benchmarks/diagnostics/partition-by-5738-report.md` and the proposed guide in
PR #6288. This draft is intentionally not posted to the PR page.

