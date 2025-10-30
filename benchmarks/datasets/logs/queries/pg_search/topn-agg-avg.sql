SET paradedb.enable_aggregate_custom_scan TO on; SELECT id, message, country, severity, timestamp, paradedb.agg('{"avg": {"field": "severity"}}'::jsonb) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10;

