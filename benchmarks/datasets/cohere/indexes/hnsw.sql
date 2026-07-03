CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING hnsw (emb vector_cosine_ops);

-- Companion index for filtered-search benchmarks: GIN/tsvector drives the full-text predicate
-- combined with kNN (used by the exact pre-filter query variant).
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
