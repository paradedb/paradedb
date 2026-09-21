-- postgres aggregate over fast fields
SET paradedb.enable_aggregate_custom_scan TO off; SELECT SUM(amount78) FROM stackoverflow_posts WHERE body ||| 'error';
