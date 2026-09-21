-- numeric ff
SET paradedb.enable_aggregate_custom_scan TO off; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id;
