-- Use the bm25 alias for benchmark runs against older refs.

CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING bm25 (
    _id,
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
    training_sample_ratio = 0.32,
    max_leaf_size = 100,
    target_segment_count = 8
);
