window.BENCHMARK_DATA = {
  "lastUpdate": 1760111077436,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'logs' Query Performance": [
      {
        "commit": {
          "author": {
            "name": "paradedb[bot]",
            "username": "paradedb-bot",
            "email": "developers@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "7caf32d33552263a7408c43c2c057dad25c77222",
          "message": "chore: Upgrade to `0.18.9` (#3234)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>",
          "timestamp": "2025-09-26T20:26:43Z",
          "url": "https://github.com/paradedb/paradedb/commit/7caf32d33552263a7408c43c2c057dad25c77222"
        },
        "date": 1758927225307,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7418.355,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7408.1515,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1901.3955,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 429.773,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 111.60249999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 432.8135,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1868.824,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 410.453,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.315,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 409.459,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2879.218,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 429.582,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 143.32549999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 431.3905,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2858.712,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 410.5645,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.674,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 415.86199999999997,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14799.9885,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1857.3629999999998,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 427.8535,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 111.767,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 432.4185,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 232.24,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 191.38400000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 101.8945,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 191.6135,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 741.4555,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 448.1,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 157.4205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 446.8845,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.813,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.3895,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.5655,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8245000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.026499999999999,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5544.5315,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 92.49950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 73.753,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 72.8655,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 125.792,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 73.26249999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba",
          "message": "chore: upgrade to `0.18.0` (#2980)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote: `cargo.toml` is already on `0.18.0` so this is docs-only\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T19:09:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba"
        },
        "date": 1758927345227,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7365.275,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7358.0525,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1887.191,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 424.697,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.0505,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 425.3935,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1899.188,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 409.161,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.072,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 408.7505,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2874.5265,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 427.822,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.29599999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 2872.544,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2851.0725,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 412.3655,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.2575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 2854.294,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14787.425,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1843.4645,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 424.004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 104.1325,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 423.41200000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 216.827,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 187.9735,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.48,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 189.7335,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 700.3305,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 445.445,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 155.0575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 447.1885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.6955,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.518000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.8469999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.7485,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.894,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 6877.099,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 96.703,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 75.3355,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.387,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 126.5645,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.3765,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758927387404,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7569.5025000000005,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7558.8295,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1790.5425,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 404.49649999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 92.33000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 1791.7530000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1751.601,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 383.43949999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 74.9125,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 1748.1815,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 6982.285,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 408.02049999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 103.9475,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 6918.2125,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 6920.018,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 388.37199999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 83.553,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 6878.7805,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14653.717,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1716.4185000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 404.6005,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 93.06450000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 1717.851,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 212.2185,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 182.9145,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 94.03999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 183.54500000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 685.336,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 441.895,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 151.85899999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 443.9235,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 10.3355,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 7.5055,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 6.872999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.6815,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.48,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.3285,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 94.8035,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 114.1215,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 94.001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 110.728,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 99.64349999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1758927401182,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7423.836,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7406.8055,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1867.771,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 416.855,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 92.40549999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 1868.996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1798.596,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 385.60400000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 75.44200000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 1799.0385,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 7247.822,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 421.5065,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 102.87899999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 7216.967000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 7211.751,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 387.3105,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 85.92150000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 7230.781999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14640.793,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1786.971,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 416.2305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 93.175,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 1785.3825,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 218.323,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 192.2375,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 102.3265,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 193.025,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 682.208,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 442.057,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 158.5235,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 446.757,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 9.696,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.83,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 6.660500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.291,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.113,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.345,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 99.93700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 119.3395,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 99.5265,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 123.35050000000001,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 109.108,
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
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759252589301,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7413.303,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7429.5525,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1912.124,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 431.68399999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 102.834,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 437.595,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1884.73,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 417.7745,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 89.2175,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 414.193,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2869.9930000000004,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 430.2395,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 113.18,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 436.71500000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2855.7885,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 417.7075,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 90.2055,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 416.1625,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14823.8235,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1874.8415,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 429.9425,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 102.33449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 436.8225,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 212.8105,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 182.60950000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 91.555,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 184.27550000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 690.787,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 453.3915,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 151.6225,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 448.0425,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.541499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 5.9719999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.7635000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.631,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.696,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5331.3055,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 87.36000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 68.4705,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 67.424,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 119.0835,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 67.9555,
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
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759265945846,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7417.8155,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7416.5785,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1879.8319999999999,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 429.14549999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.3185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 428.85450000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1896.71,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 415.06100000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.44149999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 409.84799999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2937.104,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 428.56949999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 109.657,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 430.98400000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2913.978,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 416.047,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 91.6575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 415.58950000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14792.011999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1837.8055,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 432.701,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 109.342,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 428.43600000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 205.94799999999998,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 182.6855,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 95.5305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 186.2585,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 695.4404999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 450.185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 151.202,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 450.302,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5875,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.5145,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.04,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.798,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.8935,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5469.672500000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 88.69300000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 69.38650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 67.7755,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 109.89,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 67.5865,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759332055976,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7495.6855,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7531.3099999999995,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1878.663,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 441.7495,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 110.842,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 439.1415,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1851.063,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 424.689,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.943,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 420.41499999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2931.31,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 437.4855,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 113.54050000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 441.927,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2898.7070000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 423.64700000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.2975,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 427.35450000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14821.235499999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1836.8474999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 440.4515,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 109.9075,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 439.192,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.8395,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 186.879,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.26050000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 190.32850000000002,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 708.5775,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 459.5775,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 156.125,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 456.976,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5825,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.557,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.463000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.026,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5511.646000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 92.9045,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.70500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.39949999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 114.2045,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.4975,
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
          "id": "a2dc2b40d542f767bb13622b2aa510493c993338",
          "message": "chore: Upgrade to `0.18.10` (#3250)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-01T10:50:42-04:00",
          "tree_id": "e98a6721b157dfa24946a6e5664f8b2009f68472",
          "url": "https://github.com/paradedb/paradedb/commit/a2dc2b40d542f767bb13622b2aa510493c993338"
        },
        "date": 1759333016903,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7382.419,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7379.6835,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1921.504,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 438.598,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.6875,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 438.39599999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1896.473,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 417.1005,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.06700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 420.2925,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2890.733,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 434.3655,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 109.251,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 439.594,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2864.2735000000002,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 419.12199999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.3975,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 422.468,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14811.0625,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1874.233,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 439.4045,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.624,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 438.08050000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.82,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 189.303,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 102.703,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 192.968,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 700.3344999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 451.953,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 161.714,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 457.278,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.45,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.188000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.769,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.765,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.0375,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5419.9925,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 93.998,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 73.883,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 72.048,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 122.951,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.39,
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759361486329,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7376.053,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7377.4005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1934.7995,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 424.9635,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 103.3305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 425.56,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1923.337,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 409.141,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 89.709,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 409.18050000000005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2906.3505,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 424.0025,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 106.1585,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 425.46450000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2875.6605,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 406.6445,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 90.952,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 408.53200000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14836.644,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1872.567,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 424.8605,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 103.202,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 425.709,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 215.81799999999998,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 183.18,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 94.9555,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 183.8435,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 683.794,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 443.733,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 152.378,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 442.8015,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.4575,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.33,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.7219999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8115000000000006,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.8485,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5366.6235,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.019,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 68.836,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.997,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 107.96000000000001,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 68.41,
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759364665592,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7467.143,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7466.869,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1903.1475,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 427.9225,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.80449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 428.09349999999995,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1891.623,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 413.7285,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.8195,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 412.5265,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2923.901,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 426.645,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 122.451,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 429.925,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2904.7805,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 414.28499999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 94.8575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 416.367,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14791.3865,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1871.8795,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 427.494,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.26650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 429.83500000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 224.01999999999998,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 186.909,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 100.8595,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 188.974,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 689.694,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 446.6135,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 156.83749999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 450.179,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5275,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.439,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.916,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.825,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.810500000000001,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5378.5715,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 94.042,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 74.9125,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 73.3715,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 123.413,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.409,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759370190558,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7404.796,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7390.005999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1899.9585000000002,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 425.19550000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.2795,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 426.2315,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1880.2475,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 423.33299999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.102,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 411.2345,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2945.114,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 422.692,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 108.559,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 424.84349999999995,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2951.548,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 416.924,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.2645,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 411.745,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14784.549500000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1873.2069999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 424.865,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 105.342,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 426.88300000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 214.08800000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 184.611,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 96.24199999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 188.6125,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 690.198,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 450.521,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 153.574,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 446.69849999999997,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.7044999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.3740000000000006,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.93,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.756,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.997,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5371.1215,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 91.382,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 71.99799999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 71.13499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 122.4755,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.463,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759372356188,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7360.075000000001,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7457.496999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1921.6765,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 428.626,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.287,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 440.7835,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1896.701,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 425.3115,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.595,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 413.25,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2955.0455,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 425.817,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 108.482,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 439.966,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2917.059,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 426.562,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 94.50200000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 417.5985,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14826.633,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1886.5005,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 428.322,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.3685,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 440.1605,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 223.4505,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 188.8695,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 100.23400000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 192.004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 692.149,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 459.58349999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 158.464,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 449.6545,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5809999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.3785,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.9094999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.788,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.214,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5425.7955,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 94.0575,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 74.4255,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 74.0085,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 123.607,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.581,
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759415106578,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7422.437,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7409.743,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1942.8384999999998,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 432.34450000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.5155,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 432.58399999999995,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1906.4075,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 417.6625,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 93.464,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 416.7225,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2937.4849999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 430.7385,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 117.51599999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 432.69100000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2927.3734999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 415.305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 95.12700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 418.65,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14808.86,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1888.356,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 432.4445,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.68299999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 432.7525,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 217.2305,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 191.5395,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 103.09100000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 194.7265,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 694.7280000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 451.21950000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 160.0215,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 452.37350000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.441,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.49,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.7615,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.994,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5484.834,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 93.6545,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 74.352,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 75.31700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 118.9485,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.466,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759415361436,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7734.871499999999,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7757.374,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1868.9995,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 470.4675,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 106.5195,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 467.05949999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1838.123,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 416.6775,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 93.74000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 414.413,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 3309.384,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 466.721,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 124.8955,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 468.74649999999997,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 3279.32,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 414.0385,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 95.5305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 420.61850000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14849.3445,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 2068.617,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 469.712,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.4375,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 469.422,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.88850000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 194.6055,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 100.961,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 198.34449999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 703.866,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 449.65999999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 159.188,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 449.9285,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.728,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.4005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.167,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.87,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.016,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5402.112,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 93.5935,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 73.72,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 75.07050000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 112.434,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 74.138,
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
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759525877558,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7916.252,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7918.746999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1898.989,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 437.775,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 111.10650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 441.27,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1913.7765,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 415.6835,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 94.812,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 416.76649999999995,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2913.3695,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 444.18600000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 110.4525,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 440.3355,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2912.7174999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 416.696,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 95.9245,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 421.1775,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14829.823499999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1882.382,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 437.55600000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 110.965,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 438.00350000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 215.383,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 189.24599999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 98.86850000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 188.473,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 730.5345,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 453.922,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 155.7885,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 456.7,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.444000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.3475,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.8955,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8435,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.767,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5374.407499999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.9755,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 71.902,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 72.95,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 118.8425,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.13650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759857596133,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7382.597,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7382.2275,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1961.824,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 426.5675,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.935,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 424.33050000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1983.8535000000002,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 407.6575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.53999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 405.654,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 3007.0695,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 425.0695,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 118.1965,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 428.00350000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2993.266,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 405.0465,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 91.7305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 407.36199999999997,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14775.888500000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1891.7675,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 422.706,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 424.1915,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 223.082,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 189.06099999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 97.8885,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 187.61149999999998,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 705.572,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 442.251,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 157.01600000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 441.012,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5895,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.525,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.8315,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.756,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.989,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5444.781,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.344,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 71.96600000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.1175,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 128.964,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.584,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759867095760,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7493.491,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7488.196,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1873.8095,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 426.07349999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 110.048,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 428.56600000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1887.3445000000002,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 417.089,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.686,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 406.954,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2914.273,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 427.1685,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 112.5315,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 428.0215,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2899.587,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 422.517,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.6925,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 409.8885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14823.8665,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1839.613,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 426.2765,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 108.422,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 429.0525,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 215.9975,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 186.0795,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 98.687,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 191.0725,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 702.798,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 453.1635,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.497,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 444.4275,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.6995000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.52,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.381,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.9755,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.0135,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5445.9655,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 92.4855,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.2615,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.2225,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 127.174,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.55449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759867172446,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7436.389499999999,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7441.509,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1901.3315,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 442.2075,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.9305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 461.47,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1893.6715,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 428.7595,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.978,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 426.532,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2910.281,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 441.85900000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 119.68100000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 440.132,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2908.561,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 435.4955,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.569,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 427.58,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14786.425,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1883.0275000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 440.93449999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.62950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 460.4105,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 217.9495,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 189.461,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.875,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 191.0625,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 715.867,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 461.754,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 156.6925,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 463.7315,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.575,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.434,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.9094999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8385,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.8415,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5497.6355,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 91.6695,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.9065,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 71.0145,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 117.7595,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.6995,
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
          "id": "96f6d1fa999bb11ba37313786636681219629534",
          "message": "chore: revert \"chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\"\n\nThis reverts commit 34e2d538b7327fc30b32f6ee33b55cbc9ccb2749.\n\n\nRequested by @eeeebbbbrrrr due to conflicts with the SQL UX work, will\nre-open later",
          "timestamp": "2025-10-08T13:42:37-04:00",
          "tree_id": "9db0fcd9402217db0c7f51702ef447664a0831f4",
          "url": "https://github.com/paradedb/paradedb/commit/96f6d1fa999bb11ba37313786636681219629534"
        },
        "date": 1759946719296,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7429.7055,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7430.66,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1869.4085,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 433.73400000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 109.815,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 436.2555,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1859.7005,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 416.9205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.644,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 418.0395,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2956.848,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 436.82,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 112.914,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 438.815,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2950.5885,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 419.778,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 94.95949999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 419.697,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14837.997,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1823.058,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 433.926,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.8365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 433.988,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 216.88400000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 187.297,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 96.8385,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 189.9135,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 700.4045,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 453.0235,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.054,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 454.038,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.6105,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.4855,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.022,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.660500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.840499999999999,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5425.5875,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 91.046,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.75999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 72.819,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 122.10849999999999,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 69.923,
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
          "id": "aadb55cde7a3b88a751cfa9f4250928a07a58590",
          "message": "feat: added aggregate FILTER support using Tantivy's FilterAggregation feature (#3240)\n\n# Ticket(s) Closed\n\n- Closes #3136\n\n## What\n\nImplements SQL `FILTER` clause support for aggregations using Tantivy's\n`FilterAggregation` feature.\n\n```sql\n-- Multiple filtered aggregates in a single query scan\nSELECT \n  COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics,\n  COUNT(*) FILTER (WHERE category @@@ 'books') AS books,\n  AVG(price) FILTER (WHERE status @@@ 'available') AS avg_available_price\nFROM products\nGROUP BY brand;\n```\n\n## Why\n\n**Before**: Computing multiple filtered aggregations required separate\nqueries (multiple index scans).\n\n**After**: Standard SQL `FILTER` syntax with single index scan,\nregardless of number of filters. Significant performance improvement for\nanalytical queries.\n\n## How\n\n### Core Changes\n\n1. Query Planning: extracted `FILTER` clauses from aggregate nodes,\ngroup identical filters for optimization\n2. Execution: used `FilterAggregation` structure for all aggregates\n(filtered and non-filtered)\n3. Result Processing: to handle nested JSON output\n4. Parallelization: supporting both SQL `FILTER` and legacy JSON API\n\n### Key Implementation Details\n\n- All aggregates wrapped in `FilterAggregation` (non-filtered use\n`MatchAllQuery`)\n- Sentinel filter ensures all `GROUP BY` groups appear (correct NULL/0\nhandling)\n- `ExprContextGuard` RAII wrapper for safe resource management\n- Cross-type numeric comparison support for multi-column `GROUP BY`\n\n### Optimization Strategy\n\nAggregates with identical filters are grouped together. Example:\n```\n3 aggregates, 2 filters → 2 filter groups → single Tantivy query with 2 FilterAggregations\n```\n\n## Tests\n\nAdded regression tests covering:\n- Simple & GROUP BY aggregations with FILTER\n- Multiple FILTER clauses\n- Multi-column GROUP BY with mixed filtered/non-filtered aggregates\n- Edge cases: empty results, NULL handling, ORDER BY preservation\n\n**Next steps**:\n - qgen tests\n - Performance benchmarking",
          "timestamp": "2025-10-09T02:27:04-07:00",
          "tree_id": "cedee10645583187cd9d4d8c6f356295ce57a9ae",
          "url": "https://github.com/paradedb/paradedb/commit/aadb55cde7a3b88a751cfa9f4250928a07a58590"
        },
        "date": 1760003517520,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7372.535,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7370.6385,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1924.274,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 441.2305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 109.0915,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 873.9894999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1913.506,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 421.914,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 94.478,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 866.8195000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2899.4885000000004,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 429.5295,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 110.977,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 879.8325,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2900.6710000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 430.05899999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 96.62,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 866.0805,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14794.345000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1863.341,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 442.39700000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 108.768,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 516.9675,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 222.17849999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 193.856,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 98.9585,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 213.7475,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 695.1905,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 455.66700000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 158.49099999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 523.1535,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.4665,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.529999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.929,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.798,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.1235,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5469.315500000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 93.76650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 74.4795,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 73.23349999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 120.184,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.7285,
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
          "id": "90ee6969e060cb3f1bcc852d006e4efcf38c0449",
          "message": "chore: Upgrade to `0.19.0` (#3286)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T09:44:08-04:00",
          "tree_id": "0835074ee03c85d566877553a3a60b056b2c656b",
          "url": "https://github.com/paradedb/paradedb/commit/90ee6969e060cb3f1bcc852d006e4efcf38c0449"
        },
        "date": 1760019004105,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7959.5885,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7963.8745,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1922.343,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 428.906,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.8825,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 878.5775000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1911.804,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 411.904,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.422,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 848.0915,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2921.014,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 429.5375,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 110.727,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 870.9125,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2892.3475,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 411.515,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.363,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 842.2405,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14786.194,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1877.835,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 428.655,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.416,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 504.517,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.08249999999998,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 191.0165,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.5745,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 211.6125,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 708.8230000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 443.11199999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 156.9995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 509.6885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.390499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.3945,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.9345,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.725,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.7875,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5385.284,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 92.987,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.40950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 71.23750000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 116.8045,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.571,
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
          "id": "10de316db45e848a0bc1c1ae86f820de701030f1",
          "message": "chore: show readable error if serving reads from hot standby (#3267)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nShow a better error message if trying to serve reads on a hot standby\nfrom ParadeDB Community, which does not support WALs -- only Enterprise\ndoes.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T11:48:50-04:00",
          "tree_id": "da13ba2361322103116c1d498ec45f15ad86908c",
          "url": "https://github.com/paradedb/paradedb/commit/10de316db45e848a0bc1c1ae86f820de701030f1"
        },
        "date": 1760026681310,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7434.448,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7451.6,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1902.071,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 431.3515,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.38550000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 877.2785,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1900.963,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 405.848,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 89.512,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 819.479,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2923.4925,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 432.172,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 111.43199999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 874.444,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2897.6315000000004,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 404.884,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 91.499,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 818.7574999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14789.9155,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1870.635,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 429.902,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.35249999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 492.246,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.112,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 186.9125,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.2235,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 207.1985,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 706.3425,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 438.174,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.4245,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 499.4605,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.7385,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.6205,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.018,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.5895,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.1995,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5453.7080000000005,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.862,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.90950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.523,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 119.3435,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.01050000000001,
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
          "id": "3d66fbeefb193965a576f504a7f578024eeee5e7",
          "message": "feat: standardize EXPLAIN output formatting with ExplainFormat trait (#3268)\n\n## Summary\n\nThis PR introduces the `ExplainFormat` trait to standardize how objects\nare formatted for PostgreSQL `EXPLAIN` output across the codebase. This\neliminates duplicate implementations and ensures consistent,\ndeterministic output for regression tests.\n\n## Changes\n\n- `ExplainFormat` trait: common interface for formatting objects for\nEXPLAIN output\n- `cleanup_json_for_explain()`: Removes non-deterministic data (OIDs,\npointers) from JSON for consistent test output\n- `format_for_explain()`: Convenience function for serializable objects\n\n### Refactoring\n- Removed duplicated formatting functions:\n- Deprecated `SearchQueryInput::canonical_query_string()` (now uses\n`ExplainFormat`)\n- Deprecated standalone `cleanup_variabilities_from_tantivy_query()`\n(now uses `cleanup_json_for_explain()`)\n\n- Implemented `ExplainFormat` for:\n  - `SearchQueryInput` - query formatting\n  - `AggregateType` - aggregate formatting with FILTER clauses\n\n- Updated `Explainer`:\n- New `add_explainable()` method accepts any `ExplainFormat` implementor\n  - Simplified `add_query()` to use the new trait",
          "timestamp": "2025-10-09T09:00:38-07:00",
          "tree_id": "3376b11c73acc94fb95af94e2ff1048b3579e3da",
          "url": "https://github.com/paradedb/paradedb/commit/3d66fbeefb193965a576f504a7f578024eeee5e7"
        },
        "date": 1760027041901,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7670.509,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7484.2815,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1895.8564999999999,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 433.025,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 109.0575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 871.8465,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1896.943,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 404.33349999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 93.18549999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 833.896,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2927.1639999999998,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 426.722,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 110.869,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 868.3225,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2888.7394999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 401.50350000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.902,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 830.1775,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14816.4705,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1856.9634999999998,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 429.185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 109.29650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 496.6555,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 210.4235,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 187.7105,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 97.40299999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 206.495,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 682.3175,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 439.43600000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 155.692,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 502.979,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5335,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.4395,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.9185,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.6775,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.747499999999999,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5550.807,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 89.94149999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.14150000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.79650000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 117.75450000000001,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.84649999999999,
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
          "id": "76c4decb23962ae948fff00b4b64958c5b631bbf",
          "message": "feat: support logical replication in community (#3264)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUntil now, logical replication has been an enterprise-only feature. We\nhave made the decision to move it to community, which allows anyone to\nrun ParadeDB as a logical replica of a primary Postgres.\n\nNote: Logical replication is only supported for Postgres 17 and newer.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T11:11:49-04:00",
          "tree_id": "85d200bc61f1a460e3f621edae3ca55e6a027145",
          "url": "https://github.com/paradedb/paradedb/commit/76c4decb23962ae948fff00b4b64958c5b631bbf"
        },
        "date": 1760030361320,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7967.9625,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7883.095499999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1898.047,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 431.3365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.94749999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 875.729,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1894.9740000000002,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 406.9465,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.44,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 841.842,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2910.1035,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 435.512,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 111.82,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 868.491,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2908.734,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 406.596,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 91.38499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 827.835,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14793.3065,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1866.5245,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 433.749,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.932,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 507.16650000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 217.7265,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 189.67849999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 100.435,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 210.3335,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 689.6265,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 442.1395,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.9035,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 508.72950000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.554,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.4365000000000006,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.906000000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.785,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.6445,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5598.2474999999995,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 92.711,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 73.30449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.8195,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 121.2305,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 72.47999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "60b5f95971b616bb8c197b99797555d944c3628b",
          "message": "feat: tokenizers-as-types (#3290)\n\n## What\n\nThis PR introduces first-class tokenizer types, inline tokenization via\ncasts, stricter validation and planning around tokenizer usage, and new\nbuilder functions/operators to improve query ergonomics and execution\nplanning.\n\n### New Types\n- Tokenizer SQL types with CREATE TYPE definitions and IO functions are\ngenerated via a new macros::generate_tokenizer_sql proc-macro.\n- Tokenizer types are flagged as category “t” (tokenizer), with LIKE\ntext, collatable, and can be marked PREFERRED.\n- JSON/JSONB assignment casts to tokenizer types.\n\n### Typmods\n- Generic typmod support for tokenizer types when custom typmod is not\nprovided (TYPMOD_IN/OUT wired to generic handlers).\n- Dedicated parsing for specialized typmods (e.g., fuzzy parameters).\n- Ability to attach an alias via typmod for indexed expressions; alias\nmust be present when a tokenizer is used in an indexed expression.\n\n### Casting to TEXT[] for inline tokenization\n- Each tokenizer type supports cast to TEXT[] for immediate\ntokenization, enabling:\n  - 'text'::pdb.simple::text[]\n  - 'text'::pdb.ngram(3,5)::text[]\n  - 'text'::pdb.lindera(japanese)::text[]\n  - and similar casts for all supported tokenizer types\n- JSON/JSONB values can be assignment-cast to tokenizer types and then\nto TEXT[].\n\n### CREATE INDEX examples\n- Tokenizer casts in index definitions now require an alias typmod:\n  - (t::pdb.simple('alias=simple'))\n  - (t::pdb.ngram(3,5, 'alias=ngram'))\n  - (t::pdb.lindera(japanese, 'alias=lindera_japanese'))\n- Both direct attribute indexing and expression indexing are supported;\nwhen indexing expressions using tokenizer types, an alias is required to\nbind the field name for planning.\n- Planner can identify fields from:\n  - direct Vars,\n  - tokenizer expressions with alias,\n  - JSON path references (e.g., json_field->'a'->>'b' becomes “a.b”),\n- unaliased tokenizer expressions only when safely resolvable to a\nsingle Var and generic typmod has no alias specified.\n\n## Why\n- Provide clear, typed surface for tokenizers, improving safety,\ndiscoverability, and SQL UX.\n- Enable inline tokenization for testing, debugging, and composed SQL\nexpressions.\n- Ensure indexed expressions using tokenizers are unambiguous at plan\ntime, avoiding runtime surprises.\n- Broaden operator LHS type compatibility (text, varchar, arrays,\njson/jsonb, tokenizer types) with explicit validation and better error\nmessages.\n\n## How\n- New macros:\n  - builder_fn refactored into its own module.\n- generate_tokenizer_sql macro emits CREATE TYPE, casts, and optional\ntypmod wiring.\n- Operator planning:\n  - Expanded resolution of field names from complex nodes.\n- Enforced alias requirement for tokenizer expressions in indexes unless\nsafely resolvable.\n- Validation that LHS types are text-compatible or tokenizer-compatible.\n- Request-simplify code paths updated to carry LHS through const/exec\nrewrites and to wrap RHS with index info.\n- Builder functions:\n- Added array-based query builders (match_conjunction_array,\nmatch_disjunction_array, phrase_array).\n- Operators now have overloads to accept TEXT[], pdb.query, boost,\nfuzzy, etc., with shared support function.\n\n## Tests\n- Inline tokenization: casts of literals through various tokenizer types\nto TEXT[].\n- CREATE INDEX with tokenizer types: examples covering all tokenizers\nwith alias typmods and EXPLAIN plans using those aliases.\n- Enforcement: indexing a tokenizer expression without alias raises a\nclear error.\n- RHS tokenizer casts: @@@ does not accept tokenizer-cast RHS; &&&, |||,\n###, === do accept and are validated.\n\n---------\n\nCo-authored-by: Ming Ying <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-09T14:03:50-04:00",
          "tree_id": "142dd25a67167915628ac8b52e0056b31bb70b64",
          "url": "https://github.com/paradedb/paradedb/commit/60b5f95971b616bb8c197b99797555d944c3628b"
        },
        "date": 1760034490422,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7380.9265,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7394.323,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1927.717,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 432.889,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.19999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 856.1855,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1896.4885,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 407.88149999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 93.3035,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 835.506,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2974.239,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 423.9205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.54849999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 860.5575,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2877.3725,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 410.9975,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 95.991,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 839.614,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14792.355,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1881.1755,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 432.805,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 105.16,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 499.9955,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.825,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 186.111,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 97.2835,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 207.7095,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 685.1220000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 442.296,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 152.058,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 509.5785,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.66,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.480499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.975,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.7845,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.928,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.5834999999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.36449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 71.941,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.3755,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 109.8925,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 68.4375,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8a3ca3d2629465260e3d38de91714d4fa3afb0a8",
          "message": "feat: support the `stopwords_language` typmod property (#3299)\n\n## What\n\nThis adds back in support for the language-specific built-in list of\nstopwords, using `stopwords_language=xxx`.\n\n## Why\n\nStopwords are pretty useful!\n\n## How\n\n## Tests\n\nA regression test has been added.",
          "timestamp": "2025-10-09T15:29:20-04:00",
          "tree_id": "9ed34db181fb1282dd50b9ddde0b93999a79f19a",
          "url": "https://github.com/paradedb/paradedb/commit/8a3ca3d2629465260e3d38de91714d4fa3afb0a8"
        },
        "date": 1760039507261,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7348.054,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7350.9095,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1893.975,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 424.602,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.95150000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 878.499,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1855.6205,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 407.7065,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.73599999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 849.0885,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2928.1994999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 434.185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 110.0895,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 891.721,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2915.1145,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 409.8505,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.71600000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 856.8620000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14808.6075,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1837.4769999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 424.5435,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 105.544,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 512.4010000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.126,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 187.6375,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 97.455,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 205.21,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 701.99,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 445.82550000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.282,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 514.5905,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.875,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.683,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.1855,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8745,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.253499999999999,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.6045,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 89.7525,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.9225,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.92500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 122.1425,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 68.6165,
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
          "id": "0a691cd226f9b335af5591dd84e695ec4da92df3",
          "message": "chore: Un-revert remove deprecated tokenizers (#3306)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nBrings back https://github.com/paradedb/paradedb/pull/3279 which was\nreverted\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T16:18:45-04:00",
          "tree_id": "9f4e6764d3c00556eadd693db305527cbf8a2342",
          "url": "https://github.com/paradedb/paradedb/commit/0a691cd226f9b335af5591dd84e695ec4da92df3"
        },
        "date": 1760042476346,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7491.69,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7511.83,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1855.722,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 446.605,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 109.775,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 898.3775,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1834.3984999999998,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 431.171,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.55250000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 863.6310000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2935.9300000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 465.226,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 113.9795,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 915.3795,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2893.635,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 429.8325,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.56299999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 882.6165000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14849.3445,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1840.3139999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 446.9915,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 108.766,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 519.144,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 220.759,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 186.01049999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 96.5755,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 205.1395,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 729.586,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 463.00300000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 151.138,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 524.777,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.718,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.654,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.3935,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.0785,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.118500000000001,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.586,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 89.6725,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.133,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.5475,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 111.42500000000001,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 68.89850000000001,
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
          "id": "42671fe0df806d6033d837f7b058b0b6f81b187d",
          "message": "chore: Remove default `255` byte \"remove long\" filter setting (#3288)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nA 255 byte \"remove long\" setting was applied to all text fields.\n\nThis was wrong for text fast fields -- any text longer than 255 bytes\nwas being silently dropped. And it's confusing to document for non fast\ntext fields, we should be preserving the tokens and let the user opt\ninto it rather than doing it by default.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T16:49:35-04:00",
          "tree_id": "c1ccc15293d114edb6d9d98bc04d99512d1a539d",
          "url": "https://github.com/paradedb/paradedb/commit/42671fe0df806d6033d837f7b058b0b6f81b187d"
        },
        "date": 1760044320498,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7624.1105,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7623.0095,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1900.666,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 436.27250000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 108.536,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 865.8365,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1877.4479999999999,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 419.9385,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.602,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 835.718,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2890.07,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 438.2305,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 113.0375,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 881.412,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2884.976,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 419.3185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.17349999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 850.9425,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14787.535,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1856.7865,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 436.6685,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 108.6,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 509.92650000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 218.7785,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 194.56099999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.9995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 212.683,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 707.785,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 458.07349999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 157.4085,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 521.1199999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.385,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.6635,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.781499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8665,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.035,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.5965,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.86099999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.5035,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 71.62700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 119.951,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.5525,
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
          "id": "98baf4ce05a4907c305bcbb28438d5fcc6025f09",
          "message": "chore: Remove dead code in `tokenizers/src/manager.rs` (#3310)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-10T09:28:36-04:00",
          "tree_id": "ae706715e3523d0f2bca3de4c17c19661812c2c8",
          "url": "https://github.com/paradedb/paradedb/commit/98baf4ce05a4907c305bcbb28438d5fcc6025f09"
        },
        "date": 1760104266107,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7643.3904999999995,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7653.362,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1864.5765000000001,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 442.419,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.2825,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 905.3315,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1847.576,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 429.348,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.3955,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 874.9100000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2901.1994999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 437.9635,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 112.166,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 919.202,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2872.6364999999996,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 427.4825,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.138,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 894.54,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14803.9655,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1816.7150000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 439.7555,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.48,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 520.636,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 221.333,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 188.549,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 96.298,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 209.5285,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 722.5515,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 465.291,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.63,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 532.039,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.6080000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.638999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.2835,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.089,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.657,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.577,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 89.9035,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.953,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.91300000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 107.991,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.096,
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
          "id": "d83f2888d60760ce3ab8c71ac24749d018604ce7",
          "message": "feat: Introduce `TokenLengthFilter` (#3309)\n\n# Ticket(s) Closed\n\n- Closes #3280 \n\n## What\n\nIntroduces a `TokenLengthFilter`, which is like Tantivy's\n`RemoveLongFilter` but has a min and max threshold. This enables a new\nfilter, `remove_short`, which removes tokens below a certain byte\nthreshold.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-10T09:38:04-04:00",
          "tree_id": "51697ab88c096b3ab284e210fec8ec1eb9b425c4",
          "url": "https://github.com/paradedb/paradedb/commit/d83f2888d60760ce3ab8c71ac24749d018604ce7"
        },
        "date": 1760104834861,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7429.076999999999,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7419.7635,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1908.499,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 429.4365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.4385,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 865.35,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1929.2945,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 415.02750000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.33850000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 842.6605,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2928.8355,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 431.12350000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.087,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 879.67,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2895.7655,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 414.226,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.106,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 840.6165,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14817.367,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1940.8899999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 428.415,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 105.8685,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 508.32849999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.24099999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 188.31799999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 97.9205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 207.36,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 692.111,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 449.6005,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.58800000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 514.1785,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.495,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.5345,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.9855,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.8185,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.752500000000001,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.61,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 91.0325,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.1695,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.95400000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 111.1765,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.206,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "84ace13610bc16ef88495a07915708cbe371e4a0",
          "message": "feat: add new `##>` operator for \"in order\" proximity (#3314)\n\n# Ticket(s) Closed\n\n- Closes #3305\n\n## What\n\nSimilar to the existing `##` operator, but `##>` requires the rhs be\nthat many tokens after the lhs.\n\nExample:\n\n```sql\n... WHERE text @@@ ('z' ##>24##> 'a');\n```\n\n## Why\n\n## How\n\n## Tests\n\nUpdated the `proximity.sql` test with a small test.",
          "timestamp": "2025-10-10T10:25:54-04:00",
          "tree_id": "576dd1e98eb7884b79985358e642deb5df8beda0",
          "url": "https://github.com/paradedb/paradedb/commit/84ace13610bc16ef88495a07915708cbe371e4a0"
        },
        "date": 1760107744665,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7484.081,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7474.5765,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1866.377,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 428.103,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 106.643,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 892.315,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1869.161,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 414.2625,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.66300000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 859.1189999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2972.7174999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 431.3365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.849,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 901.0754999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2917.937,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 417.072,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 93.13300000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 866.8689999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14811.1375,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1857.393,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 430.93600000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.797,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 517.046,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 216.1705,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 187.1285,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 98.21600000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 206.1485,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 749.277,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 447.99,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 153.969,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 522.7394999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.7345,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.85,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.2775,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.933999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.810500000000001,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.609,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 89.234,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.61949999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.4235,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 100.78049999999999,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 69.84649999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ec9ff340a70f67a810bbd68f1032786a6a70171f",
          "message": "fix: allow the RHS of `@@@` to support `pdb.query` at runtime (#3308)\n\n# Ticket(s) Closed\n\n- Closes #3300\n\n## What\n\nThis allows the rhs of `@@@` to support a `pdb.query` at runtime\n\n## Why\n\n## How\n\n## Tests\n\nYes",
          "timestamp": "2025-10-10T10:37:24-04:00",
          "tree_id": "5e8a7b39c086133de1e3b584061cf342da72a619",
          "url": "https://github.com/paradedb/paradedb/commit/ec9ff340a70f67a810bbd68f1032786a6a70171f"
        },
        "date": 1760108396305,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7621.2025,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7639.404,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1915.0214999999998,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 432.5915,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 106.473,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 880.8045,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1908.9569999999999,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 416.6375,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.34450000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 837.54,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2907.7389999999996,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 435.236,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.38550000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 877.133,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2885.9700000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 414.20500000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.495,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 861.7090000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14811.265,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1877.6275,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 429.64750000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 105.334,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 511.78499999999997,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 216.981,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 191.9205,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 100.45,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 209.852,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 699.3225,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 453.8055,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 153.015,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 519.592,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.615,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.5024999999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.9795,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.9155,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.7195,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.603,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 91.05799999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 73.2005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.92699999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 110.368,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.0955,
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
          "id": "051f1c91e005cf94328dff488aaa4ba9b9b89703",
          "message": "feat: Introduce `AlphaNumOnly` filter (#3315)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nA new filter that removes tokens containing non ASCII letters and\ndigits.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-10T10:48:38-04:00",
          "tree_id": "95691fe3913334da5c80c710f3451542e0fa7327",
          "url": "https://github.com/paradedb/paradedb/commit/051f1c91e005cf94328dff488aaa4ba9b9b89703"
        },
        "date": 1760110038911,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7374.073,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7374.066000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1962.1165,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 432.5745,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 106.6275,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 859.6845000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1931.7845000000002,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 416.038,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 89.27600000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 834.8695,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2916.3015,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 434.347,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 111.95150000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 865.539,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2877.282,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 416.51099999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 91.599,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 847.3969999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14789.1885,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1869.912,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 433.9315,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 107.654,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 505.323,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 219.256,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 188.67700000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 97.7415,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 209.792,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 706.8905,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 453.12850000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 152.6215,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 518.2335,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.702999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.551,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.917999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.9435,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.833,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.6074999999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 89.5375,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.90299999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 68.957,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 124.3075,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 68.43950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ec9ff340a70f67a810bbd68f1032786a6a70171f",
          "message": "fix: allow the RHS of `@@@` to support `pdb.query` at runtime (#3308)\n\n# Ticket(s) Closed\n\n- Closes #3300\n\n## What\n\nThis allows the rhs of `@@@` to support a `pdb.query` at runtime\n\n## Why\n\n## How\n\n## Tests\n\nYes",
          "timestamp": "2025-10-10T10:37:24-04:00",
          "tree_id": "5e8a7b39c086133de1e3b584061cf342da72a619",
          "url": "https://github.com/paradedb/paradedb/commit/ec9ff340a70f67a810bbd68f1032786a6a70171f"
        },
        "date": 1760110049075,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7524.9575,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7523.063,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1931.2150000000001,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 436.3355,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 108.5895,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 866.1735,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1940.7330000000002,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 418.61249999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.6635,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 830.9124999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2891.984,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 440.23900000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 110.5295,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 879.44,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2901.1995,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 420.754,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.10249999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 838.2825,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14793.3745,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1897.2255,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 442.981,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 108.12950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 510.99699999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 222.76850000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 192.6365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 102.4345,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 211.6425,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 705.685,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 454.681,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 153.9385,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 521.184,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.67,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.653499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.7595,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.741,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.7895,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.5905,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 90.50800000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.426,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 70.6585,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 126.8125,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.2,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8269d4ed9b0f6101d4627bc72eb008026312fdac",
          "message": "chore: relocate `fuzzy`, `slop`, `boost` types to the `pdb` schema (#3316)\n\n# Ticket(s) Closed\n\n- Closes #3281\n\n## What\n\nThis relocates the `fuzzy`, `slop`, and `boost` types from `pg_catalog`\nto our `pdb` schema.\n\n🚨This will be a breaking change for any existing queries that cast the\nrhs of one of our operators to one of these types. They'll need to be\nrewritten as `::pdb.fuzzy`.\n\n## Why\n\nPutting them in `pg_catalog` was a mistake and inconsistent with our\nother SQL UX work.\n\n## How\n\n## Tests\n\nExisting tests (and docs) have been updated.",
          "timestamp": "2025-10-10T10:51:05-04:00",
          "tree_id": "4a988e0eff7dab9eec493a7ab5f16feb53bb4929",
          "url": "https://github.com/paradedb/paradedb/commit/8269d4ed9b0f6101d4627bc72eb008026312fdac"
        },
        "date": 1760110066586,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7466.452499999999,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7456.8405,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1878.0855000000001,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 436.9675,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 108.5015,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 900.434,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1882.269,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 429.39549999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 91.044,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 864.9205,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2948.3140000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 441.1805,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 111.036,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 906.8565000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2922.6525,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 427.751,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 92.46799999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 872.739,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 15257.392,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1817.74,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 438.0485,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 109.18199999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 507.8985,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "count-filter",
            "value": 211.4395,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 187.7935,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 96.5835,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 204.2505,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 704.4545,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 468.0985,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 150.16899999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 533.4735000000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.729,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.728,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.361,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.022,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 9.283,
            "unit": "median ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.623,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-compound",
            "value": 88.673,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 70.7045,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 68.536,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 117.8385,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 67.5355,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      }
    ],
    "pg_search 'docs' Query Performance": [
      {
        "commit": {
          "author": {
            "name": "paradedb[bot]",
            "username": "paradedb-bot",
            "email": "developers@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "7caf32d33552263a7408c43c2c057dad25c77222",
          "message": "chore: Upgrade to `0.18.9` (#3234)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>\nCo-authored-by: Philippe Noël <21990816+philippemnoel@users.noreply.github.com>",
          "timestamp": "2025-09-26T20:26:43Z",
          "url": "https://github.com/paradedb/paradedb/commit/7caf32d33552263a7408c43c2c057dad25c77222"
        },
        "date": 1758928245859,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1195.245,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 662.918,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1484.5855,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 723.043,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 696.348,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1598.5385,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 24.026,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.92699999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.65950000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba",
          "message": "chore: upgrade to `0.18.0` (#2980)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote: `cargo.toml` is already on `0.18.0` so this is docs-only\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T19:09:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba"
        },
        "date": 1758928369292,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1182.4915,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 655.6755,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1464.0955,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 715.4635,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 692.04,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1610.4975,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.2515,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.02850000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.9695,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48",
          "message": "ci: fix benchmark links in slack messages (#2875)",
          "timestamp": "2025-07-17T12:13:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/1c48d3c6427e9fe7bdb10861a16e04d0cf1a1f48"
        },
        "date": 1758928418185,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1199.9740000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 821.921,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1487.373,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 893.706,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 863.5295000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 2651.0315,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 33.938,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 63.076,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 92.2705,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "ec43ca34644c8c72e65ed2b7e9570066e839bab7",
          "message": "chore: Upgrade to `0.17.12` (#3120)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote that `0.17.11` was skipped because it was a hotfix in enterprise.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-09-03T20:56:05Z",
          "url": "https://github.com/paradedb/paradedb/commit/ec43ca34644c8c72e65ed2b7e9570066e839bab7"
        },
        "date": 1758928442613,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1218.8020000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 835.928,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1502.512,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 900.4535000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 877.2225000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 2700.8559999999998,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 35.921499999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 67.975,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 92.7515,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "d5d310fcc28c6c692cbbcf7f8b86f61e806434a5",
          "message": "feat: introduce ascii folding filter (#3241)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nIntroduces the ASCII filter.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-09-30T12:53:56-04:00",
          "tree_id": "ec6a192f3de17da7459676d2429d3b2f5640c7b5",
          "url": "https://github.com/paradedb/paradedb/commit/d5d310fcc28c6c692cbbcf7f8b86f61e806434a5"
        },
        "date": 1759253601050,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1203.3165,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 658.1565,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1490.519,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 726.497,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 694.8765000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1608.9769999999999,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 22.618000000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 63.793000000000006,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 86.584,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "da4cfff239fd8e2e318591df7095f4cac4987a4b",
          "message": "fix: Correctly handle `COUNT(<column>)` (#3243)\n\n# Ticket(s) Closed\n\n- Closes #3196 \n\n## What\n\nBefore, any `COUNT(<column>)` was getting rewritten to a count of the\n\"ctid\" field, which is incorrect because it doesn't correctly handle\nnull values.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression tests",
          "timestamp": "2025-09-30T16:35:47-04:00",
          "tree_id": "4e13feab234146d47e8e600f153bb9a27fe8383e",
          "url": "https://github.com/paradedb/paradedb/commit/da4cfff239fd8e2e318591df7095f4cac4987a4b"
        },
        "date": 1759266985308,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1203.2345,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 647.4435,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1484.0365,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 710.0575,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 683.5535,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1607.8885,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.1955,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.6455,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.797,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "52dd41fee48d3a635a315610631235ff09ac53a5",
          "message": "chore: Upgrade to `0.18.10` (#3251)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T10:56:42-04:00",
          "tree_id": "fca37808df8505e2c564f422c635f688f82a0e1d",
          "url": "https://github.com/paradedb/paradedb/commit/52dd41fee48d3a635a315610631235ff09ac53a5"
        },
        "date": 1759333079080,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1173.7795,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 649.515,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1483.7595000000001,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 709.971,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 686.187,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1613.5285,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.4965,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.49549999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.405,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "a2dc2b40d542f767bb13622b2aa510493c993338",
          "message": "chore: Upgrade to `0.18.10` (#3250)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-01T10:50:42-04:00",
          "tree_id": "e98a6721b157dfa24946a6e5664f8b2009f68472",
          "url": "https://github.com/paradedb/paradedb/commit/a2dc2b40d542f767bb13622b2aa510493c993338"
        },
        "date": 1759334037845,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1209.7265,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 654.6044999999999,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1508.5365000000002,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 716.0995,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 694.565,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1618.3095,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 24.027,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 67.108,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.715,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "bb494d330cfac7ee1db3b904ab5266bd671abadb",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3257)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-01T19:08:56-04:00",
          "tree_id": "a78e472aad70351c0c799699330da58256d3cbc4",
          "url": "https://github.com/paradedb/paradedb/commit/bb494d330cfac7ee1db3b904ab5266bd671abadb"
        },
        "date": 1759362499513,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1175.7984999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 664.2855,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1474.473,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 742.938,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 699.859,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1600.06,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 22.823,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 64.86699999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.1035,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "762c487685bb5635c789b31781155839d2c65cf0",
          "message": "feat: Configure a limit/offset for snippets (#3254)",
          "timestamp": "2025-10-01T20:00:17-04:00",
          "tree_id": "5dcb534f0e2f513864b19abddc44396ed24760ff",
          "url": "https://github.com/paradedb/paradedb/commit/762c487685bb5635c789b31781155839d2c65cf0"
        },
        "date": 1759365685951,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1214.4705,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 663.537,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1505.5795,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 728.357,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 700.9755,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1625.92,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 24.8675,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.2575,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.7285,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "850e3a9f88033d64151d6ecfa0d37c1b1f210b27",
          "message": "fix: `paradedb.snippet` projects correctly when called multiple times over the same field (#3258)\n\n# Ticket(s) Closed\n\n- Closes #3256 \n\n## What\n\nSee GH issue\n\n## Why\n\n## How\n\nWhen mapping a const node to our list of snippet generators, we were\nmatching only on the field name and function OID, which is not unique\nenough -- it doesn't account for the function arguments.\n\n## Tests\n\nAdded regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T21:33:38-04:00",
          "tree_id": "1f119fb29d59884bd2105114833ca2d34c2acfd9",
          "url": "https://github.com/paradedb/paradedb/commit/850e3a9f88033d64151d6ecfa0d37c1b1f210b27"
        },
        "date": 1759371193610,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1195.3745,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 660.712,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1480.2295,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 735.7139999999999,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 698.476,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1608.112,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.1245,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 64.17,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.351,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2ce078f8496ff0152f7373fe94348a9cfcacd5af",
          "message": "feat: Configure a limit/offset for snippets (#3259)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n`paradedb.snippet` and `paradedb.snippet_positions` now take a limit and\noffset. For instance, if 5 snippets are found in a doc and offset is 1,\nthen the first snippet will be skipped.\n\n## Why\n\n## How\n\n## Tests\n\nSee regression test\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-01T22:10:01-04:00",
          "tree_id": "904048e17b5e988830cd9302f007cb5d18411d22",
          "url": "https://github.com/paradedb/paradedb/commit/2ce078f8496ff0152f7373fe94348a9cfcacd5af"
        },
        "date": 1759373362096,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1202.897,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 664.2145,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1490.176,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 733.4705,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 703.014,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1600.0055,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 24.0025,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.186,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.506,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "6335204e0cc58bdeb1ea12f236ab2258a44cd192",
          "message": "chore: Upgrade to `0.18.11` (#3261)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-02T10:00:31-04:00",
          "tree_id": "05d18415099467cbf114e4a87c3b536bfeafb509",
          "url": "https://github.com/paradedb/paradedb/commit/6335204e0cc58bdeb1ea12f236ab2258a44cd192"
        },
        "date": 1759416121338,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1199.547,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 658.429,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1476.8405,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 719.5605,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 694.3879999999999,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1594.0695,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.401,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.274,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.4735,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759416385286,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1207.0435,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 655.5885000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1495.7705,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 721.8245,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 692.9559999999999,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1602.527,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.6575,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 64.7365,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.9585,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "6b15d1a8e21e76267b06b7e54dbbb5194fda55b9",
          "message": "chore: Improve determinism of property tests. (#3220)\n\n## What\n\nSet a seed to control what is generated by `random()` in the property\ntests, and render it in the reproduction script after a failure.\n\n## Why\n\nTo make it easier to reproduce property test failures by running over\nreproducible data.\n\n`proptest` failures are only directly reproducible via their reported\nseed if they are not dependent on the randomly generated data in the\ntable that we test against. We can't re-generate the table and index for\nevery `proptest` query that we run, because it would take way too long\nto run a reasonable number of iterations. And we don't want to run on\nstatic data, because then we might never catch data-dependent bugs like\n#3266.\n\nSetting a seed allows us to run on random data, but still reproduce\nfailures later. And for cases where failures aren't data-dependent, the\n`proptest` repro seed (e.g. `cc 08176a8c0ae10938a...`) can still be used\ndirectly.",
          "timestamp": "2025-10-03T13:48:13-07:00",
          "tree_id": "ac9934e860da0bdb5b4b083df652ebc3d85309d4",
          "url": "https://github.com/paradedb/paradedb/commit/6b15d1a8e21e76267b06b7e54dbbb5194fda55b9"
        },
        "date": 1759526895532,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1188.615,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 645.3420000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1500.682,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 705.2345,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 682.0305,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1603.0425,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.186500000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.3905,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 91.5175,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "developers@paradedb.com",
            "name": "paradedb[bot]",
            "username": "paradedb-bot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f394c9cc9b86839fb39f22123893552f6e5a291",
          "message": "chore: Upgrade to `0.18.11` (#3262)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ming <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-02T10:04:40-04:00",
          "tree_id": "fd395f6aeafa74dd786657eebbf6f9cba6e6bf28",
          "url": "https://github.com/paradedb/paradedb/commit/7f394c9cc9b86839fb39f22123893552f6e5a291"
        },
        "date": 1759858610553,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1203.399,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 648.1195,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1497.0155,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 706.73,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 682.092,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1621.925,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 22.6655,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.693,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 91.071,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b8ebd1ad157e946d6fff8f775aec3189bc469325",
          "message": "fix: possible naming collisions with builder functions (#3275)\n\n## What\n\nFix an issue where functions tagged with our `#[builder_fn]` macro could\nend up with the same name.\n\n## Why\n\nIt's come up in CI once or twice and I've seen it locally as well\n\n## How\n\n## Tests",
          "timestamp": "2025-10-07T15:35:22-04:00",
          "tree_id": "bd88762bb48211807276741ce46d155ea36600b3",
          "url": "https://github.com/paradedb/paradedb/commit/b8ebd1ad157e946d6fff8f775aec3189bc469325"
        },
        "date": 1759868118745,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1200.328,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 649.886,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1495.52,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 708.5485,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 682.7625,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1618.276,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.093,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 67.095,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 91.247,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2f61e5c2d8fb03377c37dcd10c558e9967781b97",
          "message": "feat: A Free Space Manager fronted by an AVL tree (#3252)\n\n## What\n\nThis implements a new `v2` FSM that's fronted by an AVL tree which\nallows for minimal locking during extension and draining. It also\nprovides efficient continuation during drain as xid blocklists are\nexhausted or found to be unavailable to the current transaction. And it\nimplements a (simple) transparent conversion of the current `v1` FSM to\nthe new format.\n\nAdditionally, this fixes a problem with background merging where more\nthan one background merger process could be spawned at once -- I've seen\nup to 8 concurrently. It does this by introducing some a new page on\ndisk to track the process and coordinate locking.\n\n## Why\n\nOur current FSM is very heavyweight in terms of lock contention. This\nshould get us to something that isn't.\n\n## How\n\n## Tests\n\nA number of new tests for the array-backed AVL tree and the FSM itself.\nAll existing tests also pass and, at least, the `wide-table.toml`\nstressgres shows a slight performance improvement for the update jobs.\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-10-07T15:36:47-04:00",
          "tree_id": "31410dd4c4d2be73d287e97485f4d0faaf1b2932",
          "url": "https://github.com/paradedb/paradedb/commit/2f61e5c2d8fb03377c37dcd10c558e9967781b97"
        },
        "date": 1759868183568,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1210.8135,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 661.7065,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1488.279,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 726.621,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 696.2755,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1614.813,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.141,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 64.9795,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.36500000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "96f6d1fa999bb11ba37313786636681219629534",
          "message": "chore: revert \"chore: Remove deprecated tokenizers: `en_stem`, `stem` and `lowercase` (#3279)\"\n\nThis reverts commit 34e2d538b7327fc30b32f6ee33b55cbc9ccb2749.\n\n\nRequested by @eeeebbbbrrrr due to conflicts with the SQL UX work, will\nre-open later",
          "timestamp": "2025-10-08T13:42:37-04:00",
          "tree_id": "9db0fcd9402217db0c7f51702ef447664a0831f4",
          "url": "https://github.com/paradedb/paradedb/commit/96f6d1fa999bb11ba37313786636681219629534"
        },
        "date": 1759947727050,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1201.163,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 647.4545,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1474.636,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 706.5215000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 681.877,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1608.489,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.158,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.30449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.44749999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "aadb55cde7a3b88a751cfa9f4250928a07a58590",
          "message": "feat: added aggregate FILTER support using Tantivy's FilterAggregation feature (#3240)\n\n# Ticket(s) Closed\n\n- Closes #3136\n\n## What\n\nImplements SQL `FILTER` clause support for aggregations using Tantivy's\n`FilterAggregation` feature.\n\n```sql\n-- Multiple filtered aggregates in a single query scan\nSELECT \n  COUNT(*) FILTER (WHERE category @@@ 'electronics') AS electronics,\n  COUNT(*) FILTER (WHERE category @@@ 'books') AS books,\n  AVG(price) FILTER (WHERE status @@@ 'available') AS avg_available_price\nFROM products\nGROUP BY brand;\n```\n\n## Why\n\n**Before**: Computing multiple filtered aggregations required separate\nqueries (multiple index scans).\n\n**After**: Standard SQL `FILTER` syntax with single index scan,\nregardless of number of filters. Significant performance improvement for\nanalytical queries.\n\n## How\n\n### Core Changes\n\n1. Query Planning: extracted `FILTER` clauses from aggregate nodes,\ngroup identical filters for optimization\n2. Execution: used `FilterAggregation` structure for all aggregates\n(filtered and non-filtered)\n3. Result Processing: to handle nested JSON output\n4. Parallelization: supporting both SQL `FILTER` and legacy JSON API\n\n### Key Implementation Details\n\n- All aggregates wrapped in `FilterAggregation` (non-filtered use\n`MatchAllQuery`)\n- Sentinel filter ensures all `GROUP BY` groups appear (correct NULL/0\nhandling)\n- `ExprContextGuard` RAII wrapper for safe resource management\n- Cross-type numeric comparison support for multi-column `GROUP BY`\n\n### Optimization Strategy\n\nAggregates with identical filters are grouped together. Example:\n```\n3 aggregates, 2 filters → 2 filter groups → single Tantivy query with 2 FilterAggregations\n```\n\n## Tests\n\nAdded regression tests covering:\n- Simple & GROUP BY aggregations with FILTER\n- Multiple FILTER clauses\n- Multi-column GROUP BY with mixed filtered/non-filtered aggregates\n- Edge cases: empty results, NULL handling, ORDER BY preservation\n\n**Next steps**:\n - qgen tests\n - Performance benchmarking",
          "timestamp": "2025-10-09T02:27:04-07:00",
          "tree_id": "cedee10645583187cd9d4d8c6f356295ce57a9ae",
          "url": "https://github.com/paradedb/paradedb/commit/aadb55cde7a3b88a751cfa9f4250928a07a58590"
        },
        "date": 1760004526625,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1201.464,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 653.367,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1483.8895,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 712.4604999999999,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 692.1855,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1608.496,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.753,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 68.366,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 94.35300000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "90ee6969e060cb3f1bcc852d006e4efcf38c0449",
          "message": "chore: Upgrade to `0.19.0` (#3286)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T09:44:08-04:00",
          "tree_id": "0835074ee03c85d566877553a3a60b056b2c656b",
          "url": "https://github.com/paradedb/paradedb/commit/90ee6969e060cb3f1bcc852d006e4efcf38c0449"
        },
        "date": 1760020011997,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1201.27,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 650.098,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1496.496,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 716.2750000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 687.5115000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1613.108,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.6485,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.0725,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.8675,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "10de316db45e848a0bc1c1ae86f820de701030f1",
          "message": "chore: show readable error if serving reads from hot standby (#3267)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nShow a better error message if trying to serve reads on a hot standby\nfrom ParadeDB Community, which does not support WALs -- only Enterprise\ndoes.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T11:48:50-04:00",
          "tree_id": "da13ba2361322103116c1d498ec45f15ad86908c",
          "url": "https://github.com/paradedb/paradedb/commit/10de316db45e848a0bc1c1ae86f820de701030f1"
        },
        "date": 1760027690187,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1188.2795,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 664.543,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1483.17,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 731.1465000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 701.3415,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1613.178,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.968,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.173,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.223,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "3d66fbeefb193965a576f504a7f578024eeee5e7",
          "message": "feat: standardize EXPLAIN output formatting with ExplainFormat trait (#3268)\n\n## Summary\n\nThis PR introduces the `ExplainFormat` trait to standardize how objects\nare formatted for PostgreSQL `EXPLAIN` output across the codebase. This\neliminates duplicate implementations and ensures consistent,\ndeterministic output for regression tests.\n\n## Changes\n\n- `ExplainFormat` trait: common interface for formatting objects for\nEXPLAIN output\n- `cleanup_json_for_explain()`: Removes non-deterministic data (OIDs,\npointers) from JSON for consistent test output\n- `format_for_explain()`: Convenience function for serializable objects\n\n### Refactoring\n- Removed duplicated formatting functions:\n- Deprecated `SearchQueryInput::canonical_query_string()` (now uses\n`ExplainFormat`)\n- Deprecated standalone `cleanup_variabilities_from_tantivy_query()`\n(now uses `cleanup_json_for_explain()`)\n\n- Implemented `ExplainFormat` for:\n  - `SearchQueryInput` - query formatting\n  - `AggregateType` - aggregate formatting with FILTER clauses\n\n- Updated `Explainer`:\n- New `add_explainable()` method accepts any `ExplainFormat` implementor\n  - Simplified `add_query()` to use the new trait",
          "timestamp": "2025-10-09T09:00:38-07:00",
          "tree_id": "3376b11c73acc94fb95af94e2ff1048b3579e3da",
          "url": "https://github.com/paradedb/paradedb/commit/3d66fbeefb193965a576f504a7f578024eeee5e7"
        },
        "date": 1760028062954,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1202.6779999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 655.5545,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1511.4144999999999,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 716.9005,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 693.0725,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1617.6215,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.164,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.664,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.583,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "76c4decb23962ae948fff00b4b64958c5b631bbf",
          "message": "feat: support logical replication in community (#3264)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nUntil now, logical replication has been an enterprise-only feature. We\nhave made the decision to move it to community, which allows anyone to\nrun ParadeDB as a logical replica of a primary Postgres.\n\nNote: Logical replication is only supported for Postgres 17 and newer.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T11:11:49-04:00",
          "tree_id": "85d200bc61f1a460e3f621edae3ca55e6a027145",
          "url": "https://github.com/paradedb/paradedb/commit/76c4decb23962ae948fff00b4b64958c5b631bbf"
        },
        "date": 1760031370845,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1180.8915000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 661.4155000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1475.1545,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 729.9304999999999,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 695.625,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1614.4814999999999,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.591,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.043,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.6785,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "60b5f95971b616bb8c197b99797555d944c3628b",
          "message": "feat: tokenizers-as-types (#3290)\n\n## What\n\nThis PR introduces first-class tokenizer types, inline tokenization via\ncasts, stricter validation and planning around tokenizer usage, and new\nbuilder functions/operators to improve query ergonomics and execution\nplanning.\n\n### New Types\n- Tokenizer SQL types with CREATE TYPE definitions and IO functions are\ngenerated via a new macros::generate_tokenizer_sql proc-macro.\n- Tokenizer types are flagged as category “t” (tokenizer), with LIKE\ntext, collatable, and can be marked PREFERRED.\n- JSON/JSONB assignment casts to tokenizer types.\n\n### Typmods\n- Generic typmod support for tokenizer types when custom typmod is not\nprovided (TYPMOD_IN/OUT wired to generic handlers).\n- Dedicated parsing for specialized typmods (e.g., fuzzy parameters).\n- Ability to attach an alias via typmod for indexed expressions; alias\nmust be present when a tokenizer is used in an indexed expression.\n\n### Casting to TEXT[] for inline tokenization\n- Each tokenizer type supports cast to TEXT[] for immediate\ntokenization, enabling:\n  - 'text'::pdb.simple::text[]\n  - 'text'::pdb.ngram(3,5)::text[]\n  - 'text'::pdb.lindera(japanese)::text[]\n  - and similar casts for all supported tokenizer types\n- JSON/JSONB values can be assignment-cast to tokenizer types and then\nto TEXT[].\n\n### CREATE INDEX examples\n- Tokenizer casts in index definitions now require an alias typmod:\n  - (t::pdb.simple('alias=simple'))\n  - (t::pdb.ngram(3,5, 'alias=ngram'))\n  - (t::pdb.lindera(japanese, 'alias=lindera_japanese'))\n- Both direct attribute indexing and expression indexing are supported;\nwhen indexing expressions using tokenizer types, an alias is required to\nbind the field name for planning.\n- Planner can identify fields from:\n  - direct Vars,\n  - tokenizer expressions with alias,\n  - JSON path references (e.g., json_field->'a'->>'b' becomes “a.b”),\n- unaliased tokenizer expressions only when safely resolvable to a\nsingle Var and generic typmod has no alias specified.\n\n## Why\n- Provide clear, typed surface for tokenizers, improving safety,\ndiscoverability, and SQL UX.\n- Enable inline tokenization for testing, debugging, and composed SQL\nexpressions.\n- Ensure indexed expressions using tokenizers are unambiguous at plan\ntime, avoiding runtime surprises.\n- Broaden operator LHS type compatibility (text, varchar, arrays,\njson/jsonb, tokenizer types) with explicit validation and better error\nmessages.\n\n## How\n- New macros:\n  - builder_fn refactored into its own module.\n- generate_tokenizer_sql macro emits CREATE TYPE, casts, and optional\ntypmod wiring.\n- Operator planning:\n  - Expanded resolution of field names from complex nodes.\n- Enforced alias requirement for tokenizer expressions in indexes unless\nsafely resolvable.\n- Validation that LHS types are text-compatible or tokenizer-compatible.\n- Request-simplify code paths updated to carry LHS through const/exec\nrewrites and to wrap RHS with index info.\n- Builder functions:\n- Added array-based query builders (match_conjunction_array,\nmatch_disjunction_array, phrase_array).\n- Operators now have overloads to accept TEXT[], pdb.query, boost,\nfuzzy, etc., with shared support function.\n\n## Tests\n- Inline tokenization: casts of literals through various tokenizer types\nto TEXT[].\n- CREATE INDEX with tokenizer types: examples covering all tokenizers\nwith alias typmods and EXPLAIN plans using those aliases.\n- Enforcement: indexing a tokenizer expression without alias raises a\nclear error.\n- RHS tokenizer casts: @@@ does not accept tokenizer-cast RHS; &&&, |||,\n###, === do accept and are validated.\n\n---------\n\nCo-authored-by: Ming Ying <ming.ying.nyc@gmail.com>",
          "timestamp": "2025-10-09T14:03:50-04:00",
          "tree_id": "142dd25a67167915628ac8b52e0056b31bb70b64",
          "url": "https://github.com/paradedb/paradedb/commit/60b5f95971b616bb8c197b99797555d944c3628b"
        },
        "date": 1760035517253,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1207.6175,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 657.7760000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1505.387,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 727.695,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 695.5074999999999,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1605.2150000000001,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.997,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.804,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 91.775,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8a3ca3d2629465260e3d38de91714d4fa3afb0a8",
          "message": "feat: support the `stopwords_language` typmod property (#3299)\n\n## What\n\nThis adds back in support for the language-specific built-in list of\nstopwords, using `stopwords_language=xxx`.\n\n## Why\n\nStopwords are pretty useful!\n\n## How\n\n## Tests\n\nA regression test has been added.",
          "timestamp": "2025-10-09T15:29:20-04:00",
          "tree_id": "9ed34db181fb1282dd50b9ddde0b93999a79f19a",
          "url": "https://github.com/paradedb/paradedb/commit/8a3ca3d2629465260e3d38de91714d4fa3afb0a8"
        },
        "date": 1760040511779,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1184.5030000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 653.626,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1475.6105,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 709.1955,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 686.371,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1584.4119999999998,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.2765,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 67.6045,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 92.0225,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "0a691cd226f9b335af5591dd84e695ec4da92df3",
          "message": "chore: Un-revert remove deprecated tokenizers (#3306)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nBrings back https://github.com/paradedb/paradedb/pull/3279 which was\nreverted\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T16:18:45-04:00",
          "tree_id": "9f4e6764d3c00556eadd693db305527cbf8a2342",
          "url": "https://github.com/paradedb/paradedb/commit/0a691cd226f9b335af5591dd84e695ec4da92df3"
        },
        "date": 1760043543974,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1182.0655000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 658.679,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1468.9789999999998,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 727.989,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 698.6289999999999,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1578.8795,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 22.927500000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.1365,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 87.94999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "42671fe0df806d6033d837f7b058b0b6f81b187d",
          "message": "chore: Remove default `255` byte \"remove long\" filter setting (#3288)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nA 255 byte \"remove long\" setting was applied to all text fields.\n\nThis was wrong for text fast fields -- any text longer than 255 bytes\nwas being silently dropped. And it's confusing to document for non fast\ntext fields, we should be preserving the tokens and let the user opt\ninto it rather than doing it by default.\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-09T16:49:35-04:00",
          "tree_id": "c1ccc15293d114edb6d9d98bc04d99512d1a539d",
          "url": "https://github.com/paradedb/paradedb/commit/42671fe0df806d6033d837f7b058b0b6f81b187d"
        },
        "date": 1760045340327,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1191.4160000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 666.037,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1499.2425,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 733.635,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 703.377,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1600.7065,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.849,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.617,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.5405,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "98baf4ce05a4907c305bcbb28438d5fcc6025f09",
          "message": "chore: Remove dead code in `tokenizers/src/manager.rs` (#3310)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-10T09:28:36-04:00",
          "tree_id": "ae706715e3523d0f2bca3de4c17c19661812c2c8",
          "url": "https://github.com/paradedb/paradedb/commit/98baf4ce05a4907c305bcbb28438d5fcc6025f09"
        },
        "date": 1760105273088,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1181.4455,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 646.3924999999999,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1492.4685,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 706.8175,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 683.5035,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1618.136,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.357,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.407,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.26050000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "d83f2888d60760ce3ab8c71ac24749d018604ce7",
          "message": "feat: Introduce `TokenLengthFilter` (#3309)\n\n# Ticket(s) Closed\n\n- Closes #3280 \n\n## What\n\nIntroduces a `TokenLengthFilter`, which is like Tantivy's\n`RemoveLongFilter` but has a min and max threshold. This enables a new\nfilter, `remove_short`, which removes tokens below a certain byte\nthreshold.\n\n## Why\n\nUser request\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-10T09:38:04-04:00",
          "tree_id": "51697ab88c096b3ab284e210fec8ec1eb9b425c4",
          "url": "https://github.com/paradedb/paradedb/commit/d83f2888d60760ce3ab8c71ac24749d018604ce7"
        },
        "date": 1760105849335,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1207.6885,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 650.2805000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1506.031,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 709.348,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 687.925,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1625.533,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.508,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.2325,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.372,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "84ace13610bc16ef88495a07915708cbe371e4a0",
          "message": "feat: add new `##>` operator for \"in order\" proximity (#3314)\n\n# Ticket(s) Closed\n\n- Closes #3305\n\n## What\n\nSimilar to the existing `##` operator, but `##>` requires the rhs be\nthat many tokens after the lhs.\n\nExample:\n\n```sql\n... WHERE text @@@ ('z' ##>24##> 'a');\n```\n\n## Why\n\n## How\n\n## Tests\n\nUpdated the `proximity.sql` test with a small test.",
          "timestamp": "2025-10-10T10:25:54-04:00",
          "tree_id": "576dd1e98eb7884b79985358e642deb5df8beda0",
          "url": "https://github.com/paradedb/paradedb/commit/84ace13610bc16ef88495a07915708cbe371e4a0"
        },
        "date": 1760108748262,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1185.5085,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 649.2695,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1475.086,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 716.855,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 689.2075,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1600.0189999999998,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 22.657,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.05799999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.917,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
          "id": "051f1c91e005cf94328dff488aaa4ba9b9b89703",
          "message": "feat: Introduce `AlphaNumOnly` filter (#3315)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nA new filter that removes tokens containing non ASCII letters and\ndigits.\n\n## Why\n\n## How\n\n## Tests\n\nAdded regression test",
          "timestamp": "2025-10-10T10:48:38-04:00",
          "tree_id": "95691fe3913334da5c80c710f3451542e0fa7327",
          "url": "https://github.com/paradedb/paradedb/commit/051f1c91e005cf94328dff488aaa4ba9b9b89703"
        },
        "date": 1760111062005,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1197.3004999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 668.2435,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1499.885,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 727.0715,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 706.1890000000001,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1614.7365,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.331,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.394,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.5735,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ec9ff340a70f67a810bbd68f1032786a6a70171f",
          "message": "fix: allow the RHS of `@@@` to support `pdb.query` at runtime (#3308)\n\n# Ticket(s) Closed\n\n- Closes #3300\n\n## What\n\nThis allows the rhs of `@@@` to support a `pdb.query` at runtime\n\n## Why\n\n## How\n\n## Tests\n\nYes",
          "timestamp": "2025-10-10T10:37:24-04:00",
          "tree_id": "5e8a7b39c086133de1e3b584061cf342da72a619",
          "url": "https://github.com/paradedb/paradedb/commit/ec9ff340a70f67a810bbd68f1032786a6a70171f"
        },
        "date": 1760111063696,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1200.28,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 672.56,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1487.467,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 732.0550000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 707.7505,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1617.117,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.851,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.494,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.0015,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8269d4ed9b0f6101d4627bc72eb008026312fdac",
          "message": "chore: relocate `fuzzy`, `slop`, `boost` types to the `pdb` schema (#3316)\n\n# Ticket(s) Closed\n\n- Closes #3281\n\n## What\n\nThis relocates the `fuzzy`, `slop`, and `boost` types from `pg_catalog`\nto our `pdb` schema.\n\n🚨This will be a breaking change for any existing queries that cast the\nrhs of one of our operators to one of these types. They'll need to be\nrewritten as `::pdb.fuzzy`.\n\n## Why\n\nPutting them in `pg_catalog` was a mistake and inconsistent with our\nother SQL UX work.\n\n## How\n\n## Tests\n\nExisting tests (and docs) have been updated.",
          "timestamp": "2025-10-10T10:51:05-04:00",
          "tree_id": "4a988e0eff7dab9eec493a7ab5f16feb53bb4929",
          "url": "https://github.com/paradedb/paradedb/commit/8269d4ed9b0f6101d4627bc72eb008026312fdac"
        },
        "date": 1760111075558,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1190.1925,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 656.6175,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1498.4569999999999,
            "unit": "median ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 725.4615,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 694.9085,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1621.999,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.2885,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.2525,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.72749999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      }
    ]
  }
}