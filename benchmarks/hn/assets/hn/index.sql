CREATE INDEX {{ index_name }} ON {{ table }}
USING paradedb (
    id,
    ("by"::pdb.literal),
    time,
    emb vector_cosine_ops
) WITH ({{ with_options }});
