CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING paradedb (
    _id,
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
    boundary_epsilon = 0.35,
    training_sample_ratio = 0.32,
    target_segment_count = 8
);
