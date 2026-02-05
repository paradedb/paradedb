window.BENCHMARK_DATA = {
  "lastUpdate": 1770316188125,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'logs' (100M rows)": [
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
        "date": 1770316183513,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7843.527,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7776.4755000000005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 2279.4105,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 292.823,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 95.93299999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 295.453,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 425.804,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 2274.27,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 291.47749999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 95.64750000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 291.003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 431.3755,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 3298.1645,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 261.5235,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 65.0675,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 262.719,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 380.11400000000003,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 3262.3945,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 259.411,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 66.447,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 263.4275,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 379.3165,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 15226.0245,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 2250.1324999999997,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 292.392,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 96.25649999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 295.235,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 293.9585,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 219.159,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 130.3775,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 89.3865,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 131.13600000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 130.3625,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 712.662,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 305.716,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 156.5725,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 307.207,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 305.89099999999996,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.1705,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.0735,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.548,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 5.8965,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.546,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 6021.742,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 441.533,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 400.7735,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 453.5155,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 75.606,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 59.4935,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 41.809,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score-asc",
            "value": 89.47800000000001,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-score-desc",
            "value": 83.9305,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 45.8065,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      }
    ]
  }
}