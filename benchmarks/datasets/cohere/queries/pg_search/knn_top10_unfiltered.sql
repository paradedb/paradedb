SET paradedb.vector_cluster_max_probe={{ pg_search_max_probe_unfiltered }}; SELECT _id, title FROM cohere_wiki
WHERE _id @@@ paradedb.all()
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
