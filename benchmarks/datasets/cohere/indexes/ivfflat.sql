-- pgvector IVFFlat index. `lists` is a static starting point (~rows/1000 for the 1m set); it is not
-- size-aware, so it is undersized for 10m and worth tuning per size later.
CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING ivfflat (emb vector_cosine_ops) WITH (lists = 1000);
