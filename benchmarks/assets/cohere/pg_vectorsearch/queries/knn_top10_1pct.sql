SELECT _id, title FROM {{ table }}
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector({{ dims }})
LIMIT 10;
