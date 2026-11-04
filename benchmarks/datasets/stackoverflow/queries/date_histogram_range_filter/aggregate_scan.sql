-- Tantivy date_histogram aggregate custom scan with a date range filter
-- @mvcc_sensitive
SET paradedb.enable_aggregate_custom_scan TO on; SELECT pdb.agg('{"date_histogram": {"field": "creation_date", "fixed_interval": "1d"}}') FROM stackoverflow_posts WHERE body ||| 'code' AND post_type_id = 1 AND creation_date >= '2015-01-01';
