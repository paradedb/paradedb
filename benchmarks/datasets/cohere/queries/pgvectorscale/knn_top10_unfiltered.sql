-- query_rescore is the recall lever here: the graph is 1-bit SBQ, so its own ordering is coarse and
-- recall is set by how many candidates get exact-distance rescoring, not by beam width.
SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET diskann.query_search_list_size=100; SET diskann.query_rescore={{ pgvectorscale_rescore_unfiltered }}; SELECT _id, title FROM cohere_wiki
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
