-- string ff
SELECT tags, COUNT(*) FROM stackoverflow_posts WHERE id @@@ paradedb.all() GROUP BY tags ORDER BY tags;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "owner_display_name" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "owner_display_name" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT owner_display_name, COUNT(*) FROM stackoverflow_posts WHERE id @@@ paradedb.all() GROUP BY owner_display_name;

-- pdb.agg with GROUP BY
SELECT owner_display_name, pdb.agg('{"terms": {"field": "owner_display_name"}}'::jsonb) FROM stackoverflow_posts WHERE id @@@ paradedb.all() GROUP BY owner_display_name ORDER BY owner_display_name LIMIT 65000;
