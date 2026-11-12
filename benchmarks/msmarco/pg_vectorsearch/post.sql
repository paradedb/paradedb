DROP INDEX IF EXISTS {{ index_name }};

CREATE INDEX {{ index_name }} ON {{ table }}
USING prism (emb vec32_cosine_ops)
WITH ({{ with_options }});

VACUUM ANALYZE {{ table }};
