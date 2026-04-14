-- numeric fast field
SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error';

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'error'), agg=>'{"count": { "value_count": { "field": "ctid" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'error'), agg=>'{"count": { "value_count": { "field": "ctid" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM stackoverflow_posts WHERE body ||| 'error';

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "ctid"}}'::jsonb) FROM stackoverflow_posts WHERE body ||| 'error';
