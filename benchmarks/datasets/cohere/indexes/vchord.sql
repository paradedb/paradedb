-- VectorChord RaBitQ IVF index (`vchordrq`). Reuses pgvector's `vector` type, so the corpus schema
-- is unchanged; `CREATE EXTENSION vchord CASCADE` pulls in `vector`. Requires `vchord` in
-- shared_preload_libraries (the benchmark workflow re-applies this after restoring the heap
-- snapshot). `{{ vchord_lists }}` resolves from the dataset's [params] in config.toml (scales with
-- dataset_size per VectorChord's recommended lists table); it must be an integer literal.
--
-- Options follow the VectorChord docs' recommendation for cosine similarity: enable
-- `residual_quantization` and `build.internal.spherical_centroids` to improve both QPS and recall.
-- https://docs.vectorchord.ai/vectorchord/usage/indexing.html
CREATE EXTENSION IF NOT EXISTS vchord CASCADE;

CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING vchordrq (emb vector_cosine_ops)
  WITH (options = $$
residual_quantization = true
[build.internal]
lists = [{{ vchord_lists }}]
spherical_centroids = true
build_threads = 8
$$);

-- Companion index for filtered-search benchmarks: GIN/tsvector drives the full-text predicate
-- combined with kNN.
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
