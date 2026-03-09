SELECT id, message, country, severity, timestamp, pdb.agg('{"avg": {"field": "severity"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10;
