SELECT id, message, country, severity, timestamp, COUNT(*) OVER () FROM benchmark_logs WHERE message @@@ 'research' ORDER BY timestamp DESC LIMIT 10;
