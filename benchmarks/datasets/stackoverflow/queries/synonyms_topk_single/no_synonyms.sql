SELECT id FROM stackoverflow_posts WHERE body ||| 'exception' ORDER BY pdb.score(id) DESC LIMIT 10;
