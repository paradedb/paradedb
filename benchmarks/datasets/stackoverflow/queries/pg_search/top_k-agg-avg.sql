SELECT id, title, tags, score, creation_date, pdb.agg('{"avg": {"field": "score"}}') OVER () FROM stackoverflow_posts WHERE body ||| 'javascript' ORDER BY creation_date DESC LIMIT 10;
