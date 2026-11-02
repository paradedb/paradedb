SELECT id FROM stackoverflow_posts WHERE body ||| 'exception' ORDER BY paradedb.score(id) DESC LIMIT 10;
