-- string ff
SELECT profile_image_url, COUNT(*) FROM users WHERE id @@@ paradedb.all() GROUP BY profile_image_url ORDER BY profile_image_url;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'users_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "profile_image_url" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'users_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "profile_image_url" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT profile_image_url, COUNT(*) FROM users WHERE id @@@ paradedb.all() GROUP BY profile_image_url;

-- pdb.agg with GROUP BY
SELECT profile_image_url, pdb.agg('{"terms": {"field": "profile_image_url"}}'::jsonb) FROM users WHERE id @@@ paradedb.all() GROUP BY profile_image_url;
