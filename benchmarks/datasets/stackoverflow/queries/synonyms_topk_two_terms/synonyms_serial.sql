SET max_parallel_workers_per_gather TO 0; SELECT id FROM stackoverflow_posts WHERE body ||| (SELECT pdb.expand('python exception', rule) FROM bench_synonyms) ORDER BY pdb.score(id) DESC LIMIT 10;
