CREATE INDEX benchmark_logs_idx ON benchmark_logs
USING bm25 (id, message, country, severity, timestamp, metadata)
WITH (
    key_field = 'id',
    text_fields = '{"country": {"fast": true, "tokenizer": {"type": "raw", "lowercase": true} }}',
    json_fields = '{"metadata": { "fast": true }}'
);
