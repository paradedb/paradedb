-- pdb.agg with GROUP BY
SELECT severity, pdb.agg('{"terms": {"field": "severity"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research' GROUP BY severity;
