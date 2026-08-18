-- postgres aggregate over fast fields
SET paradedb.enable_aggregate_custom_scan TO off; SELECT SUM(view_count) FROM stackoverflow_posts WHERE body ||| 'error';

-- tantivy aggregate custom scan (reference for the NUMERIC variants)
SET paradedb.enable_aggregate_custom_scan TO on; SELECT SUM(view_count) FROM stackoverflow_posts WHERE body ||| 'error';
