SELECT _id, title FROM {{ table }}
WHERE _id @@@ paradedb.all()
ORDER BY emb <=> current_setting('msmarco.qvec')::vector({{ dims }})
LIMIT 10;
