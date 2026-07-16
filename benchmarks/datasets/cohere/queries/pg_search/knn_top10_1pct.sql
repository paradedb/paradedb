-- pg_search filtered kNN (~1% selectivity): same single-index design as the 10% variant -- the bm25
-- index serves the `text @@@ '<term>'` predicate and the `ORDER BY emb <=> qvec` IVF vector TopK
-- together. A stricter filter surfaces fewer matches per probed cluster, so this arm typically needs
-- more probes to still return 10 in-filter neighbors at ~95% recall@10; probe knobs resolve from
-- [params], tuned per size.
SET paradedb.vector_cluster_max_probes={{ pg_search_probes_1pct }}; SET paradedb.vector_cluster_probe_epsilon={{ pg_search_epsilon }}; SELECT _id, title FROM cohere_wiki
WHERE text @@@ current_setting('cohere.titles_1pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
