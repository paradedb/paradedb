SELECT *, pdb.score(id) FROM benchmark_logs WHERE message @@@ 'research' ORDER BY pdb.score(id) LIMIT 10;
