CREATE INDEX {{ index_name }} ON {{ table }}
USING paradedb (_id, emb vector_cosine_ops)
WITH ({{ with_options }});
