SELECT id, pdb.snippet(body), pdb.snippet(tags) FROM stackoverflow_posts WHERE body @@@ 'javascript' AND tags @@@ 'python' LIMIT 10;
