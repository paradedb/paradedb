SET paradedb.vector_cluster_max_probe={{ pg_search_probe_fraction_1pct }}; SELECT _id, title FROM cohere_wiki
WHERE text @@@ current_setting('cohere.titles_1pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
