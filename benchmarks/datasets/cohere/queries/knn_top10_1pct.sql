SET ivfflat.iterative_scan TO relaxed_order;
SET hnsw.iterative_scan TO relaxed_order;

SELECT _id, title FROM cohere_wiki
WHERE title = ANY (current_setting('cohere.titles_1pct')::text[])
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
