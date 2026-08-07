-- pgvectorscale StreamingDiskANN index (`diskann`). Reuses pgvector's `vector` type, so the corpus
-- schema is unchanged. The `vectorscale` extension is created by the benchmark workflow's
-- post-restore step (this file holds only CREATE INDEX statements, like the hnsw/ivfflat/vchord
-- files -- the harness runs each statement and reads its index metadata, so a CREATE EXTENSION here
-- would break that). https://github.com/timescale/pgvectorscale
--
-- Built with the default SBQ compression: `num_bits_per_dimension` defaults to 1 above 900 dims, so
-- cohere's 1024-dim vectors are 1-bit quantized. `storage_layout = plain` would lift filtered recall
-- but builds far too slowly to run here, so the filtered arms are benchmarked at their achievable
-- ceiling instead -- see the sweep comments in config.toml.
CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING diskann (emb vector_cosine_ops);

-- Companion index for filtered-search benchmarks: GIN/tsvector drives the full-text predicate
-- combined with kNN.
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
