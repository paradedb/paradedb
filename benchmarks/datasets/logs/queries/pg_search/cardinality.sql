-- numeric ff
SELECT COUNT(DISTINCT severity) FROM benchmark_logs WHERE message ||| 'research';

-- better numeric ff
SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message ||| 'research' GROUP BY severity ORDER BY severity);

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT COUNT(*) FROM (SELECT severity FROM benchmark_logs WHERE message ||| 'research' GROUP BY severity);

-- pdb.agg without GROUP BY
SELECT pdb.agg('{"value_count": {"field": "severity"}}') FROM benchmark_logs WHERE message ||| 'research';
