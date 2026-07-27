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
