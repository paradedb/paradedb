window.BENCHMARK_DATA = {
  "lastUpdate": 1778524325387,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'stackoverflow' (100k rows)": [
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
          "id": "5ce8f7cabc2743985d08edbeaffb38b3c62f6826",
          "message": "chore: Prepare `0.21.16`. (#4436)\n\n# Description\nBackport of #4434 to `0.21.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-03-20T02:44:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/5ce8f7cabc2743985d08edbeaffb38b3c62f6826"
        },
        "date": 1778523312251,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.376608800000003,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.174; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 24.2963394,
            "range": "±0.081 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=119.864; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 64.1218992,
            "range": "±0.135 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=118.739; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 64.0975588,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=119.248; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 25.7306515,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=129.027; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 25.723391100000004,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=123.943; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 43.3694251,
            "range": "±0.172 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=143.665; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 43.5822148,
            "range": "±0.123 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.225; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 54.46833499999999,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=157.466; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 54.360175600000005,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=161.903; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 19.834977799999997,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.918; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 531.6485855999999,
            "range": "±1.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=569.886; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 5.4976712,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.110; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 5.5243591,
            "range": "±0.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.273; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 5.948556300000001,
            "range": "±0.029 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.535; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 5.4616603,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.170; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 5.6014222,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.255; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 9.905451600000001,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.174; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 7.720426999999999,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.899; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 7.983813299999999,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.302; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 82.9274192,
            "range": "±0.153 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=198.439; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 46.5566674,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=138.096; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 49.3791885,
            "range": "±0.029 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=149.144; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 196.2972484,
            "range": "±0.112 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=311.249; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 122.36737149999999,
            "range": "±0.192 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=220.167; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 144.18253169999997,
            "range": "±0.438 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=240.603; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 5.9492965,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.733; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 5.8792579,
            "range": "±0.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.976; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 5.4468584,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.273; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 5.4154438,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.055; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 0.0044615,
            "range": "±0.000 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=0.007; query=SELECT 1 + 1"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 6.6449014,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.207; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 11.1256562,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.645; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 5.725144200000001,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.590; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 5.7086977999999995,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.991; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.2560968,
            "range": "±1.461 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.674; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 8.200183200000001,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.690; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.758821900000001,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.717; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.7641813,
            "range": "±0.220 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.786; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 39.35698319999999,
            "range": "±0.171 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=127.762; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 39.47147520000001,
            "range": "±0.207 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=118.570; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 5.8084743,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.686; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 5.6455169,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.033; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 14.8648179,
            "range": "±0.178 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.619; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 14.9407649,
            "range": "±0.294 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.735; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 10.062275100000003,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.838; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 10.069917199999999,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.066; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 10.3676204,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=141.416; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 10.2540846,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=119.962; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.1156423,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=210.701; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 28.6628332,
            "range": "±0.181 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=266.722; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 28.0944682,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=237.430; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 25.9558421,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=198.948; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 25.910476699999997,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=200.501; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 9.4710653,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.710; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.2317221,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=27.075; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.316182100000001,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=29.151; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.1977233,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=29.683; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 32.2510041,
            "range": "±0.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=147.405; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 8.454695,
            "range": "±0.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.368; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 10.344949500000002,
            "range": "±0.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.546; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 10.4308485,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.544; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 10.418844,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.418; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 9.531676200000001,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.799; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 10.3303634,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.754; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.4378759,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.770; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.2769954,
            "range": "±0.258 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.815; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.0099042,
            "range": "±0.259 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.024; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 5.7065683,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.894; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 5.7063121,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.489; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 5.609170799999999,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.664; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 6.8988955,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.234; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.5305987,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=87.989; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 6.840058400000001,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.398; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.438364000000001,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.428; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 6.7517852000000005,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.053; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.2469342000000005,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.950; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 6.8277608999999995,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.835; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.2183526999999996,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.222; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 6.7921702999999995,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.812; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.1081554999999996,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.753; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 6.846617,
            "range": "±0.099 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=61.607; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.9843297,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.406; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 9.9393101,
            "range": "±0.176 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=90.779; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.528341000000001,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=147.072; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 9.9457123,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.732; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 9.3176803,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=133.556; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 5.7679738,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.442; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
          "id": "0b5f5714895d3226ae9025f04f5867cf6e63215d",
          "message": "chore: Upgrade to 0.22.6 (#4694)\n\n## Summary\n- Bump version from 0.22.5 to 0.22.6\n- Add changelog entry for 0.22.6\n- Update version references in docs\n\n## Changes since 0.22.5\n- feat: Support expressions in JoinScan DISTINCT target lists (#4682)\n- fix: JoinScan `DISTINCT` planning for deferred keys (#4670)\n- fix: JoinScan pushdown with outer-only `ORDER BY` pathkeys (#4680)\n- fix: `pdb.score()` with `SELECT` subquery in `WHERE` clause (#4653)\n- fix: Handle `IN (SELECT ...) OR IS NULL` via LeftMark JoinScan (#4651)\n- fix: Score filter for joins and cases without other quals (#4650)\n- fix: Handle pruned columns in nested semi/anti join keys (#4668)\n- fix: Handle aliased indexed expressions in search resolution and top-k\norder by (#4671)\n- fix: Lower x86_64 target-cpu from x86-64-v3 to x86-64-v2 (#4673)\n- fix: Prevent DSM buffer overflow in PG18 parallel index scans (#4683)\n- fix: `PlaceHolderVar found where not expected` error (#4689)\n- fix: Restore `pdb.agg(jsonb)` if accidentally removed by an upgrade\nscript (#4688)\n\n## Test plan\n- [ ] CI passes on 0.22.x\n\n---------\n\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-04-08T18:52:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/0b5f5714895d3226ae9025f04f5867cf6e63215d"
        },
        "date": 1778523422383,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.265254700000003,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=151.351; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 24.1259177,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=149.094; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 67.0173904,
            "range": "±0.438 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=173.638; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 67.1162257,
            "range": "±0.296 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.386; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 24.938057399999998,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=151.231; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 25.1011271,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.285; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 53.550187099999995,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=147.248; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 52.99171399999999,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=137.625; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 56.081211800000005,
            "range": "±0.267 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=188.171; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 56.5426563,
            "range": "±0.222 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=193.028; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 40.3061464,
            "range": "±0.206 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=329.663; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 530.0277421,
            "range": "±1.864 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=584.067; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.227124900000001,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=367.529; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.3202723999999995,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=367.596; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.357502599999999,
            "range": "±0.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=376.433; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.7000753,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.662; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.6592592999999995,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.521; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.353674400000001,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.260; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 23.463818100000005,
            "range": "±0.180 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=208.262; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 8.2783056,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.751; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.5797418,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.947; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 6.2162057,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.690; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 90.81701720000001,
            "range": "±1.397 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=218.201; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 74.2919753,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=160.395; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 77.2801012,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=161.704; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 11.4625487,
            "range": "±0.609 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.505; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 221.34389670000002,
            "range": "±0.523 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=366.914; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 256.314428,
            "range": "±0.433 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=331.881; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 279.8087466999999,
            "range": "±0.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=363.334; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 58.9685932,
            "range": "±0.540 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=86.167; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.271230999999999,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=444.456; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.364871399999999,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=443.644; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.722069500000001,
            "range": "±0.131 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.006; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.5696882,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.835; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.218138199999999,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.393; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.6406634,
            "range": "±0.019 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=361.785; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.452699300000003,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.867; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.608820800000004,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.357; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.363093,
            "range": "±0.019 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=647.500; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.921869200000001,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.314; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.8812713,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.380; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.169736899999999,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.688; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 38.3698666,
            "range": "±0.210 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=458.810; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 8.235300899999999,
            "range": "±0.297 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.381; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 8.14063,
            "range": "±0.142 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.618; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.7968243,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.912; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 45.783151999999994,
            "range": "±0.503 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=123.398; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 67.6447333,
            "range": "±0.605 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=394.258; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.1098989999999995,
            "range": "±0.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.385; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.1171766,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.053; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 18.0387547,
            "range": "±0.163 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.287; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 16.786129199999998,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=273.043; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 11.4884693,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=112.199; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 20.3484817,
            "range": "±0.489 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=303.985; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 11.3340591,
            "range": "±0.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.852; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 20.2325184,
            "range": "±0.866 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=329.831; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 29.068766699999998,
            "range": "±0.244 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=234.250; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 29.501892299999998,
            "range": "±0.334 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=311.347; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 30.6944002,
            "range": "±0.296 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=240.333; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 24.9482567,
            "range": "±0.294 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=260.782; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.502932199999996,
            "range": "±0.265 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=236.142; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.9279205999999993,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.118; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0345214,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.367; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 6.0508424,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.935; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.982715,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.379; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 35.079789,
            "range": "±0.225 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=195.117; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.8239664,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=573.885; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.3358891,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.472; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 16.134686199999997,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=238.622; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.3170345,
            "range": "±0.171 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=94.248; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.929775099999999,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=150.066; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 21.6273265,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=236.983; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 7.0530194999999996,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.320; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 7.1300864000000015,
            "range": "±0.238 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.829; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.8823504,
            "range": "±0.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.662; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.3348877,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.340; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.356272199999999,
            "range": "±0.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.845; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.2764665,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.338; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.563559300000001,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.689; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.2387344,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.887; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.6426584,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.978; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.1944973,
            "range": "±0.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.960; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.5938085,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.137; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.0067699,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.386; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.638726499999999,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.222; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.8994961999999997,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.613; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.592845700000001,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.239; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.81093,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.229; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.617128199999999,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.593; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.7438689999999997,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.818; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.6770422,
            "range": "±0.101 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.842; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.252004699999999,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.733; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.662417399999999,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=97.705; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 9.044044199999998,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=104.772; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.4417271,
            "range": "±0.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.842; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
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
          "id": "c07921a78f3d24cbb0251b31a1150a7db600af5a",
          "message": "chore: Prepare 0.23.4. (#4997)\n\n# Description\nBackport of #4994 to `0.23.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-06T00:08:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/c07921a78f3d24cbb0251b31a1150a7db600af5a"
        },
        "date": 1778523688050,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.620700799999998,
            "range": "±0.150 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=149.486; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 32.1674321,
            "range": "±0.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=638.064; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 68.0702302,
            "range": "±0.459 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=160.935; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 68.4599398,
            "range": "±0.830 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=689.052; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 25.739029200000004,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=146.623; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 34.2551787,
            "range": "±0.243 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=592.328; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 54.0453724,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=146.996; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 58.7257202,
            "range": "±0.437 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=662.706; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 56.30131780000001,
            "range": "±0.166 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=187.890; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 57.124901,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=185.578; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 40.2195459,
            "range": "±0.117 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=278.186; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 30.830737399999997,
            "range": "±0.314 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=347.673; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.3172564,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=362.709; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.389272399999999,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=363.520; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.4216687,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=329.831; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.366113299999999,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.525; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.4438946999999995,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.894; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.0178968,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.354; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 23.5162145,
            "range": "±0.159 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=225.931; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 7.8179020999999995,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.716; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.079230599999999,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.304; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 5.878416199999999,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=29.671; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 91.2046078,
            "range": "±0.197 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=231.700; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 71.6191233,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=160.931; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 74.5284192,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=164.518; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.6508332,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.633; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 221.2003467,
            "range": "±0.400 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=360.887; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 259.3660039,
            "range": "±0.360 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=352.892; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 282.26669100000004,
            "range": "±0.420 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=378.598; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 58.77965280000001,
            "range": "±0.206 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.723; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.4523415,
            "range": "±0.177 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=427.956; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.4174203,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=438.567; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.6408263000000005,
            "range": "±0.320 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.998; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.315291,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.695; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 5.9249472,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.317; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.142655699999999,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=329.184; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.255443099999999,
            "range": "±0.315 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.063; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.2540175,
            "range": "±0.159 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.696; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.339934499999999,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=622.809; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.587732099999999,
            "range": "±0.025 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.119; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.6417631,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=61.224; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 5.8694866,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.093; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 37.019462,
            "range": "±0.210 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=401.506; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.0364423,
            "range": "±0.028 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.416; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.006935800000001,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.668; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 4.7905123000000005,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=31.187; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 44.1745523,
            "range": "±0.347 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=134.724; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 112.4514171,
            "range": "±2.493 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=553.389; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.0032791000000003,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.826; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9752492,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.724; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.1273784,
            "range": "±0.265 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=90.737; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 16.4648842,
            "range": "±0.033 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=274.683; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 10.4896727,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=104.337; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 25.5594811,
            "range": "±0.268 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=302.474; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 10.503198300000001,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.780; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 24.706260099999998,
            "range": "±0.444 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=305.257; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 27.904344,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=237.046; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 27.8999363,
            "range": "±0.207 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=282.102; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.015303799999998,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=258.665; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.356153000000003,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=236.838; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.2750277,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=231.964; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.7400867,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.941; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.6560358,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.442; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.6372116000000005,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.487; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.649218900000001,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.880; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 33.7194771,
            "range": "±0.150 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=189.062; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.289256599999998,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=649.790; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 10.8526771,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.286; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.482434899999998,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=267.505; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 10.880512800000002,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.364; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.1269204,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=142.645; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 18.785427399999996,
            "range": "±0.123 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=256.492; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.759392700000001,
            "range": "±0.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.433; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.5975307,
            "range": "±0.261 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.780; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.5925844,
            "range": "±0.313 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=85.487; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.0989765,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.079; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.0793026,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.709; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.1009687999999995,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.156; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.164257099999999,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.667; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.1604982,
            "range": "±0.025 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.460; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.3091382,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.532; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.0936106000000003,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.069; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.2152479,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.874; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.895734,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.449; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.279374300000001,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.829; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.7520937,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.868; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.2834302,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.649; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.6891306999999998,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.854; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.181181199999999,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.529; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.5883338,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.452; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.169003100000001,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.955; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 8.968734100000002,
            "range": "±0.022 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=108.961; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.1920105,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.313; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.7960199,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.398; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.1307267,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.556; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "b09f4be12951cd04a36e9bbb0dc8c405645ae09b",
          "message": "ci: Migrate create-github-app-token to client-id (#5050)\n\n## Summary\n- The `actions/create-github-app-token` action deprecated `app-id` in\nfavor of `client-id` (warning: `Input 'app-id' has been deprecated with\nmessage: Use 'client-id' instead.`)\n- Replaces `app-id:` with `client-id:` across all workflows and the\n`benchmark-stressgres` composite action\n- Switches from `vars.PARADEDB_GITHUB_APP_ID` (numeric App ID) to\n`vars.PARADEDB_GITHUB_APP_CLIENT_ID` (the App's Client ID, e.g.\n`Iv23li...`)\n\n## Notes\n- Client ID is a public identifier, so `vars.*` is appropriate; the\nPrivate Key remains in `secrets.PARADEDB_GITHUB_APP_PRIVATE_KEY`\n- `vars.PARADEDB_GITHUB_APP_CLIENT_ID` has been added to repo variables\n- The old `vars.PARADEDB_GITHUB_APP_ID` is no longer referenced and can\nbe deleted after merge\n\n## Test plan\n- [ ] Verify cherry-pick, publish-github-release,\npublish-paradedb-docker, test-pg_search-nix, and\nbenchmark-pg_search-stressgres workflows successfully mint a token on\nnext run\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-09T18:11:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/b09f4be12951cd04a36e9bbb0dc8c405645ae09b"
        },
        "date": 1778523825093,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.0847718,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=158.907; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 29.2490312,
            "range": "±0.287 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=798.034; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 66.8334071,
            "range": "±0.138 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.482; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 63.6478221,
            "range": "±0.245 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=879.755; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 24.864733200000003,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.152; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 30.017683499999997,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=829.778; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 52.7031117,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=133.384; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 52.6454557,
            "range": "±0.222 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=827.555; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 59.531900099999994,
            "range": "±0.118 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=147.538; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 59.8772372,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.319; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 39.253390399999994,
            "range": "±0.218 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=291.841; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 28.1565786,
            "range": "±0.130 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=426.869; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.1876101,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=369.654; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.2913458,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=371.500; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.4231077999999995,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.163; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.7452848,
            "range": "±0.203 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.227; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.630883499999999,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.362; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.288925600000001,
            "range": "±0.260 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.401; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 22.3239587,
            "range": "±0.176 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=204.938; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 8.0986146,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.911; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.217990400000001,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.621; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 5.972545,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.125; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 85.8835839,
            "range": "±0.117 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=235.227; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 74.6682175,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=166.698; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 78.2137829,
            "range": "±0.264 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=168.621; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.6937999,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.092; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 206.3380957,
            "range": "±0.443 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=356.149; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 259.6607753,
            "range": "±1.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=351.830; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 281.3428275,
            "range": "±1.488 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=375.915; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 58.2258499,
            "range": "±0.160 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.869; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.3467500999999995,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=334.592; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.4158089,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=385.451; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.6837982,
            "range": "±0.112 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.605; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.551663900000001,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.651; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.1822787,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.964; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.0816479,
            "range": "±0.020 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=390.388; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.6160207,
            "range": "±0.328 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.601; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.753548000000002,
            "range": "±0.198 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.408; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.252245,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=504.058; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.927683900000001,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.557; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.9326343999999995,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.013; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.1377574,
            "range": "±0.030 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.728; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 35.977130599999995,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=432.182; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.5169696,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.722; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.372482299999999,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.148; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.0613705,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.673; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 45.116133299999994,
            "range": "±0.191 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=133.545; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 78.31268359999999,
            "range": "±1.351 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=508.407; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.0140786,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.282; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9621407,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.735; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.615258,
            "range": "±0.223 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.544; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 13.775521699999999,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=351.979; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 10.7164307,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=103.022; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 39.5105798,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=432.274; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 10.9791694,
            "range": "±0.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=160.477; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 38.44788690000001,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=434.111; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 27.980979100000003,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=235.360; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 27.972723000000002,
            "range": "±0.139 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=318.998; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.183773600000002,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=237.510; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.2166449,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=238.314; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.3544456,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=250.387; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.7556880999999995,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.356; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.938012700000001,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.931; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.810683500000001,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.692; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.9086359,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.838; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 34.2974952,
            "range": "±0.177 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=199.009; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 14.989593200000002,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=552.408; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.015111600000001,
            "range": "±0.238 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=109.675; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.715011200000001,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=336.698; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 10.9066789,
            "range": "±0.187 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.552; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.299741899999997,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.737; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 19.0378273,
            "range": "±0.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=316.234; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.7731933,
            "range": "±0.214 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.495; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.6535145,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.645; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.586873100000001,
            "range": "±0.188 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.630; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.378179999999999,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.571; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3691172,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.196; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.407469000000001,
            "range": "±0.081 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.139; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.771687,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.835; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.3303111000000003,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.837; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.683334000000002,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.883; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.2698167000000007,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.030; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.6617656,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.793; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.022605,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.070; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.7083876,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.731; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.8631441000000004,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.026; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.5603459000000015,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.190; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.8284010000000004,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.318; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.584406499999998,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.804; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.6989039999999997,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.251; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.9594462,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.232; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.0429293,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=107.206; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.9401488,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=98.650; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.8435919,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=104.836; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3900672,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.443; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ],
    "pg_search 'stackoverflow' (1m rows)": [
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
          "id": "5ce8f7cabc2743985d08edbeaffb38b3c62f6826",
          "message": "chore: Prepare `0.21.16`. (#4436)\n\n# Description\nBackport of #4434 to `0.21.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: Stu Hood <stuhood@gmail.com>",
          "timestamp": "2026-03-20T02:44:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/5ce8f7cabc2743985d08edbeaffb38b3c62f6826"
        },
        "date": 1778523734105,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 166.4563372,
            "range": "±0.917 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=374.904; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 167.9028937,
            "range": "±0.528 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=411.082; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 509.3211607,
            "range": "±1.647 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=773.520; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 904.4497073,
            "range": "±0.820 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1161.789; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 175.7836879,
            "range": "±0.981 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=405.369; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 173.7521131,
            "range": "±0.395 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=397.421; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 452.8679842,
            "range": "±1.580 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=728.693; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 734.6027384000001,
            "range": "±1.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=996.676; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 461.3212227000001,
            "range": "±0.616 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=704.191; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 459.7944614,
            "range": "±0.518 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=714.233; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 191.0455779,
            "range": "±0.284 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=352.858; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 3177.9003122,
            "range": "±5.698 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3292.851; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 9.712065800000001,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=331.528; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 9.709230000000002,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=282.419; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 10.255471199999999,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=302.399; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 8.0587796,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=282.147; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 8.203286899999998,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=295.248; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 53.4898411,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=205.703; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 36.288368,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=150.852; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 37.748181900000006,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=164.199; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 207.028147,
            "range": "±0.219 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=379.937; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 109.20926899999999,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=265.823; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 114.8339972,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=269.072; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 493.1045967,
            "range": "±0.355 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=656.621; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 287.5214363,
            "range": "±0.113 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=424.390; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 320.2489133,
            "range": "±0.428 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=454.238; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 12.7417158,
            "range": "±0.028 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=362.860; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 10.205390099999999,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=291.116; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 8.0656918,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=277.305; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 7.901300699999998,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=305.114; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 0.0041037999999999995,
            "range": "±0.000 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=0.007; query=SELECT 1 + 1"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 19.3518591,
            "range": "±0.258 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=299.679; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 169.26594070000002,
            "range": "±1.171 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=423.228; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 12.4475559,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=194.456; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 11.1714451,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=171.427; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 11.076716700000002,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=177.744; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 43.2721739,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=186.653; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 36.278312899999996,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=154.885; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 36.25292399999999,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.312; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 161.2611896,
            "range": "±0.446 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=479.568; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 153.891166,
            "range": "±0.567 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=518.614; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 5.800543200000001,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=102.409; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 5.6836844,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=90.908; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 47.3635805,
            "range": "±0.248 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=284.284; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 47.055597899999995,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=250.117; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 71.6435068,
            "range": "±0.624 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=275.226; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 69.7144167,
            "range": "±1.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=340.952; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 32.88622589999999,
            "range": "±0.284 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=378.609; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 33.254994499999995,
            "range": "±0.145 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=354.987; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 46.158868899999995,
            "range": "±0.262 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=458.300; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 42.6928034,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=623.099; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 46.1433412,
            "range": "±0.266 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=590.618; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 68.4859898,
            "range": "±0.204 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=455.008; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 68.62477379999999,
            "range": "±0.297 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=579.245; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 9.986946399999999,
            "range": "±0.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.129; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.251310900000001,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=26.801; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 7.6637568,
            "range": "±0.261 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.723; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 6.894694799999999,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.045; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 64.71076009999999,
            "range": "±0.436 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=347.073; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 28.2526929,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=231.144; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 33.5618567,
            "range": "±0.530 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=326.645; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 33.2474002,
            "range": "±0.230 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=282.568; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 33.7471698,
            "range": "±0.526 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=286.313; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 29.7849103,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=177.083; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 33.142771599999996,
            "range": "±0.232 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=327.163; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 10.3071337,
            "range": "±0.478 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=271.167; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 10.547266099999998,
            "range": "±0.513 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.844; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 10.782816200000001,
            "range": "±0.464 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=298.041; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 5.6434838,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.510; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 5.589499300000001,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.399; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 5.5867915,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.694; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 6.978507400000001,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.387; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.3745365000000005,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=100.252; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.013259500000001,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.290; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 4.1517989,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=99.697; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 6.846164400000001,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.841; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.8289226999999997,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.806; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.0428628,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.509; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.8144730000000004,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=97.131; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 6.981319799999999,
            "range": "±0.118 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.361; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.6893958,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.631; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 6.932633099999999,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.414; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.4537039,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=94.447; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.303495600000002,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=100.462; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 11.092046299999998,
            "range": "±0.207 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=187.903; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.351557599999998,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=99.650; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 11.5126093,
            "range": "±0.222 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=162.212; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 5.722161900000001,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.002; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
          "id": "0b5f5714895d3226ae9025f04f5867cf6e63215d",
          "message": "chore: Upgrade to 0.22.6 (#4694)\n\n## Summary\n- Bump version from 0.22.5 to 0.22.6\n- Add changelog entry for 0.22.6\n- Update version references in docs\n\n## Changes since 0.22.5\n- feat: Support expressions in JoinScan DISTINCT target lists (#4682)\n- fix: JoinScan `DISTINCT` planning for deferred keys (#4670)\n- fix: JoinScan pushdown with outer-only `ORDER BY` pathkeys (#4680)\n- fix: `pdb.score()` with `SELECT` subquery in `WHERE` clause (#4653)\n- fix: Handle `IN (SELECT ...) OR IS NULL` via LeftMark JoinScan (#4651)\n- fix: Score filter for joins and cases without other quals (#4650)\n- fix: Handle pruned columns in nested semi/anti join keys (#4668)\n- fix: Handle aliased indexed expressions in search resolution and top-k\norder by (#4671)\n- fix: Lower x86_64 target-cpu from x86-64-v3 to x86-64-v2 (#4673)\n- fix: Prevent DSM buffer overflow in PG18 parallel index scans (#4683)\n- fix: `PlaceHolderVar found where not expected` error (#4689)\n- fix: Restore `pdb.agg(jsonb)` if accidentally removed by an upgrade\nscript (#4688)\n\n## Test plan\n- [ ] CI passes on 0.22.x\n\n---------\n\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-04-08T18:52:29Z",
          "url": "https://github.com/paradedb/paradedb/commit/0b5f5714895d3226ae9025f04f5867cf6e63215d"
        },
        "date": 1778523942282,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 166.0419615,
            "range": "±0.736 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=431.014; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 171.51702640000002,
            "range": "±0.461 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=437.068; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 513.9306948999999,
            "range": "±0.606 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=785.227; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 918.3856043,
            "range": "±4.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1236.749; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 172.0190354,
            "range": "±0.395 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=435.367; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 172.7494506,
            "range": "±0.263 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=430.593; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 452.7125025,
            "range": "±0.520 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=762.943; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 754.7482084999999,
            "range": "±5.557 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1065.825; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 464.87853690000003,
            "range": "±1.235 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=763.066; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 465.44995140000003,
            "range": "±0.623 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=761.275; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 306.8408254,
            "range": "±0.379 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=619.861; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 3086.8172463,
            "range": "±4.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3260.097; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 26.1918163,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3214.524; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 25.9508017,
            "range": "±0.407 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3161.927; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.593113900000002,
            "range": "±0.160 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2077.244; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.986904500000001,
            "range": "±0.701 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=286.053; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.6677183,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=284.812; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.8100223,
            "range": "±0.364 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.653; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 102.1438258,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.922; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 55.360877300000006,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=235.036; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 56.577332700000014,
            "range": "±0.118 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=237.255; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.849974699999999,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.638; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 225.76276969999998,
            "range": "±0.477 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=446.179; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 281.37377890000005,
            "range": "±0.251 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=467.956; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 289.4446493,
            "range": "±0.274 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=477.513; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 19.25958,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.082; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 599.6579195,
            "range": "±0.684 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=815.355; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 904.1420022,
            "range": "±1.299 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1141.679; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 954.4475623000001,
            "range": "±2.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1208.306; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 92.08612009999999,
            "range": "±0.628 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=117.716; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.667129199999998,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2010.574; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.313589200000003,
            "range": "±0.219 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2027.274; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.5173041,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=285.082; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.471021,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.671; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.291994199999999,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.478; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 49.868085300000004,
            "range": "±0.186 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2129.365; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 166.85500440000004,
            "range": "±0.814 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=471.736; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 235.4294629,
            "range": "±0.406 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=272.258; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 48.671918999999995,
            "range": "±0.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4241.987; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.442614299999999,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=179.052; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 13.466918300000003,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=178.638; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.346988699999999,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.559; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 137.87720480000002,
            "range": "±0.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=822.529; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 55.0696262,
            "range": "±0.118 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=240.467; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 55.09189289999999,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=235.240; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 6.583583899999999,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.039; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 175.43145660000002,
            "range": "±0.422 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=531.311; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 1202.2091696,
            "range": "±19.728 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1887.349; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.3650854999999993,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=102.982; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.4495239,
            "range": "±0.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=90.181; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 57.94115420000001,
            "range": "±0.492 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=328.106; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 33.2924247,
            "range": "±0.385 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=410.951; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 81.01127460000001,
            "range": "±0.693 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=350.198; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 53.258953,
            "range": "±1.259 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=486.527; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.2486471,
            "range": "±0.420 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=333.399; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 53.4820571,
            "range": "±2.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=502.491; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 57.9067539,
            "range": "±0.362 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=580.904; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 39.4025831,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=591.603; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 58.8583437,
            "range": "±0.220 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=572.380; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 53.7135947,
            "range": "±0.387 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=559.623; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 55.265541299999995,
            "range": "±0.252 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=471.258; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.469244300000001,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.603; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.070481200000001,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.440; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.6093496,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.591; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 8.1243707,
            "range": "±0.310 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.714; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 78.7222851,
            "range": "±0.235 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=964.490; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 142.9256704,
            "range": "±0.227 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4617.824; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.4139496,
            "range": "±0.233 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=311.536; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 21.399945600000002,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=299.385; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 35.325954599999996,
            "range": "±0.210 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=287.267; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 15.5856956,
            "range": "±0.229 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=218.931; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 73.52957059999999,
            "range": "±1.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=314.909; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 12.129198800000001,
            "range": "±0.410 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=353.895; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 11.775621699999999,
            "range": "±0.472 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=264.879; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 11.956534100000002,
            "range": "±0.372 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=270.787; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.4084004000000006,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.434; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3475412,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.913; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.310098399999999,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.004; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.665548599999999,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.857; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.057264099999999,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.076; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.718468,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.444; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.7955189999999996,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.052; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.776490999999998,
            "range": "±0.176 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.489; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.4982295,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.560; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.7634918,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.504; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.5276171999999995,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.387; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.7068183,
            "range": "±0.081 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.872; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.321298300000001,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.974; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.746279800000001,
            "range": "±0.158 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.167; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.1273138,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.810; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 11.022292400000001,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=101.588; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 11.128146399999999,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=130.450; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 11.273243899999999,
            "range": "±0.289 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=102.011; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 11.0181302,
            "range": "±0.309 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=126.024; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3921273,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.859; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
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
          "id": "c07921a78f3d24cbb0251b31a1150a7db600af5a",
          "message": "chore: Prepare 0.23.4. (#4997)\n\n# Description\nBackport of #4994 to `0.23.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-06T00:08:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/c07921a78f3d24cbb0251b31a1150a7db600af5a"
        },
        "date": 1778524144819,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 166.41171830000002,
            "range": "±0.758 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=428.611; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 410.60769659999994,
            "range": "±0.311 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1718.952; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 536.4025947,
            "range": "±1.871 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=807.844; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 781.7932971,
            "range": "±4.486 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2131.962; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 171.85385100000002,
            "range": "±0.543 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=440.698; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 437.16844819999994,
            "range": "±1.245 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1782.546; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 470.9878819999999,
            "range": "±1.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=743.525; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 615.2884619,
            "range": "±0.645 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1979.253; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 485.05605180000003,
            "range": "±1.237 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=806.379; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 487.2686496,
            "range": "±0.741 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=812.988; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 305.1618865,
            "range": "±0.383 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=607.090; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 299.66788610000003,
            "range": "±0.307 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1191.499; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 26.075892700000004,
            "range": "±0.226 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3175.658; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 26.2401519,
            "range": "±0.463 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3177.451; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.612674499999994,
            "range": "±0.154 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1981.635; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.0914344,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=266.936; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.3403271,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=266.495; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.2844169,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.407; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 102.516416,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=368.014; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 55.4901824,
            "range": "±0.231 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=220.718; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 56.3735856,
            "range": "±0.206 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=221.761; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.436421499999999,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.348; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 228.5467635,
            "range": "±0.257 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=447.555; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 273.538393,
            "range": "±0.237 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=494.203; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 283.61775389999997,
            "range": "±0.286 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=493.768; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.172916899999997,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=52.578; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 598.167155,
            "range": "±0.701 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=813.418; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 904.6327690999999,
            "range": "±0.951 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1126.123; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 949.7309589,
            "range": "±1.767 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1166.215; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 90.4395789,
            "range": "±0.283 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=118.278; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.6303943,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2031.720; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.3314193,
            "range": "±0.216 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2044.054; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.1190151,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=289.579; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.074077500000001,
            "range": "±0.033 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=296.578; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.098045400000001,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.941; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 49.435136299999996,
            "range": "±0.228 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2006.227; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 166.4932308,
            "range": "±1.687 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=463.473; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 234.65906109999997,
            "range": "±1.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=273.339; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 47.5422306,
            "range": "±0.359 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4515.333; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.8356573,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=177.850; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.8769642,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=184.932; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.1296634999999995,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.950; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 132.0621691,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=912.639; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 54.1535446,
            "range": "±0.099 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=227.531; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 53.939648,
            "range": "±0.148 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=222.205; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.479142900000001,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.337; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 170.17889659999997,
            "range": "±0.395 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=549.928; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 442.236852,
            "range": "±4.437 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1665.626; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.3011861,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=94.170; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.3658989,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=103.683; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 55.405179999999994,
            "range": "±0.672 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=312.007; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 35.1910597,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=523.748; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 83.94493930000002,
            "range": "±1.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=370.957; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 66.80813419999998,
            "range": "±2.630 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=498.448; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.690888799999996,
            "range": "±0.410 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=356.295; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 63.931828199999984,
            "range": "±1.168 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=492.475; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 57.98015410000001,
            "range": "±0.576 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=581.674; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 39.8703033,
            "range": "±0.113 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=613.387; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 60.50525209999999,
            "range": "±0.431 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=565.627; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 54.01245670000001,
            "range": "±0.294 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=570.895; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 57.18774990000001,
            "range": "±0.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=572.224; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.3340105,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=85.248; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.750180899999999,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.169; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.2221506,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.811; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.360942400000001,
            "range": "±0.101 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.916; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 76.2335194,
            "range": "±0.299 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=960.329; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 138.16431849999998,
            "range": "±0.158 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4691.449; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.257786100000004,
            "range": "±0.208 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=314.205; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.1586343,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=385.055; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 35.1657243,
            "range": "±0.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=316.203; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 14.716786599999997,
            "range": "±0.232 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=205.998; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 62.42839140000001,
            "range": "±0.238 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=377.750; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 10.665003100000002,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=280.108; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 10.653004000000001,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=279.803; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 10.5884283,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=279.763; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.148337,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.016; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.2403203000000005,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.973; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.1393686999999995,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.838; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.5072373,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.692; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.9473436,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.501; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.620784300000001,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.195; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.7976758000000004,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.448; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.5454212,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.413; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.4374542,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.916; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.5249511,
            "range": "±0.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.616; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.248635599999999,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.715; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.5446251,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.380; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.1556464999999996,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.972; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.3964240000000006,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.965; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.9913461999999997,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.034; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.705300600000001,
            "range": "±0.210 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=103.070; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 11.0198039,
            "range": "±0.340 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=128.012; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.925829099999998,
            "range": "±0.291 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=102.164; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 11.1236281,
            "range": "±0.295 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=124.106; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.1553995,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.474; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Philippe Noël",
            "username": "philippemnoel",
            "email": "21990816+philippemnoel@users.noreply.github.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "b09f4be12951cd04a36e9bbb0dc8c405645ae09b",
          "message": "ci: Migrate create-github-app-token to client-id (#5050)\n\n## Summary\n- The `actions/create-github-app-token` action deprecated `app-id` in\nfavor of `client-id` (warning: `Input 'app-id' has been deprecated with\nmessage: Use 'client-id' instead.`)\n- Replaces `app-id:` with `client-id:` across all workflows and the\n`benchmark-stressgres` composite action\n- Switches from `vars.PARADEDB_GITHUB_APP_ID` (numeric App ID) to\n`vars.PARADEDB_GITHUB_APP_CLIENT_ID` (the App's Client ID, e.g.\n`Iv23li...`)\n\n## Notes\n- Client ID is a public identifier, so `vars.*` is appropriate; the\nPrivate Key remains in `secrets.PARADEDB_GITHUB_APP_PRIVATE_KEY`\n- `vars.PARADEDB_GITHUB_APP_CLIENT_ID` has been added to repo variables\n- The old `vars.PARADEDB_GITHUB_APP_ID` is no longer referenced and can\nbe deleted after merge\n\n## Test plan\n- [ ] Verify cherry-pick, publish-github-release,\npublish-paradedb-docker, test-pg_search-nix, and\nbenchmark-pg_search-stressgres workflows successfully mint a token on\nnext run\n\n---------\n\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-09T18:11:33Z",
          "url": "https://github.com/paradedb/paradedb/commit/b09f4be12951cd04a36e9bbb0dc8c405645ae09b"
        },
        "date": 1778524280408,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 174.5516157,
            "range": "±0.823 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=460.176; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 411.91975130000003,
            "range": "±0.333 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1798.623; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 509.5194831999999,
            "range": "±0.786 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=803.765; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 775.9039157000001,
            "range": "±1.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2222.926; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 176.40360260000003,
            "range": "±1.272 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=463.934; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 440.86823050000004,
            "range": "±0.293 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1838.278; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 456.6081015,
            "range": "±1.282 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=773.101; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 610.1131278,
            "range": "±0.382 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2046.600; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 464.8401025,
            "range": "±0.266 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=780.686; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 465.8802912000001,
            "range": "±0.886 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=769.129; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 305.9986353,
            "range": "±0.261 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=626.254; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 284.8597614,
            "range": "±3.422 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1167.510; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 25.684875700000003,
            "range": "±0.213 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3218.487; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 26.305557599999997,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3184.493; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.5548602,
            "range": "±0.182 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2033.817; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.637580700000001,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=326.277; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.744778299999998,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=310.607; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.578480300000001,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.249; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 102.55439510000001,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=354.038; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 55.770131500000005,
            "range": "±0.163 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=231.954; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 57.276048700000004,
            "range": "±0.370 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=238.873; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.794384899999999,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.831; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 217.4255948,
            "range": "±0.208 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=432.391; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 290.6690717,
            "range": "±0.729 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=489.329; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 300.6467328,
            "range": "±0.426 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=496.739; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.6768686,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.162; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 566.6276646999999,
            "range": "±0.956 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=775.035; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 925.2082838,
            "range": "±1.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1177.191; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 972.4012842,
            "range": "±1.949 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1212.364; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 89.47146869999999,
            "range": "±0.217 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=121.335; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 22.073228899999997,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2065.239; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.181825500000002,
            "range": "±0.150 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2032.927; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.5897086,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=324.959; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.5195261,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=327.892; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.3673135,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.758; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 49.295631400000005,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2058.911; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 170.9806612,
            "range": "±1.655 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=446.945; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 244.20738459999998,
            "range": "±1.360 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=282.712; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 46.565488800000004,
            "range": "±0.229 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4270.324; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.3120199,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=184.832; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 13.188667600000002,
            "range": "±0.099 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=185.840; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.4257414,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.142; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 137.77743949999999,
            "range": "±0.166 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=875.188; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 54.324394399999996,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=226.083; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 54.2880805,
            "range": "±0.139 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=223.548; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.8605029,
            "range": "±0.242 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.646; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 177.313085,
            "range": "±0.373 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=530.666; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 273.1863951,
            "range": "±5.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1073.583; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.2998305,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.157; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.2876666,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.196; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 58.2212268,
            "range": "±0.592 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=319.670; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 70.26062950000002,
            "range": "±4.880 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=547.573; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 84.32031959999999,
            "range": "±0.538 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=361.995; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 129.49393160000002,
            "range": "±0.300 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=939.689; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.22316549999999,
            "range": "±0.505 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=411.811; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 126.86562169999999,
            "range": "±0.370 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=942.412; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 48.057578899999996,
            "range": "±0.243 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=578.731; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 37.9629884,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=630.470; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 51.220137,
            "range": "±0.188 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=566.650; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 53.4978656,
            "range": "±0.362 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=557.426; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 56.4418015,
            "range": "±0.400 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=568.169; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.3264654,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.818; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.013333,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.923; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.838873900000001,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.891; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.9994855000000005,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.071; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 77.08167989999998,
            "range": "±0.374 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=920.496; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 136.3543021,
            "range": "±0.278 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4648.545; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.1370081,
            "range": "±0.385 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=322.822; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.2900942,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=375.658; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 34.972785,
            "range": "±0.551 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=308.204; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 14.709949800000004,
            "range": "±0.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=208.644; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 65.3333955,
            "range": "±0.753 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=379.091; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 10.962258899999998,
            "range": "±0.404 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=308.608; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 10.819903700000001,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=278.734; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 10.710857899999999,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=277.607; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.3883908,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.276; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.358879900000001,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.748; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.3862021,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.411; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.8862316,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.481; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.721237,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.333; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.856254699999999,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.117; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 4.2821338,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.702; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.8553225000000015,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.695; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.8407057,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.231; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.907737300000001,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.693; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.7750276,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.013; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.948704099999999,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.562; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.5645431000000003,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.380; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.8164191,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.790; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.3121365999999997,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.624; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 11.1275498,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=104.999; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 10.2330301,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=130.569; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 11.2530837,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=103.687; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 10.297717600000002,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=128.369; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.446001999999998,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.488; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ]
  }
}