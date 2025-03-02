SELECT * FROM benchmark_logs WHERE message @@@ '"ancient artifacts"' OR country @@@ '"United States"' LIMIT 10;
