CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING paradedb (
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
    training_sample_ratio = 0.32
);
