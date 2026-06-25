CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING hnsw (emb vector_cosine_ops);

-- Companion indexes for filtered-search benchmarks: btree drives selective pre-filters
-- (title equality), GIN/tsvector drives full-text predicates combined with kNN.
CREATE INDEX cohere_wiki_title_idx ON cohere_wiki (title);
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
