-- numeric ff
SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- better numeric ff
SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id);

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "post_type_id"}}') FROM stackoverflow_posts WHERE body ||| 'javascript';

-- pdb.agg without GROUP BY (mvcc disabled)
SELECT pdb.agg('{"value_count": {"field": "post_type_id"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- high-cardinality aggregate scan
SELECT tags, COUNT(*), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY tags;

-- high-cardinality aggregate scan using pdb.agg
-- SELECT pdb.agg('{"value_count": {"field": "tags"}}'), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' AND id @@@ pdb.all() GROUP BY tags;
SELECT tags, pdb.agg('{"value_count": {"field": "tags"}}') as count, pdb.agg('{"min": {"field": "score"}}') as min, pdb.agg('{"max": {"field": "score"}}') as max, pdb.agg('{"sum": {"field": "score"}}') as sum FROM stackoverflow_posts WHERE body ||| 'javascript' AND id @@@ pdb.all() GROUP BY tags;

-- high-cardinality aggregate scan using pdb.agg (mvcc disabled)
SELECT pdb.agg('{"value_count": {"field": "tags"}}', false), MIN(score), MAX(score), SUM(score) FROM stackoverflow_posts WHERE body ||| 'javascript' AND id @@@ pdb.all() GROUP BY tags;

