SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'code' ORDER BY pdb.score(id) LIMIT 10;
