-- numeric ff
SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message @@@ 'research';

-- better numeric ff
SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity);

-- aggregate with mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{"buckets": { "terms": { "field": "severity" }}}', solve_mvcc=>true);

-- aggregate without mvcc
SELECT * FROM paradedb.aggregate(index=>'benchmark_logs_idx', query=>paradedb.term('message', 'research'), agg=>'{"buckets": { "terms": { "field": "severity" }}}', solve_mvcc=>false);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity ORDER BY severity);
