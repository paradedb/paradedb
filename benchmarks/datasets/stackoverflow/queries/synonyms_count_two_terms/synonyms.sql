SELECT count(*) FROM stackoverflow_posts WHERE body ||| pdb.with_synonyms('python exception', 'bench_synonyms_idx');
