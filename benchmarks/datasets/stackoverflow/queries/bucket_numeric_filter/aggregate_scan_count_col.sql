-- pdb.agg (under the hood) with GROUP BY
SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id;
