-- pg_search filtered kNN (~1% selectivity): one bm25 index serves both the `text @@@ '<term>'`
-- predicate and the `ORDER BY emb <=> qvec` IVF vector TopK. The probe ceiling is FIXED
-- (max_probe_fraction, a fraction of each segment's clusters -- a size-portable tail-latency guard),
-- and recall is tuned per arm via the distance-ratio gate (probe_epsilon). A stricter filter needs a
-- wider gate to still return 10 in-filter neighbors, so this arm's epsilon is the highest of the
-- three. Both are inlined literals, not {{ }} params: the runner casts params to ::bigint, which
-- would truncate these fractions to 0. Tuned for ~95% recall@10 on cohere 1M (0.01 ratio, 8 segs).
SET paradedb.vector_cluster_max_probe_fraction=0.05; SET paradedb.vector_cluster_probe_epsilon=0.65; SELECT _id, title FROM cohere_wiki
WHERE text @@@ current_setting('cohere.titles_1pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
