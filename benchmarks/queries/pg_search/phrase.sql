SELECT * FROM benchmark_logs WHERE message @@@ '"quick brown fox"' OR country @@@ '"United States"' LIMIT 10;
