SELECT *, pdb.score(id) FROM benchmark_logs WHERE message ||| 'discovered analyzed monitored captured processed' ORDER BY pdb.score(id) DESC LIMIT 10;
