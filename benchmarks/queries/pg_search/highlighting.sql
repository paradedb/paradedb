SELECT id, paradedb.snippet(message) FROM benchmark_logs WHERE message @@@ 'research' LIMIT 10;
