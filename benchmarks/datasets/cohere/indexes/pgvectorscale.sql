-- pgvectorscale StreamingDiskANN index (`diskann`). Reuses pgvector's `vector` type, so the corpus
-- schema is unchanged. The `vectorscale` extension is created by the benchmark workflow's
-- post-restore step (this file holds only CREATE INDEX statements, like the hnsw/ivfflat/vchord
-- files -- the harness runs each statement and reads its index metadata, so a CREATE EXTENSION here
-- would break that). https://github.com/timescale/pgvectorscale
--
-- Default build (SBQ compression -- num_bits_per_dimension defaults to 1 at >900 dims, so cohere's
-- 1024-d vectors are 1-bit quantized). This is pgvectorscale's headline config: a compact index and
-- a fast build. A local recall sweep on the 1M cohere set showed the trade-off -- recall@10 tops out
-- BELOW 0.95 on the filtered queries (~0.88 at 10pct, ~0.67 at 1pct) at ANY query-time setting,
-- because post-filtering keeps only ~1-10% of the <=1000 candidates diskann can exact-rescore
-- (diskann.query_rescore caps at 1000). Uncompressed `storage_layout = plain` would lift filtered
-- recall to ~0.95 but builds far too slowly to be practical here, so this arm is benchmarked at the
-- default operating point and its query params (config.toml) are tuned to the best achievable recall
-- rather than to a fixed 0.95. See queries/pgvectorscale/ + config.toml.
CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING diskann (emb vector_cosine_ops);

-- Companion index for filtered-search benchmarks: GIN/tsvector drives the full-text predicate
-- combined with kNN.
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
