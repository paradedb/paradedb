SELECT id, title, tags, score, creation_date, AVG(score) OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' and id @@@ pdb.all() ORDER BY creation_date DESC LIMIT 10;
