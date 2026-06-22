CREATE INDEX cohere_wiki_emb_idx ON cohere_wiki USING hnsw (emb vector_cosine_ops);
