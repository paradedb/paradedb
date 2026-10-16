CREATE INDEX paradedb ON hn_items
USING paradedb (
    id,
    (type::pdb.literal),
    ("by"::pdb.literal),
    score,
    time,
    emb vector_cosine_ops
) WITH (
    training_sample_ratio = 0.32,
    target_segment_count = 4
);
