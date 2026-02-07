window.BENCHMARK_DATA = {
  "lastUpdate": 1770479577175,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'logs' (100K rows)": [
      {
        "commit": {
          "author": {
            "email": "mithun.cy@gmail.com",
            "name": "Mithun Chicklore Yogendra",
            "username": "mithuncy"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fbb23b0e1a3fbdc027879e3f765445d5e894b44e",
          "message": "feat: Expose dual sorted/unsorted CustomPaths to Postgres planner (#4025)\n\n## Summary\n- Add sorted scan capability that leverages Tantivy's physical segment\nsorting\n- Expose dual CustomPaths (sorted/unsorted) to PostgreSQL planner for\ncost-based selection\n- Implement segment-level scanning with SortPreservingMergeExec for\nglobally sorted output\n\n### Implementation Details\n1. **Index Creation**: `sort_by` causes Tantivy to physically sort docs\nwithin segments\n2. **Planning**: CustomPath carries sort_order metadata for cost-based\nselection\n3. **Execution**: DataFusion plan created in exec_custom_scan:\n   - SegmentScanPlan with N partitions for N segments\n   - Each partition scans one segment independently\n   - Segments are already sorted by Tantivy during index creation\n4. **Segment checkout**: SortPreservingMergeExec calls\nexecute(partition_idx), which invokes factory function to checkout that\nsegment on-demand. Merges N pre-sorted segment streams into single\nglobally sorted output stream.\n\n### Key Changes\n- Add `sort_order()` to SearchIndexReader to read segment sort order\n- Add `into_segments()`/`from_single_segment()` for segment-level\nscanning\n- Add `sort_order` field to JoinSideInfo for planning\n- Add `Serialize`/`Deserialize` to SortByField for plan serialization\n- Add SegmentScanPlan for multi-partition segment scanning\n- Add SortPreservingMergeExec integration for sorted merges\n- Change FFHelper to `Arc<FFHelper>` for sharing across partitions\n\nFixes #4066\n\n## Test plan\n- pg_regress test `mixedff_sorted_lazy_checkout` passes\n- New Rust integration tests in `tests/tests/index_sorting.rs`\n- All pre-commit checks pass (clippy, formatting, unused deps)\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-02-04T22:06:40-08:00",
          "tree_id": "b4f39e64a91888d85b5905fdc18db95d8405c385",
          "url": "https://github.com/paradedb/paradedb/commit/fbb23b0e1a3fbdc027879e3f765445d5e894b44e"
        },
        "date": 1770272051928,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 40.951499999999996,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 43.444,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.096,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.326,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.4665,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 13.963999999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.5945,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.072499999999998,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 11.881499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.343,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.475,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 13.779,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.643500000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 12.338,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.735,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 13.5335,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.5165,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.896,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 11.687999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 11.8875,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.485499999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 13.912500000000001,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 19.6945,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 19.522,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 12.754000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.651,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 13.642,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 13.9665,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.2125,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.6845,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.5405,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.1945,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.504999999999999,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.762,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 11.843,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 12.2895,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.047,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 12.8595,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.2940000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.2204999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.7835,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.375500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.4435,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 30.356,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.232500000000002,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.278500000000001,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.160499999999999,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.6945,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.308,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.1065000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.153499999999999,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.0715,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.2745,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6e1a2dbd386bf9e37e7731d477c093f66ab1ff82",
          "message": "feat: move datafusion logical plan construction to planning time, enable `EXPLAIN` output (#4096)\n\n# Ticket(s) Closed\n\n- Closes #4059 \n\n## What\n\nIn the join custom scan, construct the `LogicalPlan` in\n`plan_custom_state` instead of `exec_custom_state`.\n\n## Why\n\nSee linked issue.\n\n## How\n\n- `LogicalPlan` requires `datafusion_proto` in order to be\nser/deserializable\n- Additionally, we need to tell Datafusion how to ser/deserialize any\ncustom table provider + UDF\n\n## Tests\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-02-05T09:55:35-08:00",
          "tree_id": "89b93237dbc7adc07aa46df51d9cdcbee97980ea",
          "url": "https://github.com/paradedb/paradedb/commit/6e1a2dbd386bf9e37e7731d477c093f66ab1ff82"
        },
        "date": 1770314747265,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 41.0585,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 40.552,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 21.3935,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 13.1225,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.7555,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.606000000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.948,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.0965,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.568000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.4905,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.751999999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 14.315999999999999,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 28.665,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 12.9875,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 13.0595,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.7645,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 15.246,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.6505,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.8845,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.588999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.411999999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 14.5555,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 20.155,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 20.2575,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 13.093,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 13.0625,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 14.614,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 14.4765,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 5.8614999999999995,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.025,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 13.409500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 14.099,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 14.174,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.407499999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.699000000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 12.1015,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.6345,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.6015,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.406000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.2435,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.4725,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.356,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.5845,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 29.110999999999997,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.383,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.375,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.2775,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.7829999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.307,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.115,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.1370000000000005,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.039,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.305,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b00b82ddc11f85c2ff02c08cff8273a9d3e2484a",
          "message": "chore: Set 0.21.7 in Cargo.toml (#4107)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\nThis was blocking a community PR.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2026-02-05T14:25:25-05:00",
          "tree_id": "7788fec8d59375d657a622a661fbd4d92df92188",
          "url": "https://github.com/paradedb/paradedb/commit/b00b82ddc11f85c2ff02c08cff8273a9d3e2484a"
        },
        "date": 1770320123817,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 44.760999999999996,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 44.1405,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.769,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.871500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.532499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.122,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.768,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.1825,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.497,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.2995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.5125,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 13.8385,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.7335,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 12.713000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.7365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.5035,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.8665,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 29.266,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.132,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.0585,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.669,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 14.028500000000001,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 21.2455,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 20.3315,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 12.744,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.72,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 14.169,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 13.724,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.0425,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.921,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.801,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.555,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.762,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.5985,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.2795,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 11.9535,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.553,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.211500000000001,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.336,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.0385,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.649,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.285,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.558,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 26.2465,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.25,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.274000000000001,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.265,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.625,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.2765,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.077500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.119,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.058999999999999,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.2325,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5e9e71456c0f7ec28db967808f8dc8811677c99d",
          "message": "chore: stabilize JoinScan EXPLAIN output across machines + disable DataFusion parallelization (#4110)\n\n## Ticket(s) Closed\n\n- Closes #2968 (partial)\n\n## What\n\nDisable DataFusion parallelization for JoinScan to ensure deterministic\nEXPLAIN output.\n\n## Why\n\nThe physical plan output from `EXPLAIN` on JoinScan queries included\npartition counts (e.g., `input_partitions=8`, `Hash([...], 8)`) that\nvaried based on the machine's CPU count. This made regression tests\nflaky and output hard to compare across environments. Also, DataFusion\nparallelization shouldn't be enabled.\n\n## How\n\n- Added `create_session_context()` helper that configures DataFusion\nwith `target_partitions=1`\n- Replaced all `SessionContext::new()` calls in JoinScan with this\nhelper\n- This forces single-partition execution, removing machine-dependent\npartition counts from EXPLAIN output\n\n## Tests\n\nUpdated expected output in `join_custom_scan.out` to reflect the new\nstable physical plan format (e.g., `HashJoinExec: mode=CollectLeft`\ninstead of partitioned mode with repartitioning).",
          "timestamp": "2026-02-05T17:48:06-08:00",
          "tree_id": "84d83931a8e3e8ebb5a36eb5b683d7ecf5206277",
          "url": "https://github.com/paradedb/paradedb/commit/5e9e71456c0f7ec28db967808f8dc8811677c99d"
        },
        "date": 1770343085489,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 41.673,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 43.541,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.316000000000003,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 13.093,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.1465,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.5085,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.091,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 11.976,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 11.8675,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.575,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 14.122,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.495,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 13.139,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.939499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.0725,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.681000000000001,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.3695,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.4335,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.181999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.3235,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 13.867,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 20.833,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 19.691000000000003,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 13.0705,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.7135,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 13.9605,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 13.971499999999999,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.042,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.4835,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.544,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.689,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.679,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.825,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.3565,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 12.1975,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 12.851500000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.1675,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.4025,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.2415,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.6245000000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.3774999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.5329999999999995,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 26.836,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.091000000000001,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.0725,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.027000000000001,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.6765,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.281,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.0585,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.103,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.045,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.243,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "45c3ba5980925355dc7de3f2bcebf891e0db087e",
          "message": "feat: store NumericBytes as Bytes instead of hex-encoded Text (#4091)\n\n## Ticket(s) Closed\n\n- Closes #2968 \n\n## What\n\nChanges `NumericBytes` storage from hex-encoded strings (Text field) to\nraw bytes (Bytes field) in Tantivy.\n\n## Why\n\nThe previous implementation stored high-precision NUMERIC values as\nhex-encoded strings in Text fields. This was a workaround because\nTantivy's `BytesColumn` didn't support range queries or TopN sorting.\n\nNow that we've added those features to Tantivy (in\nhttps://github.com/paradedb/tantivy/pull/100), we can use native Bytes\nstorage which:\n- Eliminates hex encoding/decoding overhead\n- Skips unnecessary UTF-8 validation\n- Reduces storage size (raw bytes vs 2x for hex)\n\n## How\n\n- Changed `add_text_field` to `add_bytes_field` for NumericBytes fields\n- Updated indexing to store `OwnedValue::Bytes` instead of hex-encoded\n`OwnedValue::Str`\n- Split query conversion into `numeric_value_to_raw_bytes` (for Bytes\nfields) and `numeric_value_to_hex_string` (retained for NUMRANGE JSON\nfields)\n- Added `FastFieldType::Bytes` and `FFType::Bytes` for fast field\nhandling\n- Added `SortByBytes` support for TopN sorting on Bytes fields\n- Updated Tantivy to include BytesColumn range query and TopN support\n\n## Tests\n\nExisting `numeric_pushdown` tests pass. Test output changes are cosmetic\n(reordering of fast fields in EXPLAIN output).",
          "timestamp": "2026-02-05T17:47:38-08:00",
          "tree_id": "be4097d85dbc5777a383851199cf240cdd5a155d",
          "url": "https://github.com/paradedb/paradedb/commit/45c3ba5980925355dc7de3f2bcebf891e0db087e"
        },
        "date": 1770343092396,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 43.898,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 43.0135,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.543,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.963999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 13.321000000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.287500000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 15.155999999999999,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.404000000000003,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.267499999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.453,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.8985,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 14.375,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 28.3395,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 12.9725,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 13.479,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.2405,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 15.409500000000001,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.6295,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.1175,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.432,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.433,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 14.827,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 19.9725,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 19.9825,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 13.436,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 13.2315,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 14.593,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 13.919,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.079,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.578,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 14.3675,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 14.3355,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.513,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.753499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 12.1545,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.634,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.6555,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.255,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.0565,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.6494999999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.448,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.83,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 27.2135,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.187999999999999,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.572,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.2485,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.709,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.239,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.185,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.151,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.1259999999999994,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.18,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "08e7c7f47f5027b2c0bfd3bf00e438db04c1f65c",
          "message": "chore: remove FastFieldType enum (#4111)\n\n## What\n\nRemoves the `FastFieldType` enum and uses `SearchFieldType` directly\nthroughout the fast field handling code.\n\n## Why\n\n`FastFieldType` was a redundant intermediate type that \"widened\"\n`SearchFieldType` variants into storage categories (e.g., Text, Uuid,\nJson all became `String`). This indirection:\n\n- Required duplicate Arrow type mappings in multiple places\n- Lost type metadata (like Postgres OIDs) during the conversion\n- Made consumers responsible for reverse-engineering the original types\n\nUsing `SearchFieldType` directly preserves all type information, and\nconsumers can now access the original Postgres OID via\n`SearchFieldType::typeoid()`.\n\n## How\n\n- Deleted `FastFieldType` enum and its `From<SearchFieldType>` impl\n- Changed `WhichFastField::Named` to hold `SearchFieldType` instead of\n`FastFieldType`\n- Added `arrow_data_type()` method to `SearchFieldType` to centralize\nArrow type mapping\n- Added `field_type()` helper to `WhichFastField` for accessing the\nunderlying type\n- Renamed `fast_field_type_for_pullup` → `field_type_for_pullup`\n- Simplified `build_schema` in table_provider.rs to use the new methods\n\n## Tests\n\nExisting test suite covers this refactor. Updated `scan/tests.rs` to use\n`SearchFieldType` directly.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-02-05T20:57:23-08:00",
          "tree_id": "70010517c4834d3cb93172decc2101436bfba890",
          "url": "https://github.com/paradedb/paradedb/commit/08e7c7f47f5027b2c0bfd3bf00e438db04c1f65c"
        },
        "date": 1770354293765,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 47.0595,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 46.389,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.4815,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.7865,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.3505,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.1795,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.3385,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.072,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.4815,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 11.868500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.3995,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 14.027000000000001,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.664499999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 13.016,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.8205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 13.8885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.791,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.367,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.1025,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.387,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.7025,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 13.687999999999999,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 19.692,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 20.417,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 13.393,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.597,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 13.98,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 14.155000000000001,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 5.8665,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.618,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.432,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.897,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.582,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.4375,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.006499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 11.9355,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 12.995999999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 12.773,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.252,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.1105,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.5934999999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.313,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.5969999999999995,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 30.671999999999997,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.358,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.3795,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.208,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.5225,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.1965,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.025,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.201499999999999,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.037,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.191,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "121197985+pantShrey@users.noreply.github.com",
            "name": "pantShrey",
            "username": "pantShrey"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f8ddc8324d4e7a504a1b7abc2ee5f8095e54073c",
          "message": "feat: allow disabling fieldnorms (#4054)\n\n# Ticket(s) Closed\n- Closes #3998\n## What\nAdd support for disabling fieldnorms via the v2 API\nUsers can now specify:\n```sql \ncontent pdb.simple('fieldnorms=false')\n```\nWhen disabled, document length normalization is skipped, resulting in\nidentical BM25 scores for documents with the same term frequency\nregardless of length.\n## Why\nThis was requested by users who want BM25 scoring that does not penalize\nlonger documents\n## How\nExtended the `TypmodSchema` to validate the `fieldnorms` option and read\nits value via `load_typmod` inside `search_field_config_from_type`.\n## Tests\nAdded test that verifies fieldnorms=false by asserting identical BM25\nscores for documents with equal term frequency but different lengths,\nconfirming document-length normalization is disabled.",
          "timestamp": "2026-02-06T11:35:39-05:00",
          "tree_id": "c0f32a510fef509b225a2f78a8d49e47295811da",
          "url": "https://github.com/paradedb/paradedb/commit/f8ddc8324d4e7a504a1b7abc2ee5f8095e54073c"
        },
        "date": 1770396191357,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 39.7395,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 43.1315,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.304499999999997,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.782499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 13.0425,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.2675,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.568000000000001,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 20.8545,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.59,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.150500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.555,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 13.793,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.792,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 13.192499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.71,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.3885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.893,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 28.154,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.4275,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.0465,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.701,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 14.436499999999999,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 19.7955,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 19.583,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 12.9505,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.843,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 14.422,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 14.1265,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 5.8025,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.900500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.7275,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 14.1005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.996,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.389,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.3935,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 11.955,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.511,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.2265,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.253,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.165,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.6944999999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.2780000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.559,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 26.488,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.2685,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.306000000000001,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.2365,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.722,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.259,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.0969999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.1575,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.1129999999999995,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.237,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "140686560+dask-58@users.noreply.github.com",
            "name": "Dhruv",
            "username": "dask-58"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7c12643791f63d50fcec86e4cf70b94eaeca75c6",
          "message": "fix(custom-scan): skip hook when extension is not installed (#4106)\n\n# Ticket(s) Closed\n\n- Closes #4103\n\n## What\n- Skip planner and custom scan hooks when `pg_search` is not installed\nin the current database.\n- Add a regression test for running a window function in a database\nwithout the extension.\n\n## Why\nWhen `pg_search` is preloaded but not installed, hook code tries to\naccess paradedb objects and crashes with `\"schema paradedb does not\nexist\"`.\n\n## How\n- Check if the `pg_search` extension exists before running hook logic.\n- Add `issue_4103` regression test that creates a database without the\nextension and runs a window function.\n\n## Tests\n- `cargo pgrx regress pg17 issue_4103`\n- Manual: created a DB without extension and ran `SELECT count(*) OVER\n()`",
          "timestamp": "2026-02-06T11:57:41-05:00",
          "tree_id": "8b2b6249d0eed977411c710ddcfd8645c7f8507b",
          "url": "https://github.com/paradedb/paradedb/commit/7c12643791f63d50fcec86e4cf70b94eaeca75c6"
        },
        "date": 1770397511517,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 40.36450000000001,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 40.7285,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.473,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.606,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.815,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 13.847999999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.236,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 21.448,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 11.778500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.3455,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.4545,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 13.4895,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.0795,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 12.7335,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.3825,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.282,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.8415,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.610999999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 11.849499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.0235,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.4145,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 14.1485,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 19.6285,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 19.768500000000003,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 12.8635,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.7075,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 13.947,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 13.994,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.125,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.8895,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.546,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.6465,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.9435,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.849499999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.245000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 11.964,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 12.8155,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 12.757,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.359500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.1905,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.7075,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.379,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.4595,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 26.3675,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.477,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.433499999999999,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.331,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.6370000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.323,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.068,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.194,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.1850000000000005,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.2395,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "22434f12d7eec5084167804afd22b34f86810f09",
          "message": "fix: Allow the custom scan to be used in parallel plans without its own workers (#4109)\n\n## What\n\n* Split `set_parallel_safe` from `set_parallel` for `CustomScan`s, and\nalways mark the `basescan` as `parallel_safe`.\n* Disable #4077 for joins.\n\n## Why\n\nAfter #4077, two things happened to the first benchmark query in\n`benchmarks/datasets/docs/queries/pg_search/hierarchical_content-scores-large.sql`\n(and likely others):\n\n### Loss of parallel safety\n\nThe query (which was previously using `Normal` custom scans) was failing\nto get the custom scan at all, and was instead falling back to the IAM\n(which cannot produce scores):\n\n<details>\n<summary>Query Plan</summary>\n Limit  (cost=804922.46..804924.96 rows=1000 width=3048)\n   ->  Sort  (cost=804922.46..804940.37 rows=7161 width=3048)\nSort Key: ((((pdb.score(documents.id)) + (pdb.score(files.id))) +\n(pdb.score(pages.id)))) DESC\n         ->  Hash Join  (cost=594228.68..804529.83 rows=7161 width=3048)\n               Hash Cond: (files.\"documentId\" = documents.id)\n-> Gather (cost=571487.12..688316.81 rows=144249 width=2070)\n                     Workers Planned: 7\n-> Parallel Hash Join (cost=570487.12..672891.91 rows=20607 width=2070)\n                           Hash Cond: (pages.\"fileId\" = files.id)\n-> Parallel Custom Scan (ParadeDB Scan) on pages (cost=10.00..3621.72\nrows=361172 width=1040)\n                                 Table: pages\n                                 Index: pages_index\n                                 Segment Count: 8\n                                 Exec Method: NormalScanExecState\n                                 Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"content\",\"query_string\":\"Single\nNumber Reach\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Parallel Hash (cost=566064.09..566064.09 rows=31202 width=1030)\n-> Parallel Index Scan using files_index on files (cost=10.00..566064.09\nrows=31202 width=1030)\nIndex Cond: (id @@@\n'{\"with_index\":{\"oid\":2096822,\"query\":{\"parse_with_field\":{\"field\":\"title\",\"query_string\":\"collab12\",\"lenient\":null,\"conjunction_mode\":null}}}}'::paradedb.searchqueryinput)\n               ->  Hash  (cost=1561.36..1561.36 rows=155136 width=986)\n-> Custom Scan (ParadeDB Scan) on documents (cost=10.00..1561.36\nrows=155136 width=986)\n                           Table: documents\n                           Index: documents_index\n                           Segment Count: 8\n                           Exec Method: NormalScanExecState\n                           Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"parents\",\"query_string\":\"SFR\",\"lenient\":null,\"conjunction_mode\":null}}}}\n(27 rows)\n</details>\n\nThe reason for this is that #4077 caused us to determine that because\nthe scan was scanning fewer than 300k rows, it probably didn't need\nparallel workers.\n\nBut `set_parallel` was _also_ the only place where we were claiming that\nour custom scan is `parallel_safe`. And a plan must be parallel safe to\nbe used inside of any _other_ parallel scan.\n\n### No participation in parallel hash joins\n\nAfter fixing the above, we got the custom scan, but the plan was subtly\ndifferent from before:\n\n<details>\n<summary>Query Plan</summary>\n Limit  (cost=188822.03..188822.06 rows=10 width=3048)\n   ->  Sort  (cost=188822.03..188839.93 rows=7161 width=3048)\nSort Key: ((((pdb.score(documents.id)) + (pdb.score(files.id))) +\n(pdb.score(pages.id)))) DESC\n         ->  Gather  (cost=87220.00..188667.28 rows=7161 width=3048)\n               Workers Planned: 7\n-> Hash Join (cost=86220.00..186951.18 rows=1023 width=3048)\n                     Hash Cond: (pages.\"fileId\" = files.id)\n-> Parallel Custom Scan (ParadeDB Scan) on pages (cost=10.00..3621.72\nrows=361172 width=1040)\n                           Table: pages\n                           Index: pages_index\n                           Segment Count: 8\n                           Exec Method: NormalScanExecState\n                           Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"content\",\"query_string\":\"Single\nNumber Reach\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Hash (cost=84184.19..84184.19 rows=7745 width=2016)\n-> Hash Join (cost=22751.54..84184.19 rows=7745 width=2016)\nHash Cond: (files.\"documentId\" = documents.id)\n-> Custom Scan (ParadeDB Scan) on files (cost=10.00..1570.12 rows=156012\nwidth=1030)\n                                       Table: files\n                                       Index: files_index\n                                       Segment Count: 8\n                                       Exec Method: NormalScanExecState\n                                       Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"title\",\"query_string\":\"collab12\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Hash (cost=1561.35..1561.35 rows=155135 width=986)\n-> Custom Scan (ParadeDB Scan) on documents (cost=10.00..1561.35\nrows=155135 width=986)\n                                             Table: documents\n                                             Index: documents_index\n                                             Segment Count: 8\nExec Method: NormalScanExecState\n                                             Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"parents\",\"query_string\":\"SFR\",\"lenient\":null,\"conjunction_mode\":null}}}}\n</details>\n\nRather than being able to participate in a parallel hash join with\nparallel independent sorts, the two smaller tables were instead being\nscanned sequentially into a Gather, and _then_ sorted.\n\nThis lead to a total cost of 188k, which was sufficient on CI machines\nto trigger JIT compilation, and cause queries long enough to cause\ntimeouts.\n\nDisabling #4077 in the context of joins allowed the two smaller tables\nto participate in the plan.\n\n## How\n\n* Added `set_parallel_safe`, and used it universally in the `basescan`,\nand added an additional branch to `init_search_reader` to handle the\ncase when we are part of a parallel plan, but without our own parallel\nstate.\n* Disabled #4077 in the presence of joins, and clarified the\nrelationship with the `uses_correlated_vars` flag.\n* Made a quick driveby fix to ensure that our estimates match the actual\nnumber of emitted tuples.\n\nThe final restored plan looks like:\n\n<details>\n<summary>Query Plan</summary>\n Limit  (cost=16558.60..16559.83 rows=10 width=3048)\n   ->  Gather Merge  (cost=16558.60..17428.92 rows=7084 width=3048)\n         Workers Planned: 7\n         ->  Sort  (cost=15558.48..15561.01 rows=1012 width=3048)\nSort Key: ((((pdb.score(documents.id)) + (pdb.score(files.id))) +\n(pdb.score(pages.id)))) DESC\n-> Parallel Hash Join (cost=10564.17..15536.61 rows=1012 width=3048)\n                     Hash Cond: (pages.\"fileId\" = files.id)\n-> Parallel Custom Scan (ParadeDB Scan) on pages (cost=10.00..3621.72\nrows=361172 width=1040)\n                           Table: pages\n                           Index: pages_index\n                           Segment Count: 8\n                           Exec Method: NormalScanExecState\n                           Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"content\",\"query_string\":\"Single\nNumber Reach\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Parallel Hash (cost=10540.35..10540.35 rows=1106 width=2016)\n-> Parallel Hash Join (cost=2861.14..10540.35 rows=1106 width=2016)\nHash Cond: (files.\"documentId\" = documents.id)\n-> Parallel Custom Scan (ParadeDB Scan) on files (cost=10.00..205.02\nrows=19502 width=1030)\n                                       Table: files\n                                       Index: files_index\n                                       Segment Count: 8\n                                       Exec Method: NormalScanExecState\n                                       Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"title\",\"query_string\":\"collab12\",\"lenient\":null,\"conjunction_mode\":null}}}}\n-> Parallel Hash (cost=203.84..203.84 rows=19384 width=986)\n-> Parallel Custom Scan (ParadeDB Scan) on documents (cost=10.00..203.84\nrows=19384 width=986)\n                                             Table: documents\n                                             Index: documents_index\n                                             Segment Count: 8\nExec Method: NormalScanExecState\n                                             Scores: true\nTantivy Query:\n{\"with_index\":{\"query\":{\"parse_with_field\":{\"field\":\"parents\",\"query_string\":\"SFR\",\"lenient\":null,\"conjunction_mode\":null}}}}\n</details>\n\n## Tests\n\nBenchmark queries are able to run with both a parallel plan and the\ncustom scan again.\n\nThis was really difficult to reproduce outside of the benchmark harness:\nit requires a large enough dataset to trigger a parallel plan on a\nparent node. I spent at least an hour trying to repro it in a regress\ntest, but failed.",
          "timestamp": "2026-02-06T09:04:47-08:00",
          "tree_id": "767b23c3a564e81c3f4ba39e2e4ac753fefa9bc0",
          "url": "https://github.com/paradedb/paradedb/commit/22434f12d7eec5084167804afd22b34f86810f09"
        },
        "date": 1770397945147,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 41.540000000000006,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 42.2455,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.525,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 13.0915,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.8205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 14.4955,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.6295,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 20.7965,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.725000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 12.2075,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.973,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 13.979,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.4695,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 13.286999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 13.214500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 14.707999999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.8485,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.7735,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.607,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 12.2605,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.7695,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 14.33,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 21.122,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 19.7875,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 13.274000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.7685,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 14.7095,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 14.075,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.086,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.893,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.8645,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.926,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 13.93,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.6335,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.667,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 12.4265,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.529,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.428999999999998,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.3934999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.231,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.6704999999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.3585,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.7245,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 27.558999999999997,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.533,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.455,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.387,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.7485,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.3865,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.2455,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.1575,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.253500000000001,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.3355,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a8323d7d2ed7a46ed4c398882be595628edca559",
          "message": "chore: Add missing AGPL license headers to Rust source files (#4124)\n\n# Ticket(s) Closed\n\n- Closes #N/A\n\n## What\n\n47 Rust source files were missing the standard AGPL-3.0 license header\ncomment. This adds the header to all of them so every `.rs` file in the\nrepo is consistent.\n\n## Why\n\nAll source files should carry the AGPL license header for legal\ncompliance and consistency. These files were added over time without it.\n\n## How\n\n- Identified all `.rs` files (excluding `target/`) missing the `//\nCopyright (c) 2023-2026 ParadeDB, Inc.` header\n- Prepended the standard 16-line AGPL header to each file, matching the\nexact format used across the rest of the codebase\n- Files span `benchmarks/`, `macros/`, `pg_search/`, `stressgres/`,\n`tests/`, and `tokenizers/`\n\n## Tests\n\nNo functional changes — header comments only. `cargo check`, `fmt`, and\n`clippy` all pass via pre-commit hooks.\n\nCo-authored-by: Claude Opus 4.6 <noreply@anthropic.com>",
          "timestamp": "2026-02-07T10:45:17-05:00",
          "tree_id": "627b799a5aaeb8f0076d7bcda8b95173dee601ae",
          "url": "https://github.com/paradedb/paradedb/commit/a8323d7d2ed7a46ed4c398882be595628edca559"
        },
        "date": 1770479572733,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 40.278,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 43.375,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 20.2795,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 12.6175,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 12.7445,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 13.7905,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 14.639,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 20.747,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 12.2925,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 11.734,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 13.519,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 13.8705,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 29.6,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 12.977,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 12.575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 13.951,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 14.6625,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 27.935499999999998,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 12.563500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 11.9115,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 13.477,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 13.985,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 19.649500000000003,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 20.087,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 12.847999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 12.47,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 13.927,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 13.9465,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 6.2059999999999995,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.785499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.5415,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 13.696000000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 14.709,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 8.483,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 12.045,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 12.112,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 13.3085,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 13.1945,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 4.328,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.189,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 3.6185,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 4.2475000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 4.621,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 25.862000000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 8.717,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 8.4605,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 8.371,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 5.7915,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 5.316000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 5.1370000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 5.1995000000000005,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 5.1805,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 5.209,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      }
    ]
  }
}