-- string ff
SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country ORDER BY country;

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{"buckets": { "terms": { "field": "country" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{"buckets": { "terms": { "field": "country" }}}', solve_mvcc=>false);

-- pdb.agg with GROUP BY
SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, pdb.agg('{"terms": {"field": "country"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country;

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT country, COUNT(*) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY country;
