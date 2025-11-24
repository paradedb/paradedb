window.BENCHMARK_DATA = {
  "lastUpdate": 1763942456863,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'logs' Query Performance": [
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "stuhood@gmail.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "distinct": true,
          "id": "099c58ef752c337320d3e06a685cf80ba86e533a",
          "message": "chore: Prepare `0.19.9`. (#3604)\n\nPrepare `0.19.9`.",
          "timestamp": "2025-11-23T15:34:49-08:00",
          "tree_id": "108a1316f3541a472d93c2c75d3a050eb585ba61",
          "url": "https://github.com/paradedb/paradedb/commit/099c58ef752c337320d3e06a685cf80ba86e533a"
        },
        "date": 1763942387808,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7316.378,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7304.3135,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1871.6315,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 423.53200000000004,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.47149999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 454.11,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 877.1105,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1837.913,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 408.353,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 90.778,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 437.385,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 868.294,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2897.7754999999997,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 425.4825,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.548,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 426.5025,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 833.107,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2865.29,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 407.353,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 88.8175,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 410.1865,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 812.6990000000001,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14544.909,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1834.0415,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 423.068,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 110.7835,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 453.698,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 454.3625,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 207.71800000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 146.5845,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 83.87899999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 148.7595,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 148.44549999999998,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 680.7484999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 426.63149999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 150.26749999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 427.68899999999996,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 427.499,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.435499999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.307,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 10.7645,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.468999999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.6805,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5258.184,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 584.42,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 572.8834999999999,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 590.4585,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 88.172,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 69.9865,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.06700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 112.38050000000001,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 69.78800000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "21ae4f92dc730f588c957d8cb5c893b916d95409",
          "message": "feat: supported window aggregate pushdown for all search operators (#3582)\n\n# Ticket(s) Closed\n\n- Closes #3566\n\n## What\n\nWindow aggregate queries with `|||`, `&&&`, `===`, and `###` operators\nnow properly push down to TopN scans, just like queries using the `@@@`\noperator.\n\n## Why\n\nPreviously, queries like `SELECT *, COUNT(*) OVER () FROM table WHERE\nfield ||| 'term' ORDER BY rating LIMIT 10` would fall back to\nPostgreSQL's standard WindowAgg execution path instead of using\noptimized TopN scan. This happened because the window function\nreplacement logic only checked for the `@@@` operator when deciding\nwhether to enable pushdown.\n\n## How\n\n- Added helper functions to get OIDs for all ParadeDB search operators:\n`match_disjunction_text_opoid()` for `|||`,\n`match_conjunction_text_opoid()` for `&&&`, `term_text_opoid()` for\n`===`, and `phrase_text_opoid()` for `###`\n- Updated `query_has_search_operator()` to check for all search\noperators, not just `@@@`\n\n## Tests\n\nAdded tests in `topn-agg-facet.sql` (Tests 1a-1d) verifying that window\naggregate queries with `|||`, `&&&`, `===`, and `###` operators properly\nuse TopNScanExecState execution.",
          "timestamp": "2025-11-21T21:49:27Z",
          "url": "https://github.com/paradedb/paradedb/commit/21ae4f92dc730f588c957d8cb5c893b916d95409"
        },
        "date": 1763942454480,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7364.1465,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7320.357,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1818.6835,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 415.56899999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 108.754,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 446.45950000000005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 891.2045,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1793.9499999999998,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 399.275,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.471,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 428.2745,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 857.216,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2905.6835,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 417.63599999999997,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 107.553,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 417.57500000000005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 811.1785,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2847.755,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 398.34799999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 90.406,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 401.8925,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 797.4404999999999,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14499.2725,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1795.4675,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 414.621,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 109.2405,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 454.6195,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 452.64750000000004,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 208.71550000000002,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 144.6575,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 83.4915,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 146.3785,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 145.99149999999997,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 688.5575,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 417.9225,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 148.82150000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 419.722,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 418.8295,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.3825,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.1995000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 10.7385,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.4465,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.7875,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5243.2425,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 584.394,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 573.046,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 592.16,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 87.2865,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 69.351,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.2895,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 106.9385,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 70.039,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          }
        ]
      }
    ]
  }
}