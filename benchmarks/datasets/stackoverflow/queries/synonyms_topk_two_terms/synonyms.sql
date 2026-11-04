SET paradedb.enable_synonyms TO on; SELECT id FROM stackoverflow_posts WHERE body ||| 'python exception' ORDER BY paradedb.score(id) DESC LIMIT 10;
