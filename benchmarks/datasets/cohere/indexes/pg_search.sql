-- pg_search (bm25) index that drives BOTH the full-text predicate and the vector kNN of the filtered
-- benchmarks from a single index -- no companion GIN/tsvector index (unlike hnsw/ivfflat/vchord).
--   * `_id`  -- key_field.
--   * `text` -- tokenized with english stemming + stopwords so `text @@@ '<term>'` matches the same
--              rows as the other implementations' `to_tsvector('english', text) @@ websearch_to_tsquery`
--              predicate, letting the filtered queries share the tsvector ground truth for recall.
--              Multiple tokenizer options are separate quoted typmod args (not comma-joined in one).
--   * `emb`  -- vector_cosine_ops IVF field, so `ORDER BY emb <=> qvec` pushes down through the bm25
--              TopK scan (the operator's metric must match the opclass: <=> -> cosine).
-- `centroid_ratio` sets the per-segment IVF cluster count (= ratio * segment rows, analogous to
-- ivfflat/vchord `lists`). It is a float, so it is written as a literal here rather than a `{{ }}`
-- param (the benchmark resolves params as ::bigint). 0.01 is the extension default, kept explicit to
-- document the operating point. Recall is tuned at query time via the probe knobs
-- (paradedb.vector_cluster_max_probe_fraction / _probe_epsilon) in queries/pg_search/*.sql.
-- `target_segment_count` fixes the build at 8 segments: the probe knobs are tuned for ~95%
-- recall@10 at this count, whereas the runner's default (~48) overshoots to ~99%. This is honored
-- because the cohere snapshot bakes in `global_target_segment_count = 0` (a non-zero global would
-- override the per-index value); see .github/actions/setup-benchmark-cluster.
CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING bm25 (
    _id,
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
    key_field = '_id',
    centroid_ratio = 0.01,
    target_segment_count = 8
);
