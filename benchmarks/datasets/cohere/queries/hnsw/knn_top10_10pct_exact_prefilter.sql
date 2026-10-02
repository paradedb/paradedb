-- Exact pre-filter baseline (GIN index first, then sort vectors)
SET enable_indexscan=off; SET enable_bitmapscan=on; SET enable_sort=on; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_10pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
