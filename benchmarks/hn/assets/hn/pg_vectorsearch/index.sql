CREATE INDEX {{ index_name }} ON {{ table }}
USING prism (emb vec32_cosine_ops)
WITH ({{ with_options }});
