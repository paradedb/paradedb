SELECT id FROM stackoverflow_posts WHERE body ||| pdb.with_synonyms('python exception', 'bench_synonyms_idx') ORDER BY pdb.score(id) DESC LIMIT 10;
