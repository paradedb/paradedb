-- pdb.agg date_histogram with a date range filter (mvcc disabled)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT pdb.agg('{"date_histogram": {"field": "creation_date", "fixed_interval": "1d"}}', false) FROM stackoverflow_posts WHERE body ||| 'code' AND post_type_id = 1 AND creation_date >= '2015-01-01';
