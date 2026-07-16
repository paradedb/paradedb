SET ivfflat.probes={{ probes_unfiltered }}; SET hnsw.ef_search={{ ef_search_unfiltered }}; SELECT _id, title FROM cohere_wiki
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
