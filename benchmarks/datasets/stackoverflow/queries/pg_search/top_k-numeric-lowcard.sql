SELECT * FROM stackoverflow_posts WHERE body @@@ 'javascript' AND tags @@@ 'python' ORDER BY post_type_id LIMIT 10;
