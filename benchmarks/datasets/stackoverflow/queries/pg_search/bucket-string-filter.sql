-- string ff
SELECT name, COUNT(*) FROM badges WHERE tag_based === true GROUP BY name ORDER BY name;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'badges_idx', query=>paradedb.term('tag_based', true), agg=>'{"buckets": { "terms": { "field": "name" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'badges_idx', query=>paradedb.term('tag_based', true), agg=>'{"buckets": { "terms": { "field": "name" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT name, COUNT(*) FROM badges WHERE tag_based === true GROUP BY name;

-- pdb.agg with GROUP BY
SELECT name, pdb.agg('{"terms": {"field": "name"}}'::jsonb) FROM badges WHERE tag_based === true GROUP BY name;
