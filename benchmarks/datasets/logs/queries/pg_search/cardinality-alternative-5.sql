-- pdb.agg without GROUP BY
SELECT pdb.agg('{"terms": {"field": "severity"}}'::jsonb) FROM benchmark_logs WHERE message @@@ 'research';
