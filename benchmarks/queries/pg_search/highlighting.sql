SELECT id, paradedb.snippet(message) FROM benchmark_logs WHERE message @@@ 'fox' LIMIT 10;
