-- pg_search unfiltered kNN: the bm25 TopK scan drives the whole query. `_id @@@ paradedb.all()`
-- engages the bm25 custom scan (there is no text predicate to select on), and `ORDER BY emb <=> qvec`
-- pushes down as an IVF vector TopK. The recall lever is paradedb.vector_cluster_max_probes (nprobe);
-- paradedb.vector_cluster_probe_epsilon widens the probe radius. Both resolve from [params] in
-- config.toml, tuned per size for ~95% recall@10.
SET paradedb.vector_cluster_max_probes={{ pg_search_probes_unfiltered }}; SET paradedb.vector_cluster_probe_epsilon={{ pg_search_epsilon }}; SELECT _id, title FROM cohere_wiki
WHERE _id @@@ paradedb.all()
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
