-- numeric ff
SELECT COUNT(DISTINCT post_type_id) FROM stackoverflow_posts WHERE body ||| 'javascript';

-- better numeric ff
SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id ORDER BY post_type_id);

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'javascript'), agg=>'{"buckets": { "terms": { "field": "post_type_id" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'javascript'), agg=>'{"buckets": { "terms": { "field": "post_type_id" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT post_type_id FROM stackoverflow_posts WHERE body ||| 'javascript' GROUP BY post_type_id);

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"terms": {"field": "post_type_id"}}'::jsonb) FROM stackoverflow_posts WHERE body ||| 'javascript';
