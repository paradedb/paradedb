SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY paradedb.score(id) LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY severity LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY country LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'fox' ORDER BY timestamp LIMIT 10;
