-- Variant 1: force the ANN index. max_scan_tuples/scan_mem_multiplier bound the filtered scan;
-- they are not recall knobs, so they stay fixed while ef_search is swept.
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET hnsw.iterative_scan=relaxed_order; SET hnsw.ef_search={{ ef_search_1pct }}; SET hnsw.max_scan_tuples={{ max_scan_tuples_1pct }}; SET hnsw.scan_mem_multiplier=16; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;

-- Variant 2: exact pre-filter
SET enable_indexscan=off; SET enable_bitmapscan=on; SET enable_sort=on; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
