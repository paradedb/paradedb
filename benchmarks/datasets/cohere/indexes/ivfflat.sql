-- pgvector IVFFlat index. `{{ lists }}` resolves from the dataset's [params] in config.toml
-- (it scales with dataset_size); `lists` must be an integer literal in the WITH clause.
CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING ivfflat (emb vector_cosine_ops)
  WITH (lists = {{ lists }});

-- Companion indexes for filtered-search benchmarks: btree drives selective pre-filters
-- (title equality), GIN/tsvector drives full-text predicates combined with kNN.
CREATE INDEX cohere_wiki_title_idx ON cohere_wiki (title);
CREATE INDEX cohere_wiki_text_fts_idx ON cohere_wiki USING gin (to_tsvector('english', text));
