SELECT *, paradedb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY paradedb.score(id) LIMIT 10;
