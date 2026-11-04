SELECT _id, title FROM {{ table }}
WHERE emb @@@ paradedb.all()
ORDER BY emb <=> current_setting('cohere.qvec')::vector({{ dims }})
LIMIT 10;
