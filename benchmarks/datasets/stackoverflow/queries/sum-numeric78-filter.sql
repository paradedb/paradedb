-- postgres aggregate over fast fields
SET paradedb.enable_aggregate_custom_scan TO off; SELECT SUM(amount78) FROM stackoverflow_posts WHERE body ||| 'error';

-- datafusion aggregate custom scan, NUMERIC(78,0) decimal-bytes storage
SET paradedb.enable_aggregate_custom_scan TO on; SELECT SUM(amount78) FROM stackoverflow_posts WHERE body ||| 'error';
