SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10;
