SET ivfflat.iterative_scan TO relaxed_order;
SET hnsw.iterative_scan TO relaxed_order;

SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', 'protein')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
