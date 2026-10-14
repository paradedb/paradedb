-- tantivy cardinality agg on a string field (mvcc disabled baseline)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT pdb.agg('{"cardinality": {"field": "tags"}}', false) FROM stackoverflow_posts WHERE body ||| 'javascript';
