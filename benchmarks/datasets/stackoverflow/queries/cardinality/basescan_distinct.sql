-- numeric ff
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript';

