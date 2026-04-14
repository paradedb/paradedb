SELECT id, message, country, severity, timestamp, pdb.agg('{"value_count": {"field": "country"}}'::jsonb) OVER () FROM benchmark_logs WHERE message ||| 'research' ORDER BY timestamp DESC LIMIT 10;
