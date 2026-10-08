SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'use' ORDER BY pdb.score(id) DESC LIMIT 10;
