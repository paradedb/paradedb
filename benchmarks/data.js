window.BENCHMARK_DATA = {
  "lastUpdate": 1778264323563,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search 'stackoverflow' (100k rows)": [
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
        "date": 1778264246570,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 25.453194900000003,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=139.481; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 25.2775867,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.500; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 67.93736559999999,
            "range": "±0.130 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=147.343; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 67.4246035,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=145.743; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 26.058090899999996,
            "range": "±0.170 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=149.794; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 25.8282927,
            "range": "±0.191 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=145.337; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 45.909996,
            "range": "±0.258 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=164.606; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 45.7194752,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=169.084; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 57.234291899999995,
            "range": "±0.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=182.645; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 57.416362400000004,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=180.196; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 39.6469426,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=268.017; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 540.9148815000001,
            "range": "±2.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=588.033; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.2898577,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=364.798; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.407444900000001,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.425; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.43383,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.071; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.8151482,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.344; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.7326483999999995,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.413; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.439069000000001,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.100; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 23.463735300000003,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=199.001; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 8.3712808,
            "range": "±0.127 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.723; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.6309249,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.094; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 6.1414209,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=30.678; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 93.2565205,
            "range": "±0.889 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=231.916; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 77.2581708,
            "range": "±0.229 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=167.297; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 79.8093636,
            "range": "±0.140 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=170.709; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 11.198152900000002,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.029; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 222.0192348,
            "range": "±0.373 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=359.035; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 267.4961665,
            "range": "±0.454 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=348.736; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 292.644423,
            "range": "±0.251 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=372.388; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 60.08425349999999,
            "range": "±0.361 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=87.636; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.3515546,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=382.208; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.409281699999999,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=399.221; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.7525172,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.596; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.5817488,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.770; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.2806276,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.814; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 8.7916316,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=334.491; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.5330981,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.635; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.690822200000003,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.276; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.4797757,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=506.829; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.9639596,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.581; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.989783200000001,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.219; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.3211171,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.884; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 37.8888327,
            "range": "±0.127 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=378.070; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 8.2740814,
            "range": "±0.165 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.987; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 8.363870200000001,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.386; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 6.0023938,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=30.277; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 46.1264192,
            "range": "±0.231 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=136.509; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 68.6544006,
            "range": "±0.423 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=427.970; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.1264095999999997,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.561; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.0924120999999998,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.425; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 18.150205,
            "range": "±0.197 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=89.678; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 16.918743900000003,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=263.278; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 13.224205500000002,
            "range": "±3.720 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.046; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 19.982636699999997,
            "range": "±0.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=306.439; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 11.429369099999999,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=154.778; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 19.5197302,
            "range": "±0.413 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=308.179; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 29.867735599999996,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=258.518; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 29.166187299999997,
            "range": "±0.248 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=295.375; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.331204399999997,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=271.263; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 24.328702,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=240.152; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.004583200000003,
            "range": "±0.196 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=248.332; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.9157347000000002,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.124; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.2006858,
            "range": "±0.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.657; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 6.0300093,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.452; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 6.006882,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.873; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 34.834229500000006,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=200.177; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.728318000000002,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=591.666; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.123722600000002,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.405; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 16.0196471,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=242.904; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.2839916,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=87.754; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.947571700000001,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=144.256; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 21.4147505,
            "range": "±0.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=248.739; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 7.232736,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.370; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 7.0711538,
            "range": "±0.217 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.755; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.9787869,
            "range": "±0.181 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.237; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.450765899999999,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.569; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.563941699999999,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.710; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.411246799999999,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=38.101; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.741594999999999,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.047; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.2483328,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.773; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.655140399999999,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.402; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.1856663000000003,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.152; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.757542900000002,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.326; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.0130778,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.255; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.588498800000001,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.180; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.9314623,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.840; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.524490100000001,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.341; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.8330974,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.347; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.675783099999999,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.616; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.7095112,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.547; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.621497499999998,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.876; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.2701746,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=108.404; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.6916671,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.223; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 9.1335254,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.651; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.529325699999999,
            "range": "±0.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=37.706; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778264291578,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 24.925676,
            "range": "±0.219 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.693; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 32.1088484,
            "range": "±0.156 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=641.369; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 68.15139160000001,
            "range": "±0.093 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.866; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 68.8525981,
            "range": "±0.816 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=680.203; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 26.3297289,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=159.713; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 33.62984299999999,
            "range": "±0.190 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=700.991; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 54.31463,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=157.749; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 58.289876400000004,
            "range": "±0.467 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=716.431; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 56.719244100000004,
            "range": "±0.148 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=206.585; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 56.95305140000001,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=210.120; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 40.6982469,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.159; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 30.682877400000002,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=364.297; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.3453005,
            "range": "±0.019 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=372.934; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.3783887,
            "range": "±0.020 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.651; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.402481399999999,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=384.422; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.812169200000001,
            "range": "±0.199 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.749; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.8092986,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.673; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.3743504,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.318; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 23.572394300000003,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=229.757; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 8.1928066,
            "range": "±0.134 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.349; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.5307353,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.699; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 6.0409635999999995,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.698; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 89.6973893,
            "range": "±0.136 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=251.329; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 74.7248674,
            "range": "±0.498 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=164.398; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 78.19627390000001,
            "range": "±0.557 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=166.200; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.9109,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.022; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 224.20792179999998,
            "range": "±0.398 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=376.517; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 257.83649649999995,
            "range": "±0.622 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=363.847; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 287.77802890000004,
            "range": "±0.553 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=394.436; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 59.9398211,
            "range": "±0.247 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.080; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.270929799999999,
            "range": "±0.029 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=402.085; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.4689654,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=412.155; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.7896807,
            "range": "±0.159 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.609; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.5046465000000016,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.167; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.1428139999999996,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.387; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 7.982911,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=401.832; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.537938200000001,
            "range": "±0.112 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.202; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.634593000000002,
            "range": "±0.122 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.475; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.305732999999998,
            "range": "±0.025 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=583.505; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.986927799999999,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.022; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.8250656,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.348; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.073385900000001,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.221; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 36.617903,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=452.295; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.4546323999999995,
            "range": "±0.126 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.989; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.2639896,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=55.745; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.0301974000000005,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.479; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 44.499088099999994,
            "range": "±0.223 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=137.364; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 110.1849673,
            "range": "±0.127 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=588.852; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.9771355,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.752; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9414309999999997,
            "range": "±0.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.434; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.344197700000002,
            "range": "±0.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.006; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 16.453240100000002,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=317.662; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 10.5849896,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.910; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 25.958358399999998,
            "range": "±0.734 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=335.542; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 11.175902999999998,
            "range": "±0.210 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=165.958; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 24.6635888,
            "range": "±0.576 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=351.250; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.786732999999998,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=241.470; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 27.9165923,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=301.609; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.846709800000003,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=252.612; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.4737953,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=252.981; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.601756799999997,
            "range": "±0.171 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=261.955; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.695283,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.215; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0277975999999995,
            "range": "±0.117 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.843; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.898757099999999,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.105; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.8614063000000005,
            "range": "±0.034 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.724; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 33.962058999999996,
            "range": "±0.206 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=200.438; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 15.381749099999999,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=524.651; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.087454300000001,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=99.828; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.8337145,
            "range": "±0.080 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=307.068; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.1272923,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=101.378; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.324737599999999,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=151.176; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 19.0473736,
            "range": "±0.165 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=304.416; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.826738799999999,
            "range": "±0.241 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=87.897; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.6390232,
            "range": "±0.234 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.947; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.653550699999999,
            "range": "±0.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.322; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.3389069,
            "range": "±0.139 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.008; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.2503811,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.152; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.4109831999999995,
            "range": "±0.117 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.040; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.544453599999999,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.826; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.1620768000000004,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.713; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.6815842000000005,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.623; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.1221498999999997,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.040; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.437551999999999,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.458; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.8808385000000003,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.570; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.693725000000001,
            "range": "±0.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.581; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.757740000000001,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.058; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.6636207999999995,
            "range": "±0.181 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.989; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.7344994999999996,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.890; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.609711800000001,
            "range": "±0.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.629; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.5734812,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.270; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.544976899999998,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.888; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.019536500000001,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.114; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.4849323,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=98.930; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.8293742,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.960; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3325244000000005,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.275; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ]
  }
}