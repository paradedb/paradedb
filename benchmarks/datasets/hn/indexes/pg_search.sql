CREATE INDEX hn_items_bm25_idx ON hn_items
USING bm25 (
    id,
    (type::pdb.literal),
    ("by"::pdb.literal),
    score,
    time,
    emb vector_cosine_ops
) WITH (
    key_field = 'id',
    training_sample_ratio = 0.32,
    target_segment_count = 8
);
