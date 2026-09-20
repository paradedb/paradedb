-- postgres aggregate over fast fields
SET paradedb.enable_aggregate_custom_scan TO off; SELECT SUM(amount15) FROM stackoverflow_posts WHERE body ||| 'error';

