-- string ff
SELECT tags, COUNT(*) FROM stackoverflow_posts WHERE id @@@ paradedb.all() GROUP BY tags ORDER BY tags;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "tags" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'stackoverflow_posts_idx', query=>paradedb.all(), agg=>'{"buckets": { "terms": { "field": "tags" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT tags, COUNT(*) FROM stackoverflow_posts WHERE id @@@ paradedb.all() GROUP BY tags;

-- pdb.agg with GROUP BY
SELECT tags, pdb.agg('{"terms": {"field": "tags"}}'::jsonb) FROM stackoverflow_posts WHERE id @@@ paradedb.all() GROUP BY tags;
