SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10;

-- force single worker
SET max_parallel_workers_per_gather=0; SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'why' ORDER BY pdb.score(id) DESC LIMIT 10;
