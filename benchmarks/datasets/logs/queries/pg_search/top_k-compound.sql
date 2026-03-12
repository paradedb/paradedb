SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity, timestamp LIMIT 10;
