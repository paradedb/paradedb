SELECT id, message, country, severity, timestamp, paradedb.agg('{"terms": {"field": "country"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10;
