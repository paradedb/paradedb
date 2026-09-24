CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING bm25 (
    _id,
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
<<<<<<< HEAD
    key_field = '_id',
    centroid_ratio = 0.01,
    target_segment_count = 8,
    cluster_replication = 1
=======
    training_sample_ratio = 0.32,
    max_leaf_size = 100,
    target_segment_count = 8
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
);
