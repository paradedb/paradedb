CREATE INDEX {{ index_name }} ON {{ table }}
USING vchordrq (emb vector_cosine_ops)
WITH ({{ with_options }});
