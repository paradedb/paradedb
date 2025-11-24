window.BENCHMARK_DATA = {
  "lastUpdate": 1763944020508,
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763943001729,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7624.5985,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7633.287,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1931.6405,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 431.247,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 105.452,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 888.0474999999999,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 0.418,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1892.4155,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 416.057,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.857,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 840.3735,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 0.384,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2917.5615,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 430.07849999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 105.9365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 866.765,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 0.3755,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2896.8410000000003,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 415.4765,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 91.7535,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 835.6095,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 0.38,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14483.8975,
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
            "value": 432.074,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 105.67699999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 506.227,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 0.3655,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 250.558,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 188.51749999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 100.007,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 208.587,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 0.376,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 889.2950000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 451.14300000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 153.0555,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 519.8225,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 0.365,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.5495,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.5455000000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 7.88,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.7545,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 8.955,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 5510.0455,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 0.3895,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 0.395,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 55227.162,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 90.6985,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 72.4325,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.0145,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 109.709,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 69.544,
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
        "date": 1763943069182,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7465.7595,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7464.431500000001,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1843.935,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 430.6545,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 107.715,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 430.57050000000004,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 0.3795,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1811.1154999999999,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 411.9145,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 92.66550000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 411.696,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 0.3885,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 2919.572,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 431.1515,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 108.637,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 2918.3720000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 0.386,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 2889.143,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 413.81500000000005,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 94.11449999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 2885.8025,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 0.379,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14580.975999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1795.2305000000001,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 430.16549999999995,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 106.727,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 426.7075,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 0.3655,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 212.2895,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 184.91500000000002,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 98.271,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 187.058,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 0.3695,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 737.7085,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 448.51599999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 154.10199999999998,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 453.8,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 0.3855,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 6.811,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.7,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.424,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 6.9535,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 0.36,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 6843.888999999999,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 0.409,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 0.419,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 55554.547,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 95.033,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 73.318,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 69.503,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 0.369,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 71.1785,
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
        "date": 1763943162076,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 7476.9400000000005,
            "unit": "median ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 7466.6965,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1836.9325,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 405.84299999999996,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 93.69800000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 1837.685,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 4",
            "value": 0.3315,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1762.342,
            "unit": "median ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 392.16700000000003,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 74.377,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 1761.0915,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 4",
            "value": 0.329,
            "unit": "median ms",
            "extra": "SELECT severity, pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity"
          },
          {
            "name": "bucket-string-filter",
            "value": 6876.046,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 410.019,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 105.40700000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 6932.6805,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-filter - alternative 4",
            "value": 0.323,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 6948.9220000000005,
            "unit": "median ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 393.381,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 85.0875,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 6958.9655,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 4",
            "value": 0.328,
            "unit": "median ms",
            "extra": "SELECT country, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country"
          },
          {
            "name": "cardinality",
            "value": 14448.838,
            "unit": "median ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1702.0545,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 405.593,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 93.6485,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 1702.2275,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity)"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 0.3255,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"terms\": {\"field\": \"severity\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "count-filter",
            "value": 219.6715,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 185.1365,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 99.051,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 187.9005,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 4",
            "value": 0.315,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-nofilter",
            "value": 734.2585,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 447.3,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 153.4185,
            "unit": "median ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"ctid\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 450.53700000000003,
            "unit": "median ms",
            "extra": "SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 4",
            "value": 0.3265,
            "unit": "median ms",
            "extra": "SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}'::jsonb) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "filtered-highcard",
            "value": 10.894,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 6.8635,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 6.8525,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 7.491,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "highlighting",
            "value": 0.2655,
            "unit": "median ms",
            "extra": "SELECT id, pdb.snippet(message), pdb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 0.3325,
            "unit": "median ms",
            "extra": "SELECT COUNT(*) FROM benchmark_logs WHERE (country @@@ pdb.regex('united.*') AND country ILIKE '% States')"
          },
          {
            "name": "top_n-agg-avg",
            "value": 0.327,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"avg\": {\"field\": \"severity\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-bucket-string",
            "value": 0.328,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, pdb.agg('{\"terms\": {\"field\": \"country\"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-agg-count",
            "value": 55483.883499999996,
            "unit": "median ms",
            "extra": "SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10"
          },
          {
            "name": "top_n-compound",
            "value": 99.531,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 118.199,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 98.955,
            "unit": "median ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 0.273,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 105.3195,
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
        "date": 1763943395831,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1209.5610000000001,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 645.5805,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1493.7759999999998,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 704.067,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 676.579,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1621.4975,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 24.6845,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 66.8335,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 90.7445,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
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
        "date": 1763943473968,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1173.5505,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 642.923,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1447.607,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 702.3905,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 676.222,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1597.3319999999999,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.534,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.803,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 89.15350000000001,
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
          "id": "f4cc34211b69c210cad6133141ec34b114d4e528",
          "message": "docs: fix more-like-this (MLT) JSON syntax example (#3335)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-10-14T19:24:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/f4cc34211b69c210cad6133141ec34b114d4e528"
        },
        "date": 1763944018086,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 1192.334,
            "unit": "median ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 656.976,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1474.152,
            "unit": "median ms",
            "extra": "SELECT *, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 726.9145000000001,
            "unit": "median ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 696.0675,
            "unit": "median ms",
            "extra": "SELECT documents.id, files.id, pages.id, pdb.score(documents.id) + pdb.score(files.id) + pdb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 1604.321,
            "unit": "median ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 23.733,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-max') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 65.78,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-median') ORDER BY id LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 88.70349999999999,
            "unit": "median ms",
            "extra": "SELECT * FROM pages WHERE id @@@ paradedb.all() AND id >= (SELECT value FROM docs_schema_metadata WHERE name = 'pages-row-id-min') ORDER BY id LIMIT 100"
          }
        ]
      }
    ]
  }
}