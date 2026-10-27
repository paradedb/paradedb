SELECT _id, title FROM {{ table }}
WHERE text @@@ current_setting('cohere.titles_1pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector({{ dims }})
LIMIT 10;
