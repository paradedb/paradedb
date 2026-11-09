CREATE INDEX {{ index_name }} ON {{ table }}
USING paradedb (
    id,
    (type::pdb.literal),
    ("by"::pdb.literal),
    score,
    time,
    emb vector_cosine_ops
) WITH ({{ with_options }});
