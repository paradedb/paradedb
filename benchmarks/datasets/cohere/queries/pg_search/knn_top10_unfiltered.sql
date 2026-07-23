-- pg_search unfiltered kNN: the bm25 TopK scan drives the whole query. `_id @@@ paradedb.all()`
-- engages the bm25 custom scan (there is no text predicate to select on), and `ORDER BY emb <=> qvec`
-- pushes down as an IVF vector TopK. The probe ceiling is FIXED (max_probe_fraction); the
-- distance-ratio gate (probe_epsilon) tunes recall. With no filter every cluster costs full budget, so
-- this arm is the most ceiling-sensitive and uses the lowest epsilon (raising it past the ceiling does
-- nothing). Inlined literals, not {{ }} params (the runner casts params to ::bigint). Tuned for ~95%
-- recall@10 on cohere 1M.
SET paradedb.vector_cluster_max_probe_fraction=0.05; SET paradedb.vector_cluster_probe_epsilon=0.4; SELECT _id, title FROM cohere_wiki
WHERE _id @@@ paradedb.all()
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
