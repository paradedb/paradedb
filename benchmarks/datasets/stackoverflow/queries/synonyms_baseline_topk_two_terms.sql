SELECT id FROM stackoverflow_posts WHERE body ||| 'python exception'::pdb.unicode_words ORDER BY paradedb.score(id) DESC LIMIT 10;
