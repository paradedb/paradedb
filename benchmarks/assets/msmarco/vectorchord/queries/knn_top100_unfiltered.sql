SELECT _id, title
FROM {{ table }}
ORDER BY emb <=> current_setting('msmarco.qvec')::vector({{ dims }})
LIMIT 100;
