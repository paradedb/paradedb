-- pgvectorscale StreamingDiskANN index (`diskann`). Reuses pgvector's `vector` type, so the corpus
-- schema is unchanged. The `vectorscale` extension is created by the benchmark workflow's
-- post-restore step (this file holds only CREATE INDEX statements, like the hnsw/ivfflat/vchord
-- files -- the harness runs each statement and reads its index metadata, so a CREATE EXTENSION here
-- would break that). https://github.com/timescale/pgvectorscale
--
-- storage_layout = plain (uncompressed) rather than the default SBQ compression. A local recall
-- sweep on the 1M cohere set showed the default 1-bit SBQ (num_bits_per_dimension defaults to 1 at
-- >900 dims) caps recall@10 on the FILTERED queries -- ~0.88 (10pct) / ~0.67 (1pct) -- at ANY
-- query-time setting, because post-filtering keeps only ~1-10% of the <=1000 candidates diskann can
-- exact-rescore (diskann.query_rescore maxes at 1000). Plain storage keeps full-precision vectors in
-- the index so traversal uses exact distances, letting the filtered arms reach the ~95% operating
-- point the hnsw/ivfflat/vchord arms are tuned to. Trade-off: a larger index and a much slower build.
CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING diskann (emb vector_cosine_ops)
  WITH (storage_layout = plain);

-- Companion index for filtered-search benchmarks: GIN/tsvector drives the full-text predicate
-- combined with kNN.
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
