-- pgvector IVFFlat index. `{{ lists }}` resolves from the dataset's [params] in config.toml
-- (it scales with dataset_size); `lists` must be an integer literal in the WITH clause.
CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING ivfflat (emb vector_cosine_ops)
  WITH (lists = {{ lists }});
