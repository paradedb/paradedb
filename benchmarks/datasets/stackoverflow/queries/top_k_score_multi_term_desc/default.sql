SELECT *, pdb.score(id) FROM stackoverflow_posts WHERE body ||| 'javascript python react angular typescript' ORDER BY pdb.score(id) DESC LIMIT 10;
