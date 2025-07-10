window.BENCHMARK_DATA = {
  "lastUpdate": 1752154334345,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'join' Query Performance": [
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
          "id": "75457722987cf2652425f92585955a3252059d12",
          "message": "ci: cleanup benchmark running (#2809)\n\niterating on benchmark CI jobs",
          "timestamp": "2025-07-10T08:57:20-04:00",
          "tree_id": "2b0db192445c1cd42424c58a6b8cbde94fbc8407",
          "url": "https://github.com/paradedb/paradedb/commit/75457722987cf2652425f92585955a3252059d12"
        },
        "date": 1752154294038,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 915.6446,
            "unit": "avg ms",
            "extra": "SELECT documents.id, files.id, pages.id FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 20885.3197,
            "unit": "avg ms",
            "extra": "SELECT *, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 1471.6302000000003,
            "unit": "avg ms",
            "extra": "WITH topn AS ( SELECT documents.id AS doc_id, files.id AS file_id, pages.id AS page_id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000 ) SELECT d.*, f.*, p.*, topn.score FROM topn JOIN documents d ON topn.doc_id = d.id JOIN files f ON topn.file_id = f.id JOIN pages p ON topn.page_id = p.id WHERE topn.doc_id = d.id AND topn.file_id = f.id AND topn.page_id = p.id ORDER BY topn.score DESC"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 915.2782000000001,
            "unit": "avg ms",
            "extra": "SELECT documents.id, files.id, pages.id, paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id) AS score FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach' ORDER BY score DESC LIMIT 1000"
          },
          {
            "name": "line_items-distinct",
            "value": 2236.3195,
            "unit": "avg ms",
            "extra": "SELECT DISTINCT pages.* FROM pages JOIN files ON pages.\"fileId\" = files.id WHERE pages.content @@@ 'Single Number Reach'  AND files.\"sizeInBytes\" < 5 AND files.id @@@ paradedb.all() ORDER by pages.\"createdAt\" DESC LIMIT 10"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 735.1746999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM documents JOIN files ON documents.id = files.\"documentId\" JOIN pages ON pages.\"fileId\" = files.id WHERE documents.parents @@@ 'SFR' AND files.title @@@ 'collab12' AND pages.\"content\" @@@ 'Single Number Reach'"
          }
        ]
      }
    ],
    "pg_search 'single' Query Performance": [
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
          "id": "75457722987cf2652425f92585955a3252059d12",
          "message": "ci: cleanup benchmark running (#2809)\n\niterating on benchmark CI jobs",
          "timestamp": "2025-07-10T08:57:20-04:00",
          "tree_id": "2b0db192445c1cd42424c58a6b8cbde94fbc8407",
          "url": "https://github.com/paradedb/paradedb/commit/75457722987cf2652425f92585955a3252059d12"
        },
        "date": 1752154330869,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "bucket-expr-filter",
            "value": 72361.394,
            "unit": "avg ms",
            "extra": "SELECT date_trunc('year', timestamp) as year, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-string-filter",
            "value": 4223.2113,
            "unit": "avg ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 372.6522,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 68.8629,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "highlighting",
            "value": 9.177100000000001,
            "unit": "avg ms",
            "extra": "SELECT id, paradedb.snippet(message), paradedb.snippet(country) FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 13.946200000000001,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND severity < 3 LIMIT 10"
          },
          {
            "name": "filtered_json-range",
            "value": 13.575899999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND id @@@ paradedb.range('metadata.value', int4range(10, NULL, '[)')) AND message @@@ 'research' LIMIT 10"
          },
          {
            "name": "top_n-numeric-lowcard",
            "value": 129.5022,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10"
          },
          {
            "name": "cardinality",
            "value": 11949.157,
            "unit": "avg ms",
            "extra": "SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 1674.8753999999997,
            "unit": "avg ms",
            "extra": "SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 388.43670000000003,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 104.329,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 1684.1039,
            "unit": "avg ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 400.00720000000007,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 75.5432,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1654.4741,
            "unit": "avg ms",
            "extra": "SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY severity ORDER BY severity"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 395.3158,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 96.7756,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"severity\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "top_n-numeric-highcard",
            "value": 129.7367,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10"
          },
          {
            "name": "count-filter",
            "value": 255.3385,
            "unit": "avg ms",
            "extra": "SELECT COUNT(id) FROM benchmark_logs WHERE message @@@ 'team'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 137.0709,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 29.327800000000003,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'team'), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "count-nofilter",
            "value": 916.5644,
            "unit": "avg ms",
            "extra": "SELECT COUNT(id) FROM benchmark_logs WHERE id @@@ paradedb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 392.37140000000005,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 81.551,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"count\": { \"value_count\": { \"field\": \"id\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "filtered-highcard",
            "value": 17.157,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' AND timestamp >= '2020-10-02T15:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered_json",
            "value": 11.4916,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE id @@@ paradedb.term('metadata.label', 'critical system alert') AND message @@@ 'research' AND severity < 3 LIMIT 10"
          },
          {
            "name": "top_n-string",
            "value": 110.89439999999999,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10"
          },
          {
            "name": "top_n-score",
            "value": 142.70059999999998,
            "unit": "avg ms",
            "extra": "SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 4020.6684999999998,
            "unit": "avg ms",
            "extra": "SELECT country, COUNT(*) FROM benchmark_logs WHERE id @@@ paradedb.all() GROUP BY country ORDER BY country"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 365.0372,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>true)"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 61.539699999999996,
            "unit": "avg ms",
            "extra": "SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.all(), agg=>'{\"buckets\": { \"terms\": { \"field\": \"country\" }}}', solve_mvcc=>false)"
          },
          {
            "name": "top_n-compound",
            "value": 104.4947,
            "unit": "avg ms",
            "extra": "SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10"
          }
        ]
      }
    ]
  }
}