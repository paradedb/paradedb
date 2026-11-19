SELECT _id, title FROM {{ table }}
WHERE emb @@@ paradedb.all()
ORDER BY emb <=> current_setting('msmarco.qvec')::vector({{ dims }})
LIMIT 100;
