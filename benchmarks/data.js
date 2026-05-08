window.BENCHMARK_DATA = {
  "lastUpdate": 1778270115231,
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
      },
      {
        "commit": {
          "author": {
            "name": "RJ Barman",
            "username": "barbarj",
            "email": "rjhallsted@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "6b16b4d836fa179bb7e1eb681f7709907cc75a2b",
          "message": "fix: Fix actions source logic (#5043)\n\nMissed this in the last PR",
          "timestamp": "2026-05-08T18:02:20Z",
          "url": "https://github.com/paradedb/paradedb/commit/6b16b4d836fa179bb7e1eb681f7709907cc75a2b"
        },
        "date": 1778264384037,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 23.7980734,
            "range": "±0.125 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=162.660; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 29.6029724,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=818.770; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 65.0513843,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=177.658; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 63.7906905,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=904.411; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 24.736836400000005,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=161.879; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 30.2661374,
            "range": "±0.157 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=805.149; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 44.643640000000005,
            "range": "±0.237 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=197.242; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 52.997700099999996,
            "range": "±0.219 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=883.637; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 58.82078909999999,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=171.438; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 59.5603745,
            "range": "±0.127 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=169.246; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 39.243901199999996,
            "range": "±0.117 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=289.244; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 28.5454739,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=429.851; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.1067919,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=362.309; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.166251600000001,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=366.241; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.3877355,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=321.506; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.7300661,
            "range": "±0.290 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.627; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.7265765,
            "range": "±0.265 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.483; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.178549899999999,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.112; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 22.2041815,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=214.442; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 7.860724499999999,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.881; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.2493319,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=62.538; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 5.891168899999999,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.667; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 87.6075709,
            "range": "±0.150 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=249.558; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 74.1213875,
            "range": "±0.395 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=165.392; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 77.3678247,
            "range": "±0.305 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=185.469; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.5331619,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=53.973; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 204.4720637,
            "range": "±0.284 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.666; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 255.67473909999998,
            "range": "±0.328 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=347.427; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 279.2700958,
            "range": "±0.575 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=374.054; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 57.77597850000001,
            "range": "±0.209 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=96.770; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.3939064000000005,
            "range": "±0.122 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=368.468; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.3744449,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=370.368; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.6163731,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.953; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.486201599999999,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.568; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.295438300000001,
            "range": "±0.145 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.357; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 7.908097099999999,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=336.968; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.2837055,
            "range": "±0.100 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.675; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.4051133,
            "range": "±0.226 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.243; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.016069,
            "range": "±0.022 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=557.783; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.822659700000001,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.235; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.9022226,
            "range": "±0.222 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.265; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.1136202,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.108; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 34.9964535,
            "range": "±0.160 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=393.646; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.3597179000000015,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.012; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.2710688,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.698; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.4418903,
            "range": "±1.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.858; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 44.232692099999994,
            "range": "±0.278 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=138.166; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 78.1217169,
            "range": "±1.690 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=517.961; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.9810356000000002,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.537; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9510924999999997,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.969; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.138007500000004,
            "range": "±0.142 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=101.126; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 13.653778999999997,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=393.373; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 11.4779617,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=116.150; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 39.411313899999996,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=485.999; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 10.9546505,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=159.026; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 38.422475500000004,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=469.253; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.3928292,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=240.433; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 28.2811848,
            "range": "±0.177 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=287.084; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.644916000000002,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=261.801; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.779298100000002,
            "range": "±0.244 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=245.231; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.8317074,
            "range": "±0.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=290.390; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.7373895,
            "range": "±0.020 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.929; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.8785841,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.402; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.910083700000001,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.151; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.8948757,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.846; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 34.3248823,
            "range": "±0.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=196.845; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 14.754527200000002,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=578.904; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 11.060639900000002,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.397; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.722018599999998,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=359.379; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.283318699999999,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=107.948; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.444975699999999,
            "range": "±0.099 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=162.510; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 19.1985199,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=355.851; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.5418085,
            "range": "±0.153 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.124; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.586816400000001,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.001; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.4704922,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.857; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.2957075,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.286; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3786422,
            "range": "±0.142 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.706; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.351836199999999,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.790; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.5931728,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.019; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.2933865999999994,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.669; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.5680423,
            "range": "±0.099 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.258; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.2420564,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.651; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.452683200000001,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.241; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.9942892999999997,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.139; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.5143966,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.939; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.8626784,
            "range": "±0.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.828; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.584829400000001,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.922; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.8018954999999997,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.816; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.579515499999999,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.577; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.656335,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.735; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.5725898,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.855; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 9.007942500000002,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=114.263; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.5157238,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=100.774; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.823891,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=108.607; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.362922299999999,
            "range": "±0.073 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.544; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778266109051,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 23.9457156,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.749; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 30.236017800000003,
            "range": "±0.252 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=791.954; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 66.5107825,
            "range": "±0.217 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=148.839; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 64.9826717,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=742.594; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 25.093165300000003,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=151.121; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 31.328867100000004,
            "range": "±0.143 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=715.168; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 52.9234457,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=136.270; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 54.10341079999999,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=775.628; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 55.90440339999999,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=202.581; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 56.3166064,
            "range": "±0.239 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=189.538; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 39.289513799999995,
            "range": "±0.197 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=287.545; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 28.72997,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=396.641; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 4.1377926,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=361.317; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 4.237921999999999,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=374.373; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 4.3630262,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=312.523; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 6.6116665,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.733; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 6.816457400000002,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.467; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.381143,
            "range": "±0.247 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.062; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 22.885919599999998,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=200.300; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 7.990888,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.733; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 8.329025699999999,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.263; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 6.062535000000001,
            "range": "±0.224 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.034; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 87.45423770000002,
            "range": "±0.523 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=221.926; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 76.104718,
            "range": "±0.145 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=169.927; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 79.23717309999999,
            "range": "±0.112 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=178.086; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 10.9059529,
            "range": "±0.048 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.285; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 211.1309973,
            "range": "±0.285 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=369.387; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 256.0915196,
            "range": "±0.221 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=343.446; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 281.7142215,
            "range": "±0.188 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=364.090; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 58.33717850000001,
            "range": "±0.391 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=89.737; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 4.2520322,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=436.110; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 4.3773286,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=444.771; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 6.6743403,
            "range": "±0.058 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.570; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 6.527403600000001,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.762; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.291637700000001,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.938; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 7.9652472,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=389.422; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 12.2735386,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.302; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 18.2951761,
            "range": "±0.105 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.160; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 6.0798496,
            "range": "±0.024 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=514.468; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 6.9098931,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=59.792; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 6.9360651,
            "range": "±0.070 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.664; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.1800142,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.543; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 36.4224659,
            "range": "±0.264 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=373.574; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 7.437584200000002,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.261; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 7.3289907,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=51.728; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.01233,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.355; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 43.69534740000001,
            "range": "±0.298 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=130.842; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 77.1915402,
            "range": "±0.394 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=472.360; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.9740014,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.503; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.9083492,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.112; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 17.034723,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.366; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 13.702534299999996,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=343.299; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 11.215644900000001,
            "range": "±0.037 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.808; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 39.6909573,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=429.459; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 11.0535117,
            "range": "±0.238 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=167.874; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 38.43137349999999,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=435.101; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 28.1087944,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=237.950; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 28.132364900000006,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=342.470; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 31.301987499999996,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=242.976; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 23.413913,
            "range": "±0.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=246.515; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 26.562615899999997,
            "range": "±0.151 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=249.581; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 3.6645823000000006,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.485; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0771429,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.431; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 5.9354773,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.360; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 5.8684653,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.682; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 33.975969,
            "range": "±0.248 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=205.977; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 14.674619400000001,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=572.211; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 10.9855042,
            "range": "±0.245 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=99.504; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 15.839638499999998,
            "range": "±0.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=314.448; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 11.146827199999997,
            "range": "±0.040 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=99.320; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 11.3445431,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.479; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 18.912895900000002,
            "range": "±0.153 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=301.859; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 6.7073644,
            "range": "±0.224 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.623; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 6.5382619,
            "range": "±0.165 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.194; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 6.726090599999999,
            "range": "±0.216 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.361; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.432676000000001,
            "range": "±0.139 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.320; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3339811,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.766; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.354292999999999,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.165; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.531935299999999,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.520; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.339479,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.928; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.622244599999999,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=66.917; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.1891561999999998,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.234; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.5086866,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.222; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 2.9846835,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.656; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.670557099999999,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.620; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 2.8689866,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.880; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.5171363,
            "range": "±0.080 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.424; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 2.7797796999999997,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=63.765; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.474959600000001,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.112; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.6439805,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=65.450; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.353623199999998,
            "range": "±0.063 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=94.312; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 8.962785400000001,
            "range": "±0.017 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.934; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.4269036,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.654; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 8.7971693,
            "range": "±0.016 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.043; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3820583,
            "range": "±0.114 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.922; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
          "id": "c07921a78f3d24cbb0251b31a1150a7db600af5a",
          "message": "chore: Prepare 0.23.4. (#4997)\n\n# Description\nBackport of #4994 to `0.23.x`.\n\n---------\n\nCo-authored-by: Stu Hood <stuhood@paradedb.com>\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-05-06T00:08:47Z",
          "url": "https://github.com/paradedb/paradedb/commit/c07921a78f3d24cbb0251b31a1150a7db600af5a"
        },
        "date": 1778264741095,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 170.4459789,
            "range": "±0.387 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=448.244; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 418.1337348,
            "range": "±0.465 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1808.857; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 536.2075689,
            "range": "±1.272 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=860.671; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 766.8477728,
            "range": "±5.505 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2142.999; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 176.93792309999998,
            "range": "±0.443 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=447.494; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 442.33434480000005,
            "range": "±0.309 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1830.666; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 476.8600622000001,
            "range": "±1.414 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=779.387; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 618.5140996,
            "range": "±0.624 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2039.101; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 490.11387670000005,
            "range": "±0.851 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=792.238; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 490.84584849999993,
            "range": "±0.522 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=823.003; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 305.44019040000006,
            "range": "±0.519 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=660.091; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 301.5814325,
            "range": "±0.777 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1258.184; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 26.1630942,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3178.111; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 25.4330219,
            "range": "±0.190 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3189.093; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.1878271,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1928.518; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.4935133,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=301.856; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.7366914,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=319.780; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.5229703,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.831; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 103.9252385,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=369.132; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 55.899859000000006,
            "range": "±0.113 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=227.907; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 56.928765999999996,
            "range": "±0.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=230.321; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.792150000000001,
            "range": "±0.129 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.161; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 234.67193239999997,
            "range": "±0.216 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=450.150; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 272.6234559,
            "range": "±0.253 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=469.728; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 281.8694749,
            "range": "±0.304 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=482.274; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.866463800000002,
            "range": "±0.080 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.293; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 604.4569592,
            "range": "±0.550 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=822.439; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 916.2996514999999,
            "range": "±0.954 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1133.711; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 964.6242323999999,
            "range": "±3.304 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1194.163; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 91.51979409999998,
            "range": "±0.460 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=125.415; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.3647068,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1936.898; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.387641100000003,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1971.776; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.698568,
            "range": "±0.072 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=306.059; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.5454887,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=308.743; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.3830457,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.303; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 49.0687352,
            "range": "±0.132 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2008.383; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 167.3997541,
            "range": "±1.186 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=436.014; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 236.71890290000002,
            "range": "±1.520 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.708; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 46.7169852,
            "range": "±0.136 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4427.851; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.416872299999998,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=192.888; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 13.470998699999999,
            "range": "±0.312 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=193.059; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.456448399999999,
            "range": "±0.153 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.651; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 134.14381060000002,
            "range": "±0.197 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=936.139; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 55.185948599999996,
            "range": "±0.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=224.808; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 55.1367267,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=227.798; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.7740225,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.230; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 174.9970752,
            "range": "±1.422 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=551.558; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 454.14264430000003,
            "range": "±5.518 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1686.051; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.3834316,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.415; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.3329565000000003,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.632; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 57.3081276,
            "range": "±0.252 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=319.083; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 35.7398749,
            "range": "±0.289 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=493.133; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 85.8505573,
            "range": "±0.663 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=371.578; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 66.80137970000001,
            "range": "±4.242 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=515.888; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 34.8288016,
            "range": "±0.413 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=391.290; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 64.77988660000001,
            "range": "±1.649 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=527.045; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 58.43449280000001,
            "range": "±0.384 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=578.894; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 40.143098099999996,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=573.134; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 61.25224060000001,
            "range": "±0.459 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=592.944; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 54.25687750000001,
            "range": "±0.341 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=566.879; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 57.644950800000004,
            "range": "±0.255 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=555.421; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.261627300000001,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.133; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0883266,
            "range": "±0.130 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.671; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.140007,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.467; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.746390099999999,
            "range": "±0.257 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.718; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 78.67666929999999,
            "range": "±0.444 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=951.396; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 139.1891474,
            "range": "±0.760 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4711.862; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.4302672,
            "range": "±0.181 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=336.487; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.467464099999997,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=364.842; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 35.4915746,
            "range": "±0.172 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=340.415; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 14.941629299999999,
            "range": "±0.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=213.702; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 62.1692762,
            "range": "±1.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=369.843; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 10.906166799999998,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=343.352; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 11.140962900000002,
            "range": "±0.398 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=331.224; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 10.781625100000003,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=319.894; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.4792658,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.319; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.4743021999999995,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=45.820; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.305825499999999,
            "range": "±0.128 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.509; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.7026945000000016,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.680; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 3.9410494,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.400; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.7790762,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.115; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.685075800000001,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.792; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.715711000000001,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.072; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.3881194,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.672; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.7669746,
            "range": "±0.029 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=72.913; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.2449814000000003,
            "range": "±0.023 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.407; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.727756699999999,
            "range": "±0.069 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.615; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.1230088999999994,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.620; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.7146953,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=73.753; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 2.9416075,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.575; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.8014446,
            "range": "±0.101 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=115.781; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 11.1367802,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=128.060; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.831847300000002,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=103.225; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 11.0276308,
            "range": "±0.278 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=130.260; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3371526000000005,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=46.206; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
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
        "date": 1778264773837,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 167.2092568,
            "range": "±0.657 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=456.472; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 168.9806454,
            "range": "±0.839 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=446.753; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 520.0778411,
            "range": "±0.876 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=830.111; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 928.9147493,
            "range": "±2.954 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1285.145; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 176.6350657,
            "range": "±0.816 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=472.376; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 177.19068620000002,
            "range": "±0.443 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=477.117; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 457.4669422999999,
            "range": "±0.779 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=783.114; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 745.7782540000001,
            "range": "±4.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1108.042; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 470.0642328999999,
            "range": "±0.955 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=814.099; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 471.9119331,
            "range": "±0.459 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=791.444; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 310.6169638,
            "range": "±0.139 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=663.295; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 3159.5721504000003,
            "range": "±16.845 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3359.797; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 26.2607597,
            "range": "±0.231 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3223.941; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 25.5314537,
            "range": "±0.238 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3242.897; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.821852900000003,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2019.729; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.5764204,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=297.550; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.884501,
            "range": "±0.273 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=311.434; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.572568599999999,
            "range": "±0.153 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.206; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 104.03235520000001,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=365.837; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 57.602583800000005,
            "range": "±0.156 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=228.166; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 58.965704,
            "range": "±0.221 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=228.768; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.969424999999999,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=32.515; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 226.28371940000002,
            "range": "±0.164 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=440.751; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 285.27048279999997,
            "range": "±0.440 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=485.852; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 294.8473449,
            "range": "±0.285 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=491.758; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 19.0598494,
            "range": "±0.075 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.580; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 606.9151721999999,
            "range": "±0.625 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=812.129; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 917.0415132,
            "range": "±0.802 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1162.016; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 968.8214847000002,
            "range": "±2.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1224.829; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 92.62567720000001,
            "range": "±0.382 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=119.114; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.7742255,
            "range": "±0.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2064.014; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.8116881,
            "range": "±0.144 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2011.753; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.6764435,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=299.112; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.556467,
            "range": "±0.126 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=315.738; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.3565561,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.136; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 49.569986899999996,
            "range": "±0.116 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2040.727; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 167.6202669,
            "range": "±0.764 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=472.132; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 238.5675667,
            "range": "±0.915 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=279.007; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 47.500643499999995,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4674.563; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 14.3536145,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=171.787; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 14.110457100000001,
            "range": "±0.097 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=170.459; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.3725025,
            "range": "±0.086 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.391; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 138.26664179999997,
            "range": "±0.256 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=949.713; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 57.1684218,
            "range": "±0.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=229.557; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 57.514672099999984,
            "range": "±0.188 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=230.083; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 6.550315100000001,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.043; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 176.45404450000004,
            "range": "±0.702 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=519.691; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 1224.7234388,
            "range": "±18.533 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1857.083; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.3836376,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=93.550; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.4436059,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.131; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 58.290649699999996,
            "range": "±0.538 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=310.371; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 33.4472499,
            "range": "±0.193 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=474.602; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 85.56964249999999,
            "range": "±0.627 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=371.105; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 53.226666,
            "range": "±0.304 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=486.176; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 34.2948915,
            "range": "±0.392 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=403.034; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 54.1743215,
            "range": "±1.928 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=491.198; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 49.0239685,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=542.510; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 38.82629430000001,
            "range": "±0.087 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=570.375; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 50.420369199999996,
            "range": "±0.096 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=552.414; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 54.8561775,
            "range": "±0.352 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=550.315; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 56.3835773,
            "range": "±0.290 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=506.770; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.4299252,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.449; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.2386726999999995,
            "range": "±0.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33.713; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.553922599999998,
            "range": "±0.161 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.311; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.882811199999999,
            "range": "±0.316 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.339; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 79.3147199,
            "range": "±0.349 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=883.959; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 143.35058429999998,
            "range": "±0.308 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4737.685; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.627075500000004,
            "range": "±0.203 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=343.950; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 21.714196100000002,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=304.564; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 35.8408234,
            "range": "±0.172 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=348.691; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 15.6295089,
            "range": "±0.188 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=204.797; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 73.3953375,
            "range": "±1.082 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=306.141; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 12.2283522,
            "range": "±0.428 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=313.067; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 12.359816599999998,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=334.356; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 11.900839099999999,
            "range": "±0.448 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=335.297; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.4007719000000005,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.819; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.4270065,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.242; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.3567434,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.174; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.7063605,
            "range": "±0.056 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=71.337; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.0288032000000005,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.186; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.7083202,
            "range": "±0.090 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.607; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 3.7852588000000003,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.726; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.6724401,
            "range": "±0.078 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.892; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.4771538,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.005; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.6979351,
            "range": "±0.047 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.329; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.5146651999999996,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.173; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.809638499999998,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=69.898; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.3642467,
            "range": "±0.005 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=76.595; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.7498272,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.181; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.1404564,
            "range": "±0.021 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.834; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 11.062135399999999,
            "range": "±0.159 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=101.342; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 10.6882789,
            "range": "±0.228 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=129.819; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 11.1349999,
            "range": "±0.242 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=100.062; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 11.219776099999999,
            "range": "±0.183 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=124.818; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.411748,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=39.936; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "RJ Barman",
            "username": "barbarj",
            "email": "rjhallsted@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "6b16b4d836fa179bb7e1eb681f7709907cc75a2b",
          "message": "fix: Fix actions source logic (#5043)\n\nMissed this in the last PR",
          "timestamp": "2026-05-08T18:02:20Z",
          "url": "https://github.com/paradedb/paradedb/commit/6b16b4d836fa179bb7e1eb681f7709907cc75a2b"
        },
        "date": 1778264824368,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 164.44752050000002,
            "range": "±0.681 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=452.762; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 402.0762735,
            "range": "±0.344 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1830.693; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 510.0268070000001,
            "range": "±1.582 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=798.758; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 762.685463,
            "range": "±1.430 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2205.754; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 171.52714129999998,
            "range": "±0.593 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=451.809; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 426.9550693,
            "range": "±0.264 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1853.518; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 455.1743401,
            "range": "±1.266 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=765.957; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 596.5777588999999,
            "range": "±0.272 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2058.209; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 467.18546719999995,
            "range": "±0.630 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=776.817; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 464.78997319999996,
            "range": "±0.603 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=809.461; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 305.3865397,
            "range": "±0.256 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=649.744; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 286.23508990000005,
            "range": "±0.436 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1235.377; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 25.3403424,
            "range": "±0.115 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3198.736; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 25.435237999999995,
            "range": "±0.127 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3207.613; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.3815624,
            "range": "±0.055 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2015.768; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.5438025,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=268.437; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.687812300000001,
            "range": "±0.053 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=270.016; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.5298356,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.041; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 99.19941639999999,
            "range": "±0.080 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=399.874; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 54.08960939999999,
            "range": "±0.152 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=252.042; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 55.539257899999996,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=271.816; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.714124399999998,
            "range": "±0.035 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.936; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 215.0428977,
            "range": "±0.352 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=466.876; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 283.9368599,
            "range": "±0.492 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=469.056; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 291.0366499,
            "range": "±0.577 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=488.233; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.066966600000004,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=64.281; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 565.3466592999999,
            "range": "±3.381 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=806.025; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 910.1129655000001,
            "range": "±0.928 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1148.084; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 957.7095482000001,
            "range": "±0.993 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1192.899; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 87.92395939999999,
            "range": "±0.451 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=125.478; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.7076755,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1961.187; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 21.943025300000002,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1920.242; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.453276800000001,
            "range": "±0.092 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=283.667; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.545331200000001,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=286.597; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.274554800000001,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.964; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 48.58460060000001,
            "range": "±0.215 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2043.127; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 169.21848500000002,
            "range": "±0.682 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=426.704; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 238.48499,
            "range": "±1.410 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=284.632; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 44.7639037,
            "range": "±0.157 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4484.057; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 12.9157681,
            "range": "±0.061 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=199.580; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 12.885633099999998,
            "range": "±0.054 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=197.992; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.3540218,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.243; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 130.70888180000003,
            "range": "±0.391 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=889.770; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 53.46122270000001,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=267.316; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 53.2903519,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=250.889; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.6382904,
            "range": "±0.032 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.076; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 172.7973345,
            "range": "±0.763 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=526.056; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 267.5918845,
            "range": "±7.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1195.408; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.2578738,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=94.448; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.2445325000000005,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=92.635; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 56.8084466,
            "range": "±0.517 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=336.823; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 69.03030250000002,
            "range": "±5.405 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=584.575; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 83.4306435,
            "range": "±1.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=360.467; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 127.79792769999999,
            "range": "±0.205 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=907.348; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.159168,
            "range": "±0.592 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=380.214; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 126.92333550000004,
            "range": "±0.214 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=917.321; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 47.584631800000004,
            "range": "±0.137 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=556.483; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 37.7787673,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=643.156; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 50.5018723,
            "range": "±0.200 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=567.579; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 52.59433849999999,
            "range": "±0.291 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=613.062; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 55.551799,
            "range": "±0.502 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=614.630; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.2765098,
            "range": "±0.018 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=77.367; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.9411071,
            "range": "±0.106 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.514; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.470507399999999,
            "range": "±0.068 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.551; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.790465299999999,
            "range": "±0.260 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.990; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 75.9835788,
            "range": "±0.406 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1041.732; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 137.4180648,
            "range": "±0.271 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4739.406; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 35.5745635,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=345.204; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.227551999999996,
            "range": "±0.060 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=425.799; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 35.6804016,
            "range": "±0.184 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=342.745; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 14.755341899999996,
            "range": "±0.228 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=232.453; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 65.29783189999999,
            "range": "±0.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=407.884; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 11.124245700000001,
            "range": "±0.409 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=275.972; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 11.0046398,
            "range": "±0.321 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=311.924; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 11.5248257,
            "range": "±0.579 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=368.517; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.3910478,
            "range": "±0.088 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.939; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.256288,
            "range": "±0.059 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.552; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.2451093,
            "range": "±0.031 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=50.412; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.631575,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.055; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.6808367,
            "range": "±0.012 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=86.251; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.639109899999999,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.100; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 4.270543600000001,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=86.267; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.680105299999999,
            "range": "±0.135 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.748; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.8421058,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=91.545; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.666589,
            "range": "±0.076 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.735; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.7198543,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=83.764; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.691519500000001,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=79.855; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.4917004,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.336; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.5320567,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=80.527; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.2337745999999994,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.950; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.737741500000002,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.621; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 10.2573955,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=140.764; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.959712999999999,
            "range": "±0.189 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=111.166; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 10.4198053,
            "range": "±0.026 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=131.419; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.288518,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=49.566; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "lockerman@paradedb.com",
            "name": "JLockerman",
            "username": "JLockerman"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9a827ea3a0452e482e852785e743deb2f4630222",
          "message": "fix: Issues with text casts and memory layout for tokenizer types (#4900)\n\n# Ticket(s) Closed\n\n- fixes https://github.com/paradedb/paradedb/issues/5033\n\n## What\n\nChanges the tokenizer and alias types to function as regular SQL types\n(writable to disk, reallocatable in memory contexts etc).\n\n## Why\n\nWhen used incorrectly (eg. within a non-optimized function call) the\nprevious versions would access freed memory.\n\n## How\n\nThe tokenizer format is changed from `(header, magic_num, Oid, padding\nDatum)` to `(header, magic_num, metadata, padding, Oid, data_bytes)`\nwhere the `data_bytes` are the bytes from the original value (the\n`Datum` for by-value types, and the bytes pointed-at by the `Datum` for\nby-ref types). This lets us create a new `Datum` for that type (pointing\nat the inner bytes if needed).\n\nNOTE: Since the old version of the type was storing `Datum`s directly,\nany values stored to disk with the old code is broken unless they were\nin text format (the others store dangling pointers). In the updated\nversion such values will be output as meaningless text instead.\n\n## Tests\n\n- in\n`pg_search/tests/pg_regress/sql/tokenizer-types-inline-tokenization.sql`\n\n---------\n\nCo-authored-by: Mithun Chicklore Yogendra <mithun.cy@gmail.com>",
          "timestamp": "2026-05-08T14:32:32-04:00",
          "tree_id": "9cf77ffd18186494bb164cc15f9f703749357d03",
          "url": "https://github.com/paradedb/paradedb/commit/9a827ea3a0452e482e852785e743deb2f4630222"
        },
        "date": 1778266580723,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 165.6350713,
            "range": "±0.459 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=448.558; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 418.6814963999999,
            "range": "±0.353 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1885.633; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 511.2244547,
            "range": "±0.700 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=828.643; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 771.6943193,
            "range": "±0.607 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2264.964; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 172.4125871,
            "range": "±0.668 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=448.030; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 440.54956490000006,
            "range": "±0.263 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1945.906; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 451.01723840000005,
            "range": "±1.258 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=741.912; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 606.6997152,
            "range": "±0.438 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2100.568; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 463.66030259999997,
            "range": "±0.498 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=773.194; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 463.0277881,
            "range": "±0.503 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=802.204; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 304.3044752,
            "range": "±0.257 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=652.524; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 287.28961790000005,
            "range": "±0.597 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1198.063; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 25.7323941,
            "range": "±0.107 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3252.563; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 26.1583015,
            "range": "±0.248 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3238.421; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 22.1654727,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2044.347; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 9.4412033,
            "range": "±0.043 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=330.011; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 9.6660171,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=289.323; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 6.495841799999999,
            "range": "±0.046 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.122; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 102.47060119999999,
            "range": "±0.084 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=370.003; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 55.6593328,
            "range": "±0.166 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=232.029; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 56.46841499999999,
            "range": "±0.104 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=232.762; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 7.679013500000001,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.218; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 215.3443953,
            "range": "±0.242 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=451.109; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 282.80714389999997,
            "range": "±0.384 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=475.803; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 292.17772560000003,
            "range": "±0.410 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=499.537; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 18.415094599999996,
            "range": "±0.045 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.303; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 577.1950800000001,
            "range": "±0.633 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=806.103; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 897.5914782,
            "range": "±0.545 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1133.326; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 946.5418681000001,
            "range": "±1.870 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1185.680; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 89.147988,
            "range": "±0.182 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=121.299; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 21.6804285,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2121.663; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 22.0280652,
            "range": "±0.190 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2010.506; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 9.5095201,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=268.188; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 9.4325599,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=266.711; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 6.387381199999999,
            "range": "±0.139 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=40.756; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 48.9023026,
            "range": "±0.133 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2140.871; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 168.4574586,
            "range": "±0.563 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=469.583; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 236.9635774,
            "range": "±1.475 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=281.746; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 45.6921083,
            "range": "±0.149 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4415.440; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 13.900709299999999,
            "range": "±0.085 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=185.270; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 13.676656300000001,
            "range": "±0.074 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=184.820; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 6.385177799999999,
            "range": "±0.091 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.814; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 133.12883970000001,
            "range": "±0.337 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=884.150; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 54.324342200000004,
            "range": "±0.108 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=235.441; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 54.531977299999994,
            "range": "±0.105 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=233.712; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 5.729149799999999,
            "range": "±0.071 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=36.394; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 170.7587246,
            "range": "±0.470 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=506.129; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 264.0633627,
            "range": "±0.316 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1034.845; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 3.237791299999999,
            "range": "±0.009 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=97.091; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 3.2299272,
            "range": "±0.010 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=95.063; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 55.9094959,
            "range": "±0.764 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=308.264; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 73.2489734,
            "range": "±0.842 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=559.027; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 83.1164184,
            "range": "±0.727 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=376.360; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 128.1148705,
            "range": "±0.215 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=853.448; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 33.12175,
            "range": "±0.424 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=358.920; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 127.0432251,
            "range": "±0.415 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=860.761; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 48.2294439,
            "range": "±0.330 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=558.798; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 37.65335470000001,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=615.337; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 51.3211783,
            "range": "±0.201 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=590.961; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 53.01315989999999,
            "range": "±0.352 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=554.514; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 56.4070418,
            "range": "±0.445 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=503.274; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.293316299999999,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=78.837; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 5.9644578,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=34.329; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 8.4852156,
            "range": "±0.124 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.741; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 7.867340899999999,
            "range": "±0.523 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=41.038; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 76.40561730000002,
            "range": "±0.419 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=917.057; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 136.28835009999997,
            "range": "±0.260 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4676.229; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 34.973771299999996,
            "range": "±0.505 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=317.926; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 20.1969965,
            "range": "±0.052 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=382.904; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 34.88574620000001,
            "range": "±0.481 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=344.590; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 14.9085521,
            "range": "±0.311 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=210.523; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 54.9039983,
            "range": "±0.102 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=385.502; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 11.239125900000001,
            "range": "±0.323 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=322.156; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 10.9590519,
            "range": "±0.027 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=337.588; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 11.3511115,
            "range": "±0.405 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=319.586; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 6.425278199999999,
            "range": "±0.121 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.986; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.3167290000000005,
            "range": "±0.064 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=42.794; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 6.335005400000001,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=43.300; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 7.6772216,
            "range": "±0.033 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.674; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 4.7221364999999995,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=84.668; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 7.698552499999998,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.029; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 4.2800241,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=85.340; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 7.7361181000000006,
            "range": "±0.094 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.810; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 3.8592474,
            "range": "±0.008 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=85.278; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 7.740400800000001,
            "range": "±0.095 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=75.574; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 3.7765239999999993,
            "range": "±0.015 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=82.596; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 7.6645986,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.278; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 3.5200477,
            "range": "±0.006 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.850; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 7.7563986,
            "range": "±0.109 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=74.080; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 3.2489065000000004,
            "range": "±0.014 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=81.792; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 10.7260344,
            "range": "±0.039 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=105.784; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 10.2390157,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=134.503; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 10.900646,
            "range": "±0.146 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=103.756; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 10.3258387,
            "range": "±0.051 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=130.428; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 6.3724359,
            "range": "±0.083 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.241; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ],
    "pg_search 'stackoverflow' (20m rows)": [
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
        "date": 1778270082931,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "aggregate_join_count",
            "value": 4244.3461201,
            "range": "±14.422 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13045.511; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_count - alternative 1",
            "value": 5061.6476639,
            "range": "±19.340 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13615.590; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_groupby",
            "value": 9791.3669098,
            "range": "±27.305 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10947.206; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_groupby - alternative 1",
            "value": 26801.310331099998,
            "range": "±99.314 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=27615.843; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*), SUM(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY p.title"
          },
          {
            "name": "aggregate_join_multi",
            "value": 4352.3892488,
            "range": "±18.539 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13363.463; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_multi - alternative 1",
            "value": 5092.2210706,
            "range": "±43.120 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13576.830; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*), MIN(c.score), MAX(c.score) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code'"
          },
          {
            "name": "aggregate_join_topk_count",
            "value": 8671.5983711,
            "range": "±21.456 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=9947.621; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_join_topk_count - alternative 1",
            "value": 22380.209302599997,
            "range": "±80.362 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=23088.310; query=SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort",
            "value": 8963.915502,
            "range": "±28.334 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10166.811; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_sort - alternative 1",
            "value": 8914.522670900002,
            "range": "±34.336 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=10152.001; query=SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, MAX(c.creation_date) as last_activity FROM stackoverflow_posts p JOIN comments c ON p.id = c.post_id WHERE p.body ||| 'code' GROUP BY p.id, p.title ORDER BY last_activity DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count",
            "value": 5121.0454638,
            "range": "±19.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5739.538; query=SET paradedb.enable_aggregate_custom_scan TO off; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "aggregate_topk_count - alternative 1",
            "value": 8047.6971512,
            "range": "±15.515 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=18924.949; query=SET work_mem TO '8GB'; SET paradedb.enable_aggregate_custom_scan TO on; SELECT p.title, COUNT(*) FROM stackoverflow_posts p WHERE p.body ||| 'code' GROUP BY p.title ORDER BY COUNT(*) DESC LIMIT 10"
          },
          {
            "name": "bucket-expr-filter",
            "value": 292.0641061,
            "range": "±0.235 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=20241.736; query=SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-expr-filter - alternative 1",
            "value": 292.67068140000003,
            "range": "±0.329 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=20230.890; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT date_trunc('year', creation_date) as year, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY year ORDER BY year"
          },
          {
            "name": "bucket-numeric-filter",
            "value": 223.57830589999998,
            "range": "±0.178 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14117.885; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 1",
            "value": 89.54941090000001,
            "range": "±0.174 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8042.760; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 2",
            "value": 92.54734590000001,
            "range": "±0.162 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7980.374; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-filter - alternative 3",
            "value": 10.2750031,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=48.942; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter",
            "value": 3277.2741284000003,
            "range": "±46.682 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12648.786; query=SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id ORDER BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 1",
            "value": 3203.875225,
            "range": "±63.360 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13808.795; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 2",
            "value": 3227.200451,
            "range": "±67.708 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13793.579; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-numeric-nofilter - alternative 3",
            "value": 43.2268513,
            "range": "±0.066 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=70.792; query=SELECT post_type_id, pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id"
          },
          {
            "name": "bucket-string-filter",
            "value": 365.81229700000006,
            "range": "±0.329 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=680.739; query=SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-filter - alternative 1",
            "value": 543.4968583000002,
            "range": "±0.747 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=875.133; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 2",
            "value": 559.8101353,
            "range": "±0.990 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=886.124; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-filter - alternative 3",
            "value": 29.1021504,
            "range": "±0.065 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=68.009; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE name ||| 'Question' GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter",
            "value": 1220.4264973,
            "range": "±1.612 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=1570.563; query=SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name ORDER BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 1",
            "value": 1943.0852889000003,
            "range": "±0.631 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2521.944; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 2",
            "value": 2015.7010350999997,
            "range": "±3.822 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2589.453; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(name) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "bucket-string-nofilter - alternative 3",
            "value": 129.3766359,
            "range": "±0.577 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=159.945; query=SELECT name, pdb.agg('{\"value_count\": {\"field\": \"name\"}}', false) FROM badges WHERE id @@@ pdb.all() GROUP BY name"
          },
          {
            "name": "cardinality",
            "value": 290.37637700000005,
            "range": "±0.119 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14365.604; query=SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 1",
            "value": 220.87167569999997,
            "range": "±0.278 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14307.027; query=SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 2",
            "value": 89.0800089,
            "range": "±0.514 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7991.182; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id)"
          },
          {
            "name": "cardinality - alternative 3",
            "value": 90.17419329999998,
            "range": "±0.194 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8122.864; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 4",
            "value": 8.3402352,
            "range": "±0.077 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=47.030; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"post_type_id\"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript'"
          },
          {
            "name": "cardinality - alternative 5",
            "value": 460.00039749999996,
            "range": "±0.514 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4469.615; query=SET work_mem TO '4GB'; SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 6",
            "value": 4600.7567163,
            "range": "±4.177 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12762.145; query=SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "cardinality - alternative 7",
            "value": 4757.408610200001,
            "range": "±8.306 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4988.776; query=SET work_mem = '4GB'; SELECT tags, pdb.agg('{\"value_count\": {\"field\": \"tags\"}}', false) as count, pdb.agg('{\"min\": {\"field\": \"score\"}}', false) as min, pdb.agg('{\"max\": {\"field\": \"score\"}}', false) as max, pdb.agg('{\"sum\": {\"field\": \"score\"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags"
          },
          {
            "name": "count-filter",
            "value": 250.309406,
            "range": "±0.191 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=16293.704; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 1",
            "value": 218.54796890000003,
            "range": "±0.173 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12350.480; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 2",
            "value": 217.90137399999998,
            "range": "±0.179 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=12234.196; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-filter - alternative 3",
            "value": 12.846949500000003,
            "range": "±0.103 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=67.659; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE body ||| 'error'"
          },
          {
            "name": "count-nofilter",
            "value": 3418.3702615,
            "range": "±57.474 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15557.483; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 1",
            "value": 3187.3374517,
            "range": "±69.498 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13833.146; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 2",
            "value": 3177.5434118000003,
            "range": "±64.756 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=13822.973; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(ctid) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "count-nofilter - alternative 3",
            "value": 20.9250444,
            "range": "±0.057 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=58.338; query=SELECT pdb.agg('{\"value_count\": {\"field\": \"ctid\"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all()"
          },
          {
            "name": "distinct_parent_sort",
            "value": 2249.3305319999995,
            "range": "±3.737 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4772.036; query=SET paradedb.enable_join_custom_scan TO off; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "distinct_parent_sort - alternative 1",
            "value": 24364.4811132,
            "range": "±285.962 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=33835.503; query=SET work_mem TO '8GB'; SET paradedb.enable_join_custom_scan TO on; SELECT DISTINCT u.id, u.display_name, u.about_me FROM users u JOIN stackoverflow_posts p ON u.id = p.owner_user_id JOIN comments c ON p.id = c.post_id WHERE c.score > 0 AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY u.display_name ASC LIMIT 50"
          },
          {
            "name": "filtered-highcard",
            "value": 2.3102081999999995,
            "range": "±0.011 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=44.467; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND creation_date >= '2012-01-01T00:00:00Z' LIMIT 10"
          },
          {
            "name": "filtered-lowcard",
            "value": 2.0709551,
            "range": "±0.007 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=30.174; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' AND post_type_id < 3 LIMIT 10"
          },
          {
            "name": "foreign_filter_local_sort",
            "value": 221.76898409999998,
            "range": "±0.624 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3518.563; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "foreign_filter_local_sort - alternative 1",
            "value": 220.5252637,
            "range": "±0.287 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2914.131; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date, u.display_name as user_display_name, u.about_me as user_about_me FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE u.id @@@ pdb.all() AND u.reputation > 100 AND p.title ||| 'error' ORDER BY p.creation_date DESC LIMIT 20"
          },
          {
            "name": "hierarchical_content-no-scores-large",
            "value": 742.1009676,
            "range": "±1.511 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5966.509; query=SET paradedb.enable_join_custom_scan TO off; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-large - alternative 1",
            "value": 253.60415239999998,
            "range": "±24.814 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2590.940; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT * FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small",
            "value": 122.7201104,
            "range": "±0.642 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=3748.581; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-no-scores-small - alternative 1",
            "value": 262.1101151,
            "range": "±24.363 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2526.864; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' LIMIT 5"
          },
          {
            "name": "hierarchical_content-scores-large",
            "value": 430.5106823,
            "range": "±0.806 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6066.199; query=SET paradedb.enable_join_custom_scan TO off; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 1",
            "value": 252.81639959999998,
            "range": "±0.848 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=5550.048; query=WITH topk AS ( SELECT users.id AS user_id, stackoverflow_posts.id AS post_id, comments.id AS comment_id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000 ) SELECT u.*, p.*, c.*, topk.pdb_score FROM topk JOIN users u ON topk.user_id = u.id JOIN stackoverflow_posts p ON topk.post_id = p.id JOIN comments c ON topk.comment_id = c.id WHERE topk.user_id = u.id AND topk.post_id = p.id AND topk.comment_id = c.id ORDER BY topk.pdb_score DESC"
          },
          {
            "name": "hierarchical_content-scores-large - alternative 2",
            "value": 433.5074916,
            "range": "±0.549 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=6085.443; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT *, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.about_me ||| 'java' AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small",
            "value": 326.6969117,
            "range": "±0.924 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4737.063; query=SET paradedb.enable_join_custom_scan TO off; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "hierarchical_content-scores-small - alternative 1",
            "value": 327.7227567,
            "range": "±0.722 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=4930.010; query=SET work_mem TO '4GB'; SET paradedb.enable_join_custom_scan TO on; SELECT users.id, stackoverflow_posts.id, comments.id, pdb.score(users.id) + pdb.score(stackoverflow_posts.id) + pdb.score(comments.id) AS pdb_score FROM users JOIN stackoverflow_posts ON users.id = stackoverflow_posts.owner_user_id JOIN comments ON comments.post_id = stackoverflow_posts.id WHERE users.id @@@ pdb.all() AND users.reputation > 100 AND stackoverflow_posts.title ||| 'error' AND comments.text ||| 'question' ORDER BY pdb_score DESC LIMIT 1000"
          },
          {
            "name": "highlighting",
            "value": 4.3557741,
            "range": "±0.013 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=106.015; query=SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' LIMIT 10"
          },
          {
            "name": "paging-string-max",
            "value": 6.0331837,
            "range": "±0.098 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=35.087; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-max') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-median",
            "value": 14.6031737,
            "range": "±0.767 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=60.022; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-median') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "paging-string-min",
            "value": 13.498621500000002,
            "range": "±0.050 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.608; query=SELECT * FROM comments WHERE id @@@ pdb.all() AND user_display_name >= (SELECT value FROM stackoverflow_schema_metadata WHERE name = 'comments-user-display-name-min') ORDER BY user_display_name LIMIT 100"
          },
          {
            "name": "permissioned_search",
            "value": 514.4177030000001,
            "range": "±1.002 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=15018.354; query=SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, pdb.score(p.id) as relevance FROM stackoverflow_posts p JOIN users u ON p.owner_user_id = u.id WHERE p.title ||| 'how using get create' AND u.id @@@ pdb.all() AND u.reputation > 100 ORDER BY relevance DESC LIMIT 10"
          },
          {
            "name": "regex-and-heap",
            "value": 621.0718858,
            "range": "±0.855 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=14348.270; query=SELECT COUNT(*) FROM stackoverflow_posts WHERE (tags @@@ pdb.regex('java.*') AND tags ILIKE '%script%')"
          },
          {
            "name": "semi_join_filter",
            "value": 546.1821941000001,
            "range": "±0.813 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2372.099; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 1",
            "value": 104.89879679999999,
            "range": "±0.111 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=683.726; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 2",
            "value": 531.0333287999999,
            "range": "±0.526 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=2373.271; query=SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 3",
            "value": 45.293686,
            "range": "±0.292 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=335.138; query=SET paradedb.enable_columnar_sort TO off; SET paradedb.enable_join_custom_scan TO off; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id @@@ pdb.term_set(( SELECT array_agg(id) FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' )) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "semi_join_filter - alternative 4",
            "value": 1114.2319518,
            "range": "±0.997 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=864.359; query=SET work_mem TO '4GB'; SET paradedb.enable_columnar_sort TO on; SET paradedb.enable_join_custom_scan TO on; SELECT p.id, p.title, p.creation_date FROM stackoverflow_posts p WHERE p.owner_user_id IN ( SELECT id FROM users WHERE about_me ||| 'java' AND display_name ||| 'David John Alex' ) ORDER BY p.title ASC LIMIT 25"
          },
          {
            "name": "top_k-agg-avg",
            "value": 94.3640641,
            "range": "±0.199 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=8064.353; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-bucket-string",
            "value": 97.2835521,
            "range": "±0.274 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7929.341; query=SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, title, tags, post_type_id, creation_date, COUNT(owner_display_name) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-agg-count",
            "value": 101.5864544,
            "range": "±0.347 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=7970.876; query=SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10"
          },
          {
            "name": "top_k-compound",
            "value": 7.4671758,
            "range": "±0.089 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=56.357; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY score, creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-highcard",
            "value": 6.9635261,
            "range": "±0.079 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.979; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY creation_date LIMIT 10"
          },
          {
            "name": "top_k-numeric-lowcard",
            "value": 7.268826299999999,
            "range": "±0.042 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=54.657; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY post_type_id LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity",
            "value": 11.903493999999998,
            "range": "±0.116 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=117.721; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-high-selectivity - alternative 1",
            "value": 16.484206099999998,
            "range": "±0.041 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=176.448; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity",
            "value": 11.6521253,
            "range": "±0.416 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=115.193; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc-medium-selectivity - alternative 1",
            "value": 13.364622700000002,
            "range": "±0.036 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=160.186; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc",
            "value": 11.322978,
            "range": "±0.188 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=114.186; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-asc - alternative 1",
            "value": 10.934548900000001,
            "range": "±0.038 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=155.923; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity",
            "value": 11.679937599999999,
            "range": "±0.197 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=114.672; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-high-selectivity - alternative 1",
            "value": 12.932168200000001,
            "range": "±0.049 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=169.699; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'help' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity",
            "value": 11.294335700000001,
            "range": "±0.286 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.615; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc-medium-selectivity - alternative 1",
            "value": 10.8162115,
            "range": "±0.025 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=153.821; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc",
            "value": 11.118986,
            "range": "±0.327 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=113.827; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-desc - alternative 1",
            "value": 9.1729911,
            "range": "±0.062 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=159.245; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc",
            "value": 18.884713400000003,
            "range": "±0.304 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=152.168; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-asc - alternative 1",
            "value": 44.419939299999996,
            "range": "±5.203 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=261.906; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc",
            "value": 18.9171895,
            "range": "±0.246 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=150.982; query=SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-score-multi-term-desc - alternative 1",
            "value": 42.61569300000001,
            "range": "±0.033 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=257.551; query=SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10"
          },
          {
            "name": "top_k-string",
            "value": 7.484212000000001,
            "range": "±0.067 ms",
            "unit": "mean ms",
            "extra": "cold_query_ms=57.569; query=SELECT * FROM stackoverflow_posts WHERE body ||| 'javascript' AND tags ||| 'python' ORDER BY tags LIMIT 10"
          }
        ]
      }
    ]
  }
}