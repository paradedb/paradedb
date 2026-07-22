SET diskann.query_search_list_size={{ pgvectorscale_search_list_size_unfiltered }}; SET diskann.query_rescore={{ pgvectorscale_query_rescore_unfiltered }}; SELECT _id, title FROM cohere_wiki
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
