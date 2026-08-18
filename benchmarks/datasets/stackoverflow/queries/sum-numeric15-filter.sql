-- postgres aggregate over fast fields
SET paradedb.enable_aggregate_custom_scan TO off; SELECT SUM(amount15) FROM stackoverflow_posts WHERE body ||| 'error';

-- datafusion aggregate custom scan, NUMERIC(15,2) scaled-i64 storage
SET paradedb.enable_aggregate_custom_scan TO on; SELECT SUM(amount15) FROM stackoverflow_posts WHERE body ||| 'error';
