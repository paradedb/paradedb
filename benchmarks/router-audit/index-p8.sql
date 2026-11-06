CREATE INDEX cohere_wiki_bm25_idx ON cohere_wiki
USING paradedb (
    (text::pdb.unicode_words('stemmer=english', 'stopwords_language=english')),
    emb vector_cosine_ops
) WITH (
    training_sample_ratio = 0.32,
    target_segment_count = 8,
    partition_by = 'emb',
    vector_router_quantization = true,
    spill = false
);
