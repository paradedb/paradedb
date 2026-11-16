SELECT _id, title FROM {{ table }}
ORDER BY emb <=> current_setting('cohere.qvec')::vector({{ dims }})
LIMIT 10;
