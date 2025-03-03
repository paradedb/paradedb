SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY severity LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY country LIMIT 10;
SELECT * FROM benchmark_logs WHERE message @@@ 'research' AND country @@@ 'Canada' ORDER BY timestamp LIMIT 10;
