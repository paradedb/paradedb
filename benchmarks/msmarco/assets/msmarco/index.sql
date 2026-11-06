CREATE INDEX {{ index_name }} ON {{ table }}
USING paradedb (emb vector_cosine_ops)
WITH ({{ with_options }}, spill = true);
