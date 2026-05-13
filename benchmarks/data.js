window.BENCHMARK_DATA = {
  "lastUpdate": 1778632623120,
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
        "date": 1778531548292,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 23.9486888,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=139.164; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 31.1677845,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=663.239; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 65.8773412,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.646; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 68.7451427,
            "range": "±0.225 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=694.843; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 25.0255232,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=144.372; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 32.64782819999999,
            "range": "±0.215 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=631.889; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 52.07899590000001,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=129.585; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 57.4785974,
            "range": "±0.315 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=614.295; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 55.5966281,
            "range": "±0.147 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=184.779; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 55.76831440000001,
            "range": "±0.127 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=184.076; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 40.210893600000006,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=280.463; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 30.630420200000003,
            "range": "±0.105 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=360.674; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.2607685,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=380.150; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.359291799999999,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=379.978; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.3964831,
            "range": "±0.019 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.125; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.658247,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.973; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.7100117,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.609; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.300176200000001,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.323; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 23.6394376,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=209.954; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 8.0371822,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.487; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.264745599999998,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.275; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 6.1281336,
            "range": "±0.131 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.702; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 100.5995184,
            "range": "±0.345 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=246.573; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 75.5938093,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=172.430; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 78.95215639999999,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=168.786; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.9468944,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.421; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 221.93416729999998,
            "range": "±0.299 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=350.569; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 255.6527723,
            "range": "±0.697 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=350.241; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 277.7128628,
            "range": "±0.623 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=370.936; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 59.306676100000004,
            "range": "±0.394 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=86.668; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.2192826000000005,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=394.820; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.328229,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=396.651; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.592751399999999,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.634; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.587453599999999,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.183; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.1420779,
            "range": "±0.110 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.395; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 7.9232776000000005,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=411.382; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.234129300000001,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.882; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.4384335,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=52.660; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.174923100000001,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=537.334; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.9050607,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.698; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 7.0463148,
            "range": "±0.154 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.952; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.0981836000000005,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.792; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 36.545253599999995,
            "range": "±0.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=385.306; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.257261099999999,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.807; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.2649278,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.383; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 4.993507600000001,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=29.962; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 44.9137338,
            "range": "±0.450 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=124.575; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 111.8129117,
            "range": "±1.875 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=556.214; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.0028899000000004,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.389; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9693605,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.528; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.5908313,
            "range": "±0.165 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.867; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 16.710774999999998,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=273.386; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 10.9865172,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=104.034; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 26.165378099999998,
            "range": "±0.587 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=303.394; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 11.0443974,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.807; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 24.846884799999998,
            "range": "±0.450 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=292.674; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.5559619,
            "range": "±0.220 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=238.288; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 28.160981900000003,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=295.865; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.555605099999998,
            "range": "±0.281 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=241.460; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.444065000000002,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=233.888; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.7294598,
            "range": "±0.276 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=250.597; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.7361369999999994,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.527; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.007479699999999,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.660; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.9495097,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.125; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.9748368,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.210; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 33.7597948,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=194.677; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.1682144,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=503.862; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.253705299999998,
            "range": "±0.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.867; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.932734299999998,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=263.984; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 10.9821286,
            "range": "±0.029 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=89.381; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.5207335,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=139.896; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 19.4279922,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=257.349; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.787257800000001,
            "range": "±0.209 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.799; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.9974408,
            "range": "±0.253 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.479; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 7.017499500000001,
            "range": "±0.250 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.666; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.5281825000000016,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.623; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3726313,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.245; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.396385499999999,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.334; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.536697199999999,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.508; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.1325158,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.893; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.5701348,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.129; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.0741790000000004,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.550; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.5282447,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.326; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.9035188,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.964; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.593379200000001,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.434; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.7348577,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.970; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.558509299999999,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.375; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.7108350000000003,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.301; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.552691800000001,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.712; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.5740342,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.446; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.429217500000002,
            "range": "±0.081 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.287; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 8.994786399999999,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=109.177; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.450863,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=94.855; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.7553395,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.873; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.4535121,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.900; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778613902688,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.2760323,
            "range": "±0.141 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=186.975; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 31.7078176,
            "range": "±0.143 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=778.534; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 65.6273922,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=200.710; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 68.4609715,
            "range": "±0.714 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=956.815; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 24.860996399999998,
            "range": "±0.143 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=200.831; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 32.397143299999996,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1017.161; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 52.584557700000005,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=188.797; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 57.5034692,
            "range": "±0.336 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=934.017; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 55.7465951,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=240.036; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 55.9009595,
            "range": "±0.209 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=244.745; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 40.26659840000001,
            "range": "±0.300 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=437.942; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 30.5689232,
            "range": "±0.113 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=436.801; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.2857634,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=963.679; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.3934073,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=968.048; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.3525286,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=616.051; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.494537300000002,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.853; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.5355382,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.918; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.1031260000000005,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.592; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 22.9576496,
            "range": "±0.140 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=266.550; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 7.906297,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.921; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.0822136,
            "range": "±0.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.219; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 5.877559499999999,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.043; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 88.90747759999998,
            "range": "±0.216 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=246.320; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 70.6043953,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=166.377; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 72.78565919999998,
            "range": "±0.232 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=162.406; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.7544653,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.679; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 220.56778300000005,
            "range": "±0.337 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=362.353; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 254.7114627,
            "range": "±0.300 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=342.700; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 278.48705789999997,
            "range": "±0.376 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=362.927; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 59.119495099999995,
            "range": "±0.388 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=87.937; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.337643699999999,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=277.961; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.3527491000000005,
            "range": "±0.020 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=279.253; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.502098999999999,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.125; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.465713400000001,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.616; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 5.9807986,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.927; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.0425215,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=292.490; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.217473599999998,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.430; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.320013,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.509; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.3184365,
            "range": "±0.019 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=453.436; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.7967404,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.086; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.7752334,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.171; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.0175023,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.160; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 36.91587919999999,
            "range": "±0.138 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=353.472; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.270851199999998,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.127; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.130536999999999,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.182; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 4.8175349999999995,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.077; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 44.314295300000005,
            "range": "±0.324 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=184.603; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 109.5964755,
            "range": "±0.219 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=693.927; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.9882812000000003,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.454; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9419886,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.209; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.254494900000005,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=86.099; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 16.4573688,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=263.325; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 10.884387800000002,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=99.076; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 25.4759112,
            "range": "±0.484 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=290.392; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 10.8487921,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=139.785; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 24.721241499999998,
            "range": "±1.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=532.931; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.147574400000003,
            "range": "±0.193 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=461.235; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 27.8485389,
            "range": "±0.232 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=266.452; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 30.9589517,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=227.917; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.2117974,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=220.532; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.3248168,
            "range": "±0.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=446.487; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.7230333,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=146.791; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.7447654,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.388; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.7565762,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.589; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.7264558999999995,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.673; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 33.503365099999996,
            "range": "±0.140 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=305.474; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.3015117,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=477.721; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 10.8971851,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.751; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.640780900000001,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=400.952; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.0092768,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=127.192; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.266493299999999,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=264.876; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 18.9016675,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=393.234; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.642630299999999,
            "range": "±0.277 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.005; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.5810382,
            "range": "±0.258 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.876; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.362702199999999,
            "range": "±0.200 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.228; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.212668000000001,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.160; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.1393319,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.878; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.184592599999998,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.145; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.4576023,
            "range": "±0.099 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=112.127; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.1457299,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=138.272; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.347293499999999,
            "range": "±0.033 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=107.794; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.0812409,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=133.893; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.332207200000001,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.405; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.8908956000000003,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.266; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.5088628,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.935; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.7590210000000006,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.543; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.388424599999999,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=61.779; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.7077531999999995,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=61.120; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.412997,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.091; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.5845842000000006,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.921; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.3075161,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=88.673; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 8.9557177,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=100.232; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.301658999999999,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=90.007; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.7674425,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=98.662; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.261059000000001,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.090; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778630711374,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.3361745,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=157.217; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 29.5463679,
            "range": "±0.138 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=780.592; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 2",
            "value": 29.6624988,
            "range": "±0.196 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=646.843; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 66.38206840000001,
            "range": "±0.131 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=167.638; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 64.0686278,
            "range": "±0.424 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=837.269; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 2",
            "value": 443.2128162,
            "range": "±1.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=659.870; query=SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 25.199681900000005,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=147.279; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 32.1223595,
            "range": "±0.234 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=771.723; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 2",
            "value": 31.668914100000002,
            "range": "±0.146 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=590.174; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 54.7621387,
            "range": "±0.207 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=149.603; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 53.5617735,
            "range": "±0.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=784.769; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 2",
            "value": 431.5705945000001,
            "range": "±0.652 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=636.777; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 55.9215236,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=196.298; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 56.3938848,
            "range": "±0.122 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=195.110; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 39.32831610000001,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=292.816; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 28.6953393,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=396.821; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.137292,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=366.834; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.2421354000000004,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=380.147; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.417996799999999,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.866; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.6903466,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.380; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.7275431,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.694; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.3871968,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.699; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 22.961560799999997,
            "range": "±0.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=209.315; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 8.220926899999998,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.334; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.464969,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.193; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 6.072679800000001,
            "range": "±0.029 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.341; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 98.2329387,
            "range": "±0.615 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=262.054; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 75.08208210000001,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=166.315; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 77.02198430000001,
            "range": "±0.190 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=168.494; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.9339033,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.271; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 210.0864987,
            "range": "±0.278 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=389.913; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 257.90684300000004,
            "range": "±0.584 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=343.285; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 282.36978669999996,
            "range": "±0.826 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.294; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 57.78665470000001,
            "range": "±0.445 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.797; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.2932609,
            "range": "±0.022 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=318.519; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.410815,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=389.933; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.7657391,
            "range": "±0.110 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.282; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.618588,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.821; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.2325617,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.538; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.151072899999999,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=383.378; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.333530999999999,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.899; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.6506265,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.007; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.1383125,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=542.042; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.901627700000001,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.753; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.9238073,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.896; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.407377200000001,
            "range": "±0.520 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.894; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 36.8837905,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=393.843; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.4560783000000015,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.772; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.491793400000001,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.133; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.1434854,
            "range": "±0.311 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.288; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 44.2414045,
            "range": "±0.376 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=128.647; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 78.48166900000001,
            "range": "±1.454 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=468.314; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.9968969000000003,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.937; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9449587,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.142; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.6633622,
            "range": "±0.117 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=89.949; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 13.8991435,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=345.498; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 12.3676925,
            "range": "±0.279 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=110.202; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 39.7140222,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=427.770; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 11.141913900000002,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=173.695; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 38.540592700000005,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=433.716; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.7234599,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=245.461; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 28.3330918,
            "range": "±0.147 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=314.257; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.704624199999994,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=247.252; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.7079592,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=281.648; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.758716099999997,
            "range": "±0.233 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=267.112; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.7193554,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.165; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.001736800000001,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.131; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.925681300000001,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.779; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.9622387,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.935; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 34.4513482,
            "range": "±0.220 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=199.993; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.0460026,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=511.940; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.1596714,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.209; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.813046100000003,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=312.158; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.039809700000001,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=98.983; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.320271199999999,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=170.339; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 19.2236626,
            "range": "±0.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=327.410; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.6337481,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=85.009; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.690602199999999,
            "range": "±0.280 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.422; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.5771795000000015,
            "range": "±0.170 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.925; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.329823099999999,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.281; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3350843999999995,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.697; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.3504664,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.455; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.5149645000000005,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.186; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.2900412,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.995; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.6203796,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.454; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.2349230999999996,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.790; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.5094714,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.005; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.9676785,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.563; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.6081452999999994,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.279; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.8440996000000003,
            "range": "±0.025 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.545; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.5411916,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.309; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.8107958999999996,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.803; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.5463894,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.912; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.6461908999999997,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.726; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.5095747,
            "range": "±0.123 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.506; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.0022525,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=107.478; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.4821892,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.526; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.8382406,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=108.431; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.525763899999999,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.144; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "james.sewell@gmail.com",
            "name": "James Sewell",
            "username": "jamessewell"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8182eaf110c30cbefe008197caa40efa8b44f8e0",
          "message": "refactor: use existing FFHelper ctid cache instead of dedicated cache (#4905)\n\nFix a performance regression introduced in e0804b347 (#4765) which\nremoved ctid from SearchIndexScore and switched to lazy per-row\nresolution.\n\nPrior to #4765, ctid was resolved during result construction and carried\nin `SearchIndexScore` — no per-row fast-field lookups needed. #4765\nmoved ctid resolution to the consumption side (top_k.rs, normal.rs,\nscan.rs) using a single-entry `Option<(SegmentOrdinal, FFType)>` cache.\nWhen TopK results interleave across segments (sorted by score), every\nsegment transition re-opens the ctid column via `FastFieldReaders::u64\n-> DynamicColumnHandle::open -> BlockwiseLinearCodec::load`, which is\nvery expensive. Profiling showed 45% of total cycles spent in this\nre-open path.\n\nThe columnar scan path (`ColumnarExecState`) was unaffected — it already\nused `FFHelper`'s per-segment `OnceLock` ctid cache. This PR brings the\nremaining paths in line:\n\n- `scan.rs` uses its existing `Bm25ScanState.fast_fields` FFHelper\n- `normal.rs` and `top_k.rs` use a new `ctid_cache` FFHelper on\n`BaseScanState`\n\nEach segment's ctid column is opened at most once via `OnceLock`,\neliminating the thrashing. `FFHelper` has had this per-segment ctid\ncaching built in since cb78f0ca2 (Oct 2024).",
          "timestamp": "2026-05-13T12:18:31+12:00",
          "tree_id": "814e1da895eec41e0dfe3cbb5348bdb237811bf7",
          "url": "https://github.com/paradedb/paradedb/commit/8182eaf110c30cbefe008197caa40efa8b44f8e0"
        },
        "date": 1778632597516,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 23.909201,
            "range": "±0.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=161.032; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 29.1912103,
            "range": "±0.181 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=606.845; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 2",
            "value": 29.200845400000002,
            "range": "±0.185 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=487.334; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 65.7579799,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=166.872; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 64.24596640000001,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=658.518; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 2",
            "value": 443.7195464,
            "range": "±0.835 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=638.971; query=SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 24.922572399999996,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=157.021; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 31.087400600000002,
            "range": "±0.118 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=603.358; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 2",
            "value": 31.255545199999993,
            "range": "±0.185 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=495.566; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 52.78236550000001,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=155.144; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 52.389726700000004,
            "range": "±0.196 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=642.472; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 2",
            "value": 423.451149,
            "range": "±0.591 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=629.478; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 59.482159100000004,
            "range": "±0.105 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=163.577; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 59.6883924,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=164.151; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 38.2185713,
            "range": "±0.110 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=310.975; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 28.5824802,
            "range": "±0.116 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=416.355; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.179770400000001,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=385.882; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.273876500000001,
            "range": "±0.022 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=387.234; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.406807199999999,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=431.125; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.5289491,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.960; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.5711662,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.762; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.1613521,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.105; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 22.0204819,
            "range": "±0.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=212.572; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 7.882265400000001,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.203; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.171880300000002,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.042; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 5.8715152999999995,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.221; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 85.7193919,
            "range": "±0.467 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=226.927; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 74.0634369,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=163.807; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 77.0806811,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=168.895; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.803030000000001,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.522; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 206.59109479999998,
            "range": "±0.326 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.531; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 258.8511163,
            "range": "±0.713 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=346.070; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 281.9921369,
            "range": "±0.285 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=366.914; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 58.209896900000004,
            "range": "±0.333 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=90.904; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.296064400000001,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=345.036; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.3846127,
            "range": "±0.020 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=332.048; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.5232443,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.482; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.424461000000001,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.962; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.0094554,
            "range": "±0.030 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.426; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.012220300000001,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=418.177; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.189918599999999,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.258; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.4110913,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.608; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.205757999999999,
            "range": "±0.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=535.020; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.7924922,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.145; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.773528999999999,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.979; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.0750125,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.436; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 36.9341462,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=381.540; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.132618099999999,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.056; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.181320000000001,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.039; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 4.9051286,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.030; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 43.900145800000004,
            "range": "±0.281 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=136.291; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 76.577954,
            "range": "±0.250 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=485.290; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.9575250000000004,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.001; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9091059000000006,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.741; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.8247689,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=116.809; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 13.673395099999999,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=348.061; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 12.750158899999999,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=142.155; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 39.3339747,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=419.740; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 10.945050599999998,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.459; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 38.3974427,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=413.367; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 29.107741200000003,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=273.621; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 27.990355599999997,
            "range": "±0.185 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=288.752; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 32.160383700000004,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=294.188; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.366787,
            "range": "±0.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=244.805; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.451655,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=247.600; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.800997400000001,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.971; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.170000600000001,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.049; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 6.1825598,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.223; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 6.2651451,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.269; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 34.56662680000001,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=227.041; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 14.683709099999998,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=670.012; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 10.949782399999998,
            "range": "±0.028 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=100.317; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.876498700000003,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=299.436; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 10.8751994,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=98.553; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.639388799999999,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=183.970; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 19.087052800000002,
            "range": "±0.187 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=302.401; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.8427141,
            "range": "±0.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=89.382; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.7948352,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=86.701; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.8113843,
            "range": "±0.182 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.281; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.60394,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.367; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.624181700000001,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.146; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.5451292,
            "range": "±0.020 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.044; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.781768,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.339; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.3963210000000004,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.671; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.8752329,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.837; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.2919396999999995,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.632; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.751247599999999,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.829; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.0517415,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.282; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.7601123,
            "range": "±0.033 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.825; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.947018,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.872; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.7883528,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.332; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.8914233,
            "range": "±0.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.971; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.7236809,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.099; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.7307717,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.351; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.7389099,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.424; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.0498902,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=118.387; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.6810354,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.431; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.913908600000001,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=117.022; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.586522,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.500; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778614366576,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 167.04128419999998,
            "range": "±0.505 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=445.363; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 411.17415520000003,
            "range": "±0.356 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2016.107; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 516.6099644,
            "range": "±0.929 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=762.596; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 763.6199216000001,
            "range": "±1.365 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2367.141; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 173.21354829999999,
            "range": "±0.533 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=454.393; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 435.6178672,
            "range": "±0.333 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2072.789; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 454.24389579999996,
            "range": "±0.606 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=724.374; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 624.4385675999999,
            "range": "±0.868 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2181.859; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 468.09247619999996,
            "range": "±1.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=712.843; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 467.6064772,
            "range": "±0.529 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=739.963; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 307.5314188,
            "range": "±1.322 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=641.182; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 299.9079866,
            "range": "±0.385 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1207.438; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 26.398548000000005,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3704.505; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 26.593867499999998,
            "range": "±0.141 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2956.897; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.9071963,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2396.142; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.3852898,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=259.837; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.685652400000002,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=224.973; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.4394760999999985,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.869; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 102.3126723,
            "range": "±0.225 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=343.031; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 54.6842306,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=263.054; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 56.239128500000014,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=254.818; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.621456400000001,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.184; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 227.88570440000004,
            "range": "±0.379 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=431.131; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 277.26116040000005,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=454.420; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 286.65126200000003,
            "range": "±0.233 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=467.398; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.5968312,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.867; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 599.1499847,
            "range": "±0.572 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=799.010; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 920.5923296000001,
            "range": "±3.345 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1147.891; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 963.4731914,
            "range": "±3.467 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1195.575; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 90.7663094,
            "range": "±0.315 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=129.337; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.947826799999998,
            "range": "±0.126 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2955.093; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.394429300000002,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2714.429; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.3884422,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=258.448; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.410889,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=222.592; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.2141136,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.100; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 49.055743299999996,
            "range": "±0.193 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2452.687; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 166.990028,
            "range": "±1.202 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=410.676; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 234.2899329,
            "range": "±1.318 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=275.100; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 46.883337600000004,
            "range": "±0.241 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4358.491; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.0978479,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=160.558; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 13.299827200000001,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=168.339; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.2610603000000005,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.675; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 133.4196007,
            "range": "±0.330 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1009.153; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 53.1866676,
            "range": "±0.147 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=245.219; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 53.248146,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=261.166; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.6586098,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.839; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 174.49003270000003,
            "range": "±0.723 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=454.642; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 446.3158044,
            "range": "±2.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1421.146; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.253029,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=88.197; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.3410095999999996,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=85.623; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 56.434851400000014,
            "range": "±0.557 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=294.155; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 34.864097900000004,
            "range": "±0.166 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=451.550; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 83.73189169999999,
            "range": "±0.441 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.023; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 65.73982709999999,
            "range": "±2.713 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=466.655; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.61415219999999,
            "range": "±0.267 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=369.232; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 68.6375138,
            "range": "±5.156 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=893.130; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 47.4428594,
            "range": "±0.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=622.878; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 37.439744399999995,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=646.529; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 50.314799099999995,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=718.095; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 52.532484499999995,
            "range": "±0.298 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=561.056; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 55.23310359999999,
            "range": "±0.239 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=647.439; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.2868905,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=196.793; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.974715,
            "range": "±0.160 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.617; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.3084684,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.791; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.923483499999999,
            "range": "±0.889 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=52.150; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 76.67353339999998,
            "range": "±0.342 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=984.993; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 138.6033453,
            "range": "±0.345 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4119.956; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.5868906,
            "range": "±0.740 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=233.218; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.3575899,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=528.852; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 35.28673010000001,
            "range": "±0.219 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=276.711; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 15.8376489,
            "range": "±0.912 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=398.686; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 62.974826900000004,
            "range": "±0.136 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=522.794; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 10.615240199999999,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=295.697; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 10.6985745,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=284.984; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 10.7699693,
            "range": "±0.130 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.475; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.404735499999999,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.758; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.2379679999999995,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.155; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.2159668,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.833; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.6468565,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=151.957; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.9875493000000004,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=179.146; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.6364225,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=120.951; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.7397205,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=117.040; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.6504529,
            "range": "±0.105 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.836; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.4181091,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=112.160; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.621651,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.473; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.2232560999999995,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.955; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.6386747,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.197; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.124832,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.637; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.6159300000000005,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.342; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.9846899000000002,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.643; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.760707499999999,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.067; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 10.1948992,
            "range": "±0.140 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=118.001; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.905614799999999,
            "range": "±0.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.202; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 11.3626141,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=114.409; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3579103,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.734; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
          "id": "6bdea0d414fb563f086ddfe56712b2960d200626",
          "message": "feat(mpp): AggregateScan in-process MPP via custom shm_mq transport (#4988)\n\n# Ticket(s) Closed\n\n- Closes #4152\n\n## What\n\nAdds an MPP execution path for AggregateScan that runs\njoin-with-aggregate shapes inside Postgres parallel-worker processes via\na custom transport on top of `paradedb/datafusion-distributed`. Default\noff behind `paradedb.enable_mpp` (minimum `mpp_worker_count = 3`). Other\nshapes and the JoinScan path are unchanged.\n\n## Why\n\nSingle-process DataFusion bottlenecks on join-with-aggregate at scale.\nDistributing the producer fragment across PG parallel workers gives us\n1.55–1.92× speedup on the 25M `aggregate_join_groupby` bench at N=2/4\nwithout leaving the embedded model — every worker is still a real PG\nprocess with its own snapshot, no gRPC, no extra daemon.\n\n## How\n\n- Leader builds the logical plan and stashes it in a DSM segment\nalongside an N-way `shm_mq` mesh.\n- Workers attach, deserialize, and re-plan with the same `SessionState`.\nIdentical inputs ⇒ structurally identical physical plans on every\nworker, so we don't need to serialize physical subplans.\n- Each worker runs its producer fragment and pushes batches through its\noutbound queue. Leader runs `NetworkShuffleExec` + final aggregate and\nreturns rows to the client. Leader is consumer-only in this iteration.\n- Build side (non-partitioning sources like `HashJoinExec(CollectLeft)`)\nis split via DSM all-gather: each worker scans its 1/N slice, writes to\na per-worker DSM region, completion-flag barrier, then everyone reads\nevery slice. Build is fully parallel; no leader-side serial scan.\n- The fork emits the network operators (`NetworkShuffleExec`,\n`NetworkBroadcastExec`, `NetworkCoalesceExec`) and we register a custom\ntransport that short-circuits the gRPC dialer. The fork's in-process\ntwo-boundary planner distinguishes outer (worker → leader, N producers)\nfrom nested (single local producer) Network boundaries.\n\n## Reviewer's Guide\n\nSuggested reading order — most of the diff lives under\n`pg_search/src/postgres/customscan/mpp/`:\n\n1. `mpp/dsm.rs` — the DSM layout: header, queue mesh, build-cache\nregion. `compute_dsm_layout` is the math; `leader_init` /\n`worker_attach` are the unsafe FFI boundaries.\n2. `mpp/runtime.rs` — `MppMesh` (runtime handle), `ShmMqWorkerTransport`\n(the `WorkerTransport` impl the leader registers),\n`LocalExecWorkerTransport` (the worker-side stub for nested broadcasts),\n`MppWorkerResolver`.\n3. `mpp/transport.rs` — `DrainHandle` and the cooperative-pull\nprimitives. The drain runs inline on the backend thread because pgrx\n0.18 enforces single-threaded Postgres FFI.\n4. `mpp/glue.rs` — the public API the customscan calls:\n`estimate_dsm_size`, `leader_setup`, `worker_setup`. Thin wrappers\naround dsm/runtime.\n5. `mpp/exec.rs` — `run_producer_fragment` is the worker push loop.\n6. `aggregatescan/mod.rs` — the integration: `stash_mpp_plan_bytes`,\n`exec_mpp_worker`, `build_mpp_leader_session_context`, the\n`ParallelQueryCapable` impl, the `parallel_workers` clamp in\n`try_build_datafusion_aggregate_path`. Both leader and worker session\ncontexts call `with_distributed_in_process_mode(true)` explicitly.\n\nThe build-side all-gather lives in\n`aggregatescan/mod.rs::exec_mpp_worker` and `mpp/dsm.rs` (the cache\nregion). Read those together.\n\n## Tests\n\n- pgrx regression suite — new `mpp_aggregate.sql` covers correctness on\nthe join-with-aggregate shapes; existing `mpp_*` suites still pass.\n- 25M `aggregate_join_groupby` bench: byte-exact result vs serial\nDataFusion at N=2/4/8/10.\n- Build-side all-gather is exercised at all N in the regression suite\n(workers vs leader-only-writer paths).\n\n---------\n\nCo-authored-by: paradedb-bot <developers@paradedb.com>\nCo-authored-by: paradedb-github-app[bot] <282009505+paradedb-github-app[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-12T16:47:00-07:00",
          "tree_id": "38dfa579bfcab58f4b6b66ddea91de57c32f5204",
          "url": "https://github.com/paradedb/paradedb/commit/6bdea0d414fb563f086ddfe56712b2960d200626"
        },
        "date": 1778631203136,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 171.7548191,
            "range": "±0.602 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=454.431; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 409.6177501,
            "range": "±0.293 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1845.246; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 2",
            "value": 416.4163765,
            "range": "±0.353 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1690.995; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 517.0235897,
            "range": "±1.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=832.627; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 772.9339085999999,
            "range": "±1.209 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2253.439; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 2",
            "value": 863.3488556999998,
            "range": "±2.002 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1427.290; query=SET statement_timeout TO '600s'; SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 175.7001799,
            "range": "±1.369 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=454.952; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 429.3468397,
            "range": "±0.352 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1822.402; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 2",
            "value": 434.2180215,
            "range": "±0.319 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1730.189; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 458.1689137,
            "range": "±1.671 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=763.767; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 600.4326814,
            "range": "±0.378 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2069.940; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 2",
            "value": 698.7793308,
            "range": "±3.294 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1265.742; query=SET statement_timeout TO '600s'; SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SET paradedb.enable_mpp TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 467.4635432,
            "range": "±0.549 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=807.646; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 466.9530253,
            "range": "±0.546 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=789.298; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 302.7088236000001,
            "range": "±0.459 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=675.093; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 283.8827945,
            "range": "±0.707 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1222.672; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 25.2317493,
            "range": "±0.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3237.758; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 25.209462,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3249.066; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.0973322,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2077.878; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.4330397,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.244; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.656486600000001,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=292.353; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.515168300000001,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.170; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 100.00676580000001,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=384.490; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 53.3012677,
            "range": "±0.228 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=260.690; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 54.703563900000006,
            "range": "±0.080 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=275.541; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.7898483,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.956; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 221.27144570000004,
            "range": "±0.232 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=452.571; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 281.7611243,
            "range": "±1.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=477.991; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 289.26180389999996,
            "range": "±1.680 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=487.828; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.509093899999996,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.844; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 573.9853981,
            "range": "±0.596 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=791.719; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 909.8090943000001,
            "range": "±2.241 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1159.779; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 947.3654659999999,
            "range": "±2.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1218.278; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 88.86169410000001,
            "range": "±0.290 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=121.217; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.494283199999998,
            "range": "±0.241 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2139.026; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.167397200000003,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2120.955; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.355587400000001,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=278.018; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.358859899999999,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=276.298; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.3623168,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.458; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 48.88630259999999,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2178.893; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 169.9924321,
            "range": "±1.264 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=486.408; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 237.6484764,
            "range": "±1.456 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=284.545; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 45.6932854,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4505.060; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.9075275,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=199.728; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.954945600000002,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=202.841; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.4402892,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.785; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 131.46248129999998,
            "range": "±0.235 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=854.600; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 52.648997300000005,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=267.542; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 52.6903275,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=259.139; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.8929157000000005,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.599; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 172.583669,
            "range": "±0.407 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=521.846; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 274.3764937,
            "range": "±3.893 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1089.284; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.2299798,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.419; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.2265131000000005,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.851; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 56.4240095,
            "range": "±0.472 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=315.800; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 70.65607349999999,
            "range": "±4.528 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=509.062; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 82.8060302,
            "range": "±0.448 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=364.087; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 129.15100389999998,
            "range": "±0.267 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=878.436; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.0550209,
            "range": "±0.329 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=419.641; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 128.06822079999998,
            "range": "±0.168 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=883.082; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 48.1150095,
            "range": "±0.206 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=547.295; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 37.6744738,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=619.270; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 50.924389700000006,
            "range": "±0.159 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=603.150; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 52.4579306,
            "range": "±0.390 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=560.330; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 55.584920999999994,
            "range": "±0.311 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=507.731; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.253845599999999,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.250; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.9610759,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=31.840; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.568184800000001,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.304; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.909529100000002,
            "range": "±0.254 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.601; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 77.18541719999999,
            "range": "±0.441 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=952.091; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 136.811206,
            "range": "±0.202 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4644.069; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 34.8688868,
            "range": "±0.453 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=318.875; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.244952200000004,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=371.389; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 34.6592949,
            "range": "±0.564 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=321.986; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 14.655622899999997,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=211.537; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 65.0589113,
            "range": "±0.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=375.120; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 10.8409145,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=285.126; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 10.7255245,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=280.314; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 10.8684524,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=287.620; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.3434408,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.701; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3019587999999995,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.769; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.393929900000001,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.015; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.862028999999998,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.298; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.7028266,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.735; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.769527600000001,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.117; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 4.2789775,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.430; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.6495254,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.029; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.8658488999999996,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.315; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.8243721,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.924; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.7465853999999994,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.098; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.7648991999999994,
            "range": "±0.136 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.819; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.494452699999999,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.110; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.748530499999999,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.741; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.2337919,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.478; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.876520300000003,
            "range": "±0.218 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.638; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 10.209993700000002,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=132.999; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.7728552,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=114.114; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 10.3394742,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=131.833; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.350905799999999,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.498; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ],
    "pg_search 'stackoverflow' (20m rows)": [
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
        "date": 1778527661877,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 2935.5015079999994,
            "range": "±23.752 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3974.678; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 3786.7217113000006,
            "range": "±26.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4140.560; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 9438.960825500002,
            "range": "±21.521 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10375.727; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 25504.325618,
            "range": "±77.628 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=25845.748; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 3082.6631780999996,
            "range": "±23.241 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4057.745; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 3887.0212880999998,
            "range": "±20.945 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4275.874; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 8345.278115000001,
            "range": "±21.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9369.650; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 21419.865404000004,
            "range": "±41.143 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=21691.645; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 8608.5883298,
            "range": "±32.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9520.035; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 8614.659066,
            "range": "±17.890 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9540.271; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 4788.5197961,
            "range": "±24.577 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5164.308; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 6496.571582499999,
            "range": "±20.349 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6872.331; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 98.8762413,
            "range": "±0.208 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1724.992; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 99.3272851,
            "range": "±0.327 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1719.078; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 98.36241439999999,
            "range": "±0.403 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1715.781; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 62.3756657,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1715.131; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 63.88757660000001,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1714.044; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 1639.9662119,
            "range": "±19.311 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1945.761; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 1268.8994733999998,
            "range": "±6.499 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1612.116; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 1338.4746476,
            "range": "±20.859 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1622.010; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 334.4080492,
            "range": "±0.211 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=579.183; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 177.03741,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=409.012; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 187.1072753,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=418.985; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 904.5688947000001,
            "range": "±0.668 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1160.263; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 509.19993240000014,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=742.087; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 553.4477521,
            "range": "±0.535 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=788.257; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 153.71730999999997,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1759.771; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 98.4038166,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1742.136; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 61.871575400000005,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1671.984; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 61.8581268,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1721.725; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 0.0042289,
            "range": "±0.000 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=0.007; query=SELECT 1 + 1"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 261.8399371,
            "range": "±0.328 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1902.955; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 4515.0521733000005,
            "range": "±3.489 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6090.318; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 165.88139619999998,
            "range": "±0.112 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1337.244; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 137.99686839999998,
            "range": "±0.400 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1256.082; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 134.94079770000002,
            "range": "±0.225 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1335.956; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 1516.5312593,
            "range": "±21.373 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1834.209; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 1255.3962811000001,
            "range": "±10.733 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1631.785; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 1242.7800293,
            "range": "±4.622 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1557.449; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 2238.1302268000004,
            "range": "±3.141 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4692.770; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 2926.5202347,
            "range": "±9.928 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4548.678; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 5.140405100000001,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=26.632; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 4.9707129,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=25.320; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 189.59988310000003,
            "range": "±1.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3424.311; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 151.6075884,
            "range": "±2.355 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3393.650; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 684.1398310000001,
            "range": "±1.499 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4355.938; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 347.23676340000003,
            "range": "±3.745 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3712.964; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 124.35952689999999,
            "range": "±0.835 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3527.900; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 124.4300704,
            "range": "±1.208 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3535.622; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 399.77122449999996,
            "range": "±0.390 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4502.706; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 340.3478056999999,
            "range": "±0.365 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4550.136; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 399.85036149999996,
            "range": "±0.408 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4527.256; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 453.10702979999996,
            "range": "±0.822 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4444.479; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 451.3592893,
            "range": "±0.818 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4541.624; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.2087787,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=97.894; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.3637522,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=30.945; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 13.993809099999998,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=52.968; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 13.3552425,
            "range": "±0.123 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.493; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 441.9971366,
            "range": "±1.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1786.067; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 520.5195211,
            "range": "±0.881 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1710.211; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 535.0452959,
            "range": "±1.627 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2272.423; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 534.3444938999999,
            "range": "±2.229 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2290.903; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 532.7631459,
            "range": "±3.311 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2280.286; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 232.18571330000003,
            "range": "±0.170 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=648.309; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 532.9927939000002,
            "range": "±0.654 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2296.074; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 71.92294919999999,
            "range": "±0.364 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1735.644; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 71.50399580000001,
            "range": "±0.493 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1713.727; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 71.6105432,
            "range": "±0.476 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1703.592; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 7.624220199999999,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.464; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.888184900000001,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.233; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.790568500000001,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.942; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 12.0308896,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.229; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 19.844967899999997,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=206.713; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 11.4188206,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.631; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 16.0117849,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=185.452; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 11.0023705,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=109.150; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 13.146050300000002,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=185.783; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 11.5914592,
            "range": "±0.118 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.642; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 16.4095278,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=198.074; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 11.280944199999999,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=108.733; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 13.525296299999999,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=177.438; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 10.911888399999999,
            "range": "±0.160 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=109.333; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 12.302605400000001,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=181.406; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 19.4469389,
            "range": "±0.259 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=156.850; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 44.0893162,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=337.752; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 20.221614099999996,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=161.091; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 44.58572399999999,
            "range": "±0.171 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=333.884; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.9479568,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.594; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778529288244,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 4172.8888396,
            "range": "±6.496 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13087.839; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 5012.6794347,
            "range": "±27.449 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13598.348; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 9804.9082867,
            "range": "±16.654 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10875.620; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 27058.354775199998,
            "range": "±122.386 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=27859.760; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 4379.3552291,
            "range": "±10.533 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13340.766; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 5154.903336400001,
            "range": "±16.777 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13826.492; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 8656.566755100002,
            "range": "±33.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9890.464; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 22100.964956899996,
            "range": "±81.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=22769.003; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 8982.261525,
            "range": "±32.874 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10112.064; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 8960.1552927,
            "range": "±38.465 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10145.568; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 5174.1094759,
            "range": "±25.289 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5733.290; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 8157.1331034,
            "range": "±27.310 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=19798.289; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 293.2346508,
            "range": "±0.546 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=20318.964; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 293.2643631,
            "range": "±0.571 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=20254.803; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 224.4280802,
            "range": "±0.173 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14458.274; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 90.405722,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8081.507; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 88.99171540000002,
            "range": "±0.166 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8191.495; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 10.5059071,
            "range": "±0.394 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.959; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 3339.9232882999995,
            "range": "±52.765 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12537.401; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 3281.8426112,
            "range": "±96.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14525.834; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 3228.7491991,
            "range": "±78.196 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14668.622; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 43.3208596,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.467; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 369.90265009999996,
            "range": "±0.400 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=681.908; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 539.8678794,
            "range": "±1.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=859.453; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 558.7267989,
            "range": "±1.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=881.355; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 29.5753779,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.811; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 1236.5417499999999,
            "range": "±2.257 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1536.853; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 1943.9401584,
            "range": "±0.814 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2526.463; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 2007.0226404,
            "range": "±0.532 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2627.126; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 129.14678999999998,
            "range": "±0.358 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=157.065; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 294.11490779999997,
            "range": "±0.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14598.701; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 223.4439692,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14602.447; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 88.2548769,
            "range": "±0.101 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8093.447; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 88.37155179999999,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7953.670; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 8.416945600000002,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.529; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 462.1043486,
            "range": "±0.590 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4436.028; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 4680.3145319,
            "range": "±7.600 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12487.486; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 4865.937675399999,
            "range": "±9.535 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5067.494; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 253.63029820000003,
            "range": "±0.258 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16159.590; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 212.59362240000002,
            "range": "±0.131 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12982.156; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 213.57698190000002,
            "range": "±0.375 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13019.716; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 12.987737099999999,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.123; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 3502.2114196,
            "range": "±82.761 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15322.018; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 3221.1743437,
            "range": "±64.677 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14408.054; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 3224.5587527000002,
            "range": "±65.001 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14485.313; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 20.982413,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.684; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 2265.0889345000005,
            "range": "±3.238 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4832.335; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 24753.0602346,
            "range": "±471.982 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40159.300; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 1.9741138,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.243; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.0922650999999997,
            "range": "±0.028 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=25.362; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 225.02643599999996,
            "range": "±0.947 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3553.120; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 219.4495145,
            "range": "±0.252 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2945.054; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 748.1887035000001,
            "range": "±3.226 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6024.172; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 288.82437500000003,
            "range": "±27.877 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2640.429; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 127.2206048,
            "range": "±0.835 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3756.910; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 260.95679670000004,
            "range": "±29.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2599.254; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 433.36170400000003,
            "range": "±0.697 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6015.242; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 248.77663580000004,
            "range": "±0.376 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5263.983; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 435.84719620000004,
            "range": "±0.827 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6041.879; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 333.89596,
            "range": "±1.493 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5096.485; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 334.4648967,
            "range": "±1.302 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5127.243; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.4124465,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.797; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0810781,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.339; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 14.553538399999999,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.421; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 13.3778828,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.759; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 519.2528734,
            "range": "±1.178 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14929.151; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 645.4455189999999,
            "range": "±1.233 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14509.969; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 555.1639931000001,
            "range": "±1.368 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2282.856; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 106.24459140000002,
            "range": "±0.209 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=697.968; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 550.5253955000001,
            "range": "±1.241 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2269.166; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 45.32977150000001,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=351.175; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 1131.5782382000002,
            "range": "±19.212 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=866.803; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 96.29681000000001,
            "range": "±0.212 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8301.695; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 100.3825956,
            "range": "±0.466 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8131.349; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 104.13734830000001,
            "range": "±0.264 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8192.050; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 7.3881506,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.835; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.908030300000002,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.227; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 7.2409842,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=52.281; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 12.3698274,
            "range": "±0.584 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=116.587; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 16.615945200000002,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=171.667; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 11.737402200000002,
            "range": "±0.277 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.974; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 13.433720099999999,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=155.413; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 11.4452895,
            "range": "±0.210 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.950; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 10.903939900000001,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=173.186; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 11.778400900000001,
            "range": "±0.288 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=117.043; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 13.2316003,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=165.170; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 11.284809,
            "range": "±0.190 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.424; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 10.731300599999999,
            "range": "±0.028 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=150.023; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 10.844311199999998,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=112.186; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 9.2196881,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=150.300; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 19.673043,
            "range": "±0.387 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.327; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 46.1655333,
            "range": "±0.295 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=243.378; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 19.1129219,
            "range": "±0.222 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.486; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 47.464571899999996,
            "range": "±0.847 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=245.185; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 7.377712099999999,
            "range": "±0.044 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.672; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778530151128,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 4123.190862400001,
            "range": "±12.378 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13198.593; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 22406.7592138,
            "range": "±73.619 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95864.307; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 9636.065107499999,
            "range": "±17.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10849.807; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 27977.546824899997,
            "range": "±59.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47424.177; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 4327.7136568999995,
            "range": "±15.247 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13489.078; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 22622.3913538,
            "range": "±64.722 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96237.338; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 8589.654313199999,
            "range": "±17.801 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9907.001; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 22779.78303,
            "range": "±30.804 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42232.399; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 8864.795766500001,
            "range": "±23.208 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10065.234; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 8840.8430645,
            "range": "±19.856 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10082.503; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 5090.186934700001,
            "range": "±18.478 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5677.191; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 12761.582845100002,
            "range": "±83.580 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16957.969; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 279.13175379999996,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=20414.554; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 280.0497059,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=20386.364; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 221.4280208,
            "range": "±0.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14359.992; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 86.7646297,
            "range": "±0.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7826.063; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 89.2216818,
            "range": "±0.252 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7868.461; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 10.2372087,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.240; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 3280.3067787999994,
            "range": "±49.896 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12385.324; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 3254.9217071000003,
            "range": "±92.771 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13717.889; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 3304.0259564000007,
            "range": "±91.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13680.764; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 42.6257015,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.823; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 348.6545833,
            "range": "±1.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=645.306; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 536.2989442,
            "range": "±1.390 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=874.923; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 551.9610261,
            "range": "±1.561 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=892.572; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 31.591519800000004,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.118; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 1163.9499179,
            "range": "±1.795 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1465.949; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 1922.5321252,
            "range": "±1.350 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2472.227; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 2005.962243,
            "range": "±0.676 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2569.443; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 138.0689989,
            "range": "±0.583 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=171.610; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 293.0401971,
            "range": "±0.175 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14448.885; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 220.1568757,
            "range": "±0.193 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14334.296; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 86.88937560000001,
            "range": "±0.135 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7926.421; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 88.1312519,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7721.752; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 8.187909200000002,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=52.815; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 450.46944640000004,
            "range": "±3.597 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4270.793; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 4621.8418212,
            "range": "±5.639 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12421.353; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 4844.401829100001,
            "range": "±9.952 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5072.291; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 249.9854734,
            "range": "±0.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15894.346; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 209.2165877,
            "range": "±0.509 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12202.763; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 208.00312490000002,
            "range": "±0.396 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12302.132; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 12.978862000000001,
            "range": "±0.146 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.378; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 3467.5976748000003,
            "range": "±72.812 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15676.990; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 3227.1245958000004,
            "range": "±94.741 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13652.859; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 3227.0041463999996,
            "range": "±91.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13796.851; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 19.552621699999996,
            "range": "±0.255 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.531; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 2252.9287461,
            "range": "±3.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4771.591; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 3249.6495600999997,
            "range": "±131.365 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=18286.827; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 1.8554779,
            "range": "±0.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=30.038; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 1.6976901999999998,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=27.125; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 220.9547538,
            "range": "±1.672 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3505.555; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 131.4662705,
            "range": "±0.169 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3142.409; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 742.5544335,
            "range": "±2.946 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6045.392; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 182.1340234,
            "range": "±0.185 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2985.077; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 123.84595099999999,
            "range": "±0.662 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3779.524; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 163.2061521,
            "range": "±0.165 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2993.501; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 429.2871901,
            "range": "±0.718 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6089.205; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 244.65478320000003,
            "range": "±0.214 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5137.263; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 430.6541733,
            "range": "±0.562 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6095.830; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 324.2806053,
            "range": "±0.905 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4876.306; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 327.3483466,
            "range": "±0.585 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5000.105; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.2317146,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.900; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0122298,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.631; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 14.6482068,
            "range": "±0.465 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.831; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 13.405954499999998,
            "range": "±0.191 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.161; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 504.08927509999995,
            "range": "±0.339 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15178.977; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 602.0268483,
            "range": "±1.250 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14274.632; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 544.9639349000001,
            "range": "±1.243 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2360.479; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 59.3682783,
            "range": "±0.140 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=731.972; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 539.855851,
            "range": "±0.876 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2342.163; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 41.6670468,
            "range": "±0.153 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=335.721; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 1100.1960157999997,
            "range": "±18.269 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=944.703; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 96.3646097,
            "range": "±0.167 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8046.717; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 96.08291720000001,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8035.642; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 101.50867400000001,
            "range": "±0.213 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8101.312; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 7.054741299999999,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.444; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.888031000000001,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.442; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.7923165999999995,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.955; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 13.6448809,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=119.314; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 29.526468600000005,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=182.675; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 12.736785200000002,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=115.315; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 22.410805399999997,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=169.711; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 12.1614462,
            "range": "±0.131 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=118.200; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 17.7660846,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=169.101; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 10.4662268,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=117.608; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 7.4326753,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=158.519; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 10.372256499999999,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.283; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 7.149960600000002,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.235; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 10.286675800000001,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=116.683; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 6.933417999999999,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.458; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 16.821504800000003,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=150.519; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 32.2382069,
            "range": "±0.113 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=242.792; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 16.0847251,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=151.157; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 24.3060526,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=231.455; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 7.272065900000001,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.750; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778620512419,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 4146.6933943,
            "range": "±8.563 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12030.650; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 25351.528655500002,
            "range": "±47.663 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=120729.456; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 9781.210559,
            "range": "±21.641 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10371.104; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 27909.763612499995,
            "range": "±53.926 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57699.219; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 4246.090472600001,
            "range": "±8.561 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=11961.231; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 25689.220661699997,
            "range": "±35.467 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=123277.752; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 8557.7623537,
            "range": "±18.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9241.568; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 23046.379515599994,
            "range": "±42.652 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55062.497; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 8860.8934224,
            "range": "±21.452 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9507.892; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 8866.991252,
            "range": "±19.313 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9523.743; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 5102.7189729,
            "range": "±15.868 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5524.084; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 12997.0618457,
            "range": "±75.734 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15254.627; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 288.0336453,
            "range": "±0.269 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=19547.512; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 289.9436129,
            "range": "±0.269 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=19082.552; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 221.5355219,
            "range": "±0.163 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16754.183; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 88.2763943,
            "range": "±0.131 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7258.899; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 88.37548999999999,
            "range": "±0.688 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7254.349; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 10.1405021,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.296; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 3246.9451495000003,
            "range": "±49.415 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=11576.027; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 3163.8293887000004,
            "range": "±70.863 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12270.361; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 3185.3547009999997,
            "range": "±74.690 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12294.520; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 42.05375360000001,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.212; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 367.2684966,
            "range": "±0.870 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=641.419; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 544.1629848,
            "range": "±0.685 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=844.556; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 561.0500328,
            "range": "±0.910 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=838.961; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 28.933500400000003,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.424; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 1219.7113983,
            "range": "±3.430 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1493.379; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 1916.9173734,
            "range": "±3.804 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2567.763; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 2002.2938706,
            "range": "±2.813 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2627.545; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 129.04256059999997,
            "range": "±0.621 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=171.457; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 291.19922299999996,
            "range": "±0.252 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16926.232; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 221.3680676,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16891.231; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 88.60353760000001,
            "range": "±0.560 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7258.476; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 89.20482610000002,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7246.473; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 8.337993999999998,
            "range": "±0.346 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.036; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 454.54005229999996,
            "range": "±0.561 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3899.950; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 4630.486183499999,
            "range": "±31.977 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=11925.524; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 4863.1530792,
            "range": "±25.582 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5018.343; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 248.874226,
            "range": "±0.116 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14312.561; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 211.8291432,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9976.240; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 211.8162025,
            "range": "±0.143 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9938.573; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 12.9060415,
            "range": "±0.276 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.070; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 3402.1885211,
            "range": "±74.436 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14832.325; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 3178.8647599,
            "range": "±72.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12280.467; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 3202.2148950999995,
            "range": "±73.262 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12344.012; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 19.6330583,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.744; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 2343.1947791,
            "range": "±5.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3807.253; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 8178.941377,
            "range": "±182.884 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16208.494; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 1.9209206,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.845; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.1082588000000007,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.225; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 225.7635127,
            "range": "±3.605 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3437.108; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 218.57800780000002,
            "range": "±0.270 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2734.803; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 746.103512,
            "range": "±3.000 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5664.502; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 313.1124967,
            "range": "±102.826 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5246.362; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 126.81196849999999,
            "range": "±1.270 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4255.960; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 273.74206430000004,
            "range": "±65.413 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5335.711; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 427.53474059999996,
            "range": "±0.806 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5841.862; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 245.9161685,
            "range": "±0.113 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5997.936; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 430.1756789,
            "range": "±0.923 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5866.536; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 329.4838776,
            "range": "±0.831 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5354.352; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 331.80809889999995,
            "range": "±0.725 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5423.913; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.4239125999999995,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=254.956; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.9087854,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.513; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 14.108536599999999,
            "range": "±0.451 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.180; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 13.5388194,
            "range": "±0.232 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.089; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 510.5713354000001,
            "range": "±1.182 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13421.755; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 630.2452497,
            "range": "±1.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13283.625; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 543.2455764,
            "range": "±0.952 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1570.398; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 61.13818620000001,
            "range": "±0.156 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=948.446; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 537.3201151000001,
            "range": "±3.604 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1552.404; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 42.8411516,
            "range": "±0.182 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=614.334; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 973.9642942,
            "range": "±10.424 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1132.562; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 92.9191153,
            "range": "±0.374 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7136.115; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 96.0582762,
            "range": "±0.438 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7119.236; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 100.8269467,
            "range": "±0.379 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7156.929; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 7.3298000000000005,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.596; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.846926999999999,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.012; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 7.1412462,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.837; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 12.053668300000002,
            "range": "±0.243 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=109.808; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 16.603331700000002,
            "range": "±0.143 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=212.828; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 11.8576819,
            "range": "±0.488 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=225.895; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 13.030272700000001,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=314.960; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 11.264321200000001,
            "range": "±0.215 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=230.612; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 10.9710523,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=320.242; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 10.232472099999999,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=229.938; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 6.5301452,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=313.347; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 10.332808599999998,
            "range": "±0.344 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=224.739; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 6.242182,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=304.229; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 10.540515800000001,
            "range": "±1.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=228.714; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 6.1221183,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=304.067; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 19.7570085,
            "range": "±0.451 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=317.536; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 47.4195874,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=487.118; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 18.8865951,
            "range": "±0.466 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=316.108; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 42.554507,
            "range": "±1.227 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.014; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 7.210041499999998,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.947; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ]
  }
}