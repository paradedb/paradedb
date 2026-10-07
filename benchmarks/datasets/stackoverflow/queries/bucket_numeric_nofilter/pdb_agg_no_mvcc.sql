-- pdb.agg with GROUP BY (mvcc disabled)
SET paradedb.enable_aggregate_custom_scan TO off; SELECT post_type_id, pdb.agg('{"value_count": {"field": "post_type_id"}}', false) FROM stackoverflow_posts WHERE id @@@ pdb.all() GROUP BY post_type_id;
