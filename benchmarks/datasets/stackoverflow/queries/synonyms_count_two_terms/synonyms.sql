SELECT count(*) FROM stackoverflow_posts WHERE body ||| (SELECT pdb.expand('python exception', rule) FROM bench_synonyms);
