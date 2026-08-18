SET paradedb.vector_cluster_max_probe={{ pg_search_max_probe_10pct }}; SELECT _id, title FROM cohere_wiki
WHERE text @@@ current_setting('cohere.titles_10pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
