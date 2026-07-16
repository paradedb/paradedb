-- Variant 1: force the ANN index
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET ivfflat.iterative_scan=relaxed_order; SET hnsw.iterative_scan=relaxed_order; SET ivfflat.probes={{ probes_10pct }}; SET hnsw.ef_search={{ ef_search_10pct }}; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_10pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;

-- Variant 2: exact pre-filter
SET enable_indexscan=off; SET enable_bitmapscan=on; SET enable_sort=on; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_10pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
