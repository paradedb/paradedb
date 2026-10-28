SELECT _id, title FROM {{ table }}
WHERE _id @@@ paradedb.all()
ORDER BY emb <=> current_setting('cohere.qvec')::vector({{ dims }})
LIMIT 10;
