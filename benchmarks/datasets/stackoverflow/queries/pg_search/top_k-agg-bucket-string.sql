SELECT id, title, tags, post_type_id, creation_date, pdb.agg('{"terms": {"field": "tags"}}'::jsonb) OVER () FROM stackoverflow_posts WHERE body @@@ 'javascript' ORDER BY creation_date DESC LIMIT 10;
