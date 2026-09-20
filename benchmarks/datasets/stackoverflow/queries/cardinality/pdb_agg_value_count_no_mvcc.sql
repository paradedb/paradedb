-- pdb.agg without GROUP BY (mvcc disabled)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT pdb.agg('{"value_count": {"field": "post_type_id"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript';
