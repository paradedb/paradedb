-- numeric ff
SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'javascript'), agg=>'{"buckets": { "terms": { "field": "post_type_id" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'javascript'), agg=>'{"buckets": { "terms": { "field": "post_type_id" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT post_type_id, COUNT(*) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id;

-- pdb.agg with GROUP BY
SELECT post_type_id, pdb.agg('{"value_count": {"field": "post_type_id"}}'::jsonb) FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id;
