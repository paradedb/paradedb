-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error';
