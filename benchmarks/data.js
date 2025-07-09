window.BENCHMARK_DATA = {
  "lastUpdate": 1752038825793,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'join' Query Performance": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "35abeff494503653aeca1073004f5e0cfd89e115",
          "message": "Backfill for 0.15.26",
          "timestamp": "2025-07-09T04:35:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/35abeff494503653aeca1073004f5e0cfd89e115"
        },
        "date": 1752038653042,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-scores-small",
            "value": 923.1689,
            "unit": "avg ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 4347.1485999999995,
            "unit": "avg ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 1002.2123000000001,
            "unit": "avg ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "line_items-distinct",
            "value": 2660.0068999999994,
            "unit": "avg ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 1371.9487000000001,
            "unit": "avg ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 1135.3713,
            "unit": "avg ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          }
        ]
      }
    ],
    "pg_search 'single' Query Performance": [
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "committer": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@gmail.com"
          },
          "id": "35abeff494503653aeca1073004f5e0cfd89e115",
          "message": "Backfill for 0.15.26",
          "timestamp": "2025-07-09T04:35:02Z",
          "url": "https://github.com/paradedb/paradedb/commit/35abeff494503653aeca1073004f5e0cfd89e115"
        },
        "date": 1752038823068,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "filtered-lowcard",
            "value": 11.891499999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 4186.686599999999,
            "unit": "avg ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 525.0065999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 107.92590000000003,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "top_n-compound",
            "value": 80.76440000000001,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 8.6872,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 89.8572,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 8.105499999999997,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "cardinality",
            "value": 17711.600199999997,
            "unit": "avg ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1956.208,
            "unit": "avg ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 656.7335999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 113.73440000000001,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 2007.2747,
            "unit": "avg ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 722.6624999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 119.99830000000001,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 99.8132,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "count-nofilter",
            "value": 1419.1227999999999,
            "unit": "avg ms",
            "extra": "SELECT COUNT(id) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 719.9252000000001,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 144.73139999999998,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "highlighting",
            "value": 9.2056,
            "unit": "avg ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 13609.2948,
            "unit": "avg ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1558.2573000000002,
            "unit": "avg ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 531.1084,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 74.30449999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-filter",
            "value": 337.3357,
            "unit": "avg ms",
            "extra": "SELECT COUNT(id) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 226.3951,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 52.580499999999994,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-string-filter",
            "value": 4347.0855,
            "unit": "avg ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 569.7501,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 106.2721,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "filtered-highcard",
            "value": 10.902099999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 92.3164,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 92.1828,
            "unit": "avg ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          }
        ]
      }
    ]
  }
}