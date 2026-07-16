-- Variant 1: force the diskann index. Arbitrary-WHERE filtering is automatic post-filter streaming
-- (no prefilter GUC): the ANN scan streams candidates and the tsvector predicate filters them. A
-- stricter filter forces the scan to stream more candidates, hence the larger search_list_size.
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET diskann.query_search_list_size={{ pgvectorscale_search_list_size_1pct }}; SET diskann.query_rescore={{ pgvectorscale_query_rescore_1pct }}; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;

-- Variant 2: exact pre-filter
SET enable_indexscan=off; SET enable_bitmapscan=on; SET enable_sort=on; SELECT _id, title FROM cohere_wiki
WHERE to_tsvector('english', text) @@ websearch_to_tsquery('english', current_setting('cohere.titles_1pct'))
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
