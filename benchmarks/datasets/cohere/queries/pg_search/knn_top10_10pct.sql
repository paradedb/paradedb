-- pg_search filtered kNN (~10% selectivity): a single bm25 index drives BOTH the full-text predicate
-- and the vector kNN -- `text @@@ '<term>'` replaces the hnsw/ivfflat/vchord tsvector prefilter, and
-- `ORDER BY emb <=> qvec` pushes down as an IVF vector TopK over the matching rows. The text field is
-- indexed with english stemming (see indexes/pg_search.sql), so this matches the same rows as
-- `to_tsvector('english', text) @@ websearch_to_tsquery('english', '<term>')` and shares its ground
-- truth. The probe ceiling is FIXED (max_probe_fraction); recall is tuned via the distance-ratio gate
-- (probe_epsilon). Inlined literals, not {{ }} params (the runner casts params to ::bigint, truncating
-- fractions). Tuned for ~95% recall@10 on cohere 1M.
SET paradedb.vector_cluster_max_probe_fraction=0.05; SET paradedb.vector_cluster_probe_epsilon=0.45; SELECT _id, title FROM cohere_wiki
WHERE text @@@ current_setting('cohere.titles_10pct')
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
