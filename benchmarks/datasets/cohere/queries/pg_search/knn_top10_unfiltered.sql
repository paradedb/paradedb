SET paradedb.vector_cluster_max_probe=0.02; SET paradedb.vector_cluster_probe_epsilon={{ pg_search_probe_epsilon_unfiltered }}; SELECT _id, title FROM cohere_wiki
WHERE _id @@@ paradedb.all()
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
