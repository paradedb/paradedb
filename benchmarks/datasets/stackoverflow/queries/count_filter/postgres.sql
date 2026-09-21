-- numeric fast field
SET paradedb.enable_aggregate_custom_scan TO off; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error';
