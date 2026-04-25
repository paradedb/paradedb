-- numeric ff
SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- better numeric ff
SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id);

-- pdb.agg (under the hood) without GROUP BY
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- pdb.agg without GROUP BY (mvcc disabled)
SELECT pdb.agg('{"value_count": {"field": "post_type_id"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- high-cardinality aggregate scan
SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags;

-- high-cardinality aggregate scan using pdb.agg
SET paradedb.enable_aggregate_custom_scan TO on; SET work_mem = '4GB'; SELECT tags, COUNT(tags), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags;

-- high-cardinality aggregate scan using pdb.agg (mvcc disabled)
SET work_mem = '4GB'; SELECT tags, pdb.agg('{"value_count": {"field": "tags"}}', false) as count, pdb.agg('{"min": {"field": "score"}}', false) as min, pdb.agg('{"max": {"field": "score"}}', false) as max, pdb.agg('{"sum": {"field": "score"}}', false) as sum FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags;
