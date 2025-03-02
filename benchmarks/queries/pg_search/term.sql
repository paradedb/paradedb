SELECT * FROM benchmark_logs WHERE message @@@ 'research' OR country @@@ 'Canada' LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'research' OR country @@@ 'Canada' LIMIT 250;
