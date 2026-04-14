-- numeric ff
SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- better numeric ff
SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id);

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "post_type_id"}}') FROM stackoverflow_posts WHERE body ||| 'javascript';
