-- tantivy cardinality agg on a string field (mvcc enabled -> lazy vischeck)
SET work_mem TO '4GB'; SET paradedb.enable_aggregate_custom_scan TO off; SELECT pdb.agg('{"cardinality": {"field": "tags"}}', true) FROM stackoverflow_posts WHERE body ||| 'javascript';
