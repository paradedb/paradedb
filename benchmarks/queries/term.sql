SELECT * FROM benchmark_logs WHERE message @@@ 'fox' OR country @@@ 'canada' LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'fox' OR country @@@ 'canada' LIMIT 250;
