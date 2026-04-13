-- string ff
SELECT owner_display_name, COUNT(*) FROM stackoverflow_posts WHERE body @@@ 'javascript' GROUP BY owner_display_name ORDER BY owner_display_name;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'javascript'), agg=>'{"buckets": { "terms": { "field": "owner_display_name" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.term('body', 'javascript'), agg=>'{"buckets": { "terms": { "field": "owner_display_name" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT owner_display_name, COUNT(*) FROM stackoverflow_posts WHERE body @@@ 'javascript' GROUP BY owner_display_name;

-- pdb.agg with GROUP BY
SELECT owner_display_name, pdb.agg('{"terms": {"field": "owner_display_name"}}'::jsonb) FROM stackoverflow_posts WHERE body @@@ 'javascript' GROUP BY owner_display_name;
