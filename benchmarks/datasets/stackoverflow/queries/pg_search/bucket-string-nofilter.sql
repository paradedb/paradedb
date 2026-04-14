-- string ff
SELECT location, COUNT(*) FROM users WHERE id @@@ paradedb.all() GROUP BY location ORDER BY location;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'users_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "location" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'users_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "location" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT location, COUNT(*) FROM users WHERE id @@@ paradedb.all() GROUP BY location;

-- pdb.agg with GROUP BY
SELECT location, pdb.agg('{"terms": {"field": "location"}}'::jsonb) FROM users WHERE id @@@ paradedb.all() GROUP BY location;
