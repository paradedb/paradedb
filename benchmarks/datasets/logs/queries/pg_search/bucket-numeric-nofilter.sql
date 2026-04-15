-- numeric ff
SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ pdb.all() GROUP BY severity ORDER BY severity;

-- aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on; SELECT severity, COUNT(*) FROM benchmark_logs WHERE id @@@ pdb.all() GROUP BY severity;

-- pdb.agg with GROUP BY
SELECT severity, pdb.agg('{"value_count": {"field": "severity"}}') FROM benchmark_logs WHERE id @@@ pdb.all() GROUP BY severity;

-- pdb.agg with GROUP BY (mvcc disabled)
SELECT severity, pdb.agg('{"value_count": {"field": "severity"}}', false) FROM benchmark_logs WHERE id @@@ pdb.all() GROUP BY severity;
