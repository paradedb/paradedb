SELECT * FROM stackoverflow_posts WHERE body @@@ 'javascript' AND tags @@@ 'python' ORDER BY score, creation_date LIMIT 10;
