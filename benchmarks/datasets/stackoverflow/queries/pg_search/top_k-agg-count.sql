SELECT id, title, tags, post_type_id, creation_date, COUNT(*) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10;
