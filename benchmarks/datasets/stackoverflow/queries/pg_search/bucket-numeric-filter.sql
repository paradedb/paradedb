-- numeric ff
SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id;

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id;

-- pdb.agg with GROUP BY
SELECT post_type_id, pdb.agg('{"value_count": {"field": "post_type_id"}}') FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id;
