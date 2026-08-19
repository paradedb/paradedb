\set ON_ERROR_STOP on

CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_maintenance_workers = 0;

DROP INDEX IF EXISTS anti_bench_library_bm25;
DROP INDEX IF EXISTS anti_bench_owned_bm25;
CREATE INDEX anti_bench_library_bm25 ON anti_bench_library
USING bm25 (id, title, category)
WITH (
    key_field = 'id',
    target_segment_count = 64,
    background_layer_sizes = '0',
    text_fields = '{
        "title": {"fast": true},
        "category": {"fast": true, "tokenizer": {"type": "keyword"}}
    }'
);

CREATE INDEX anti_bench_owned_bm25 ON anti_bench_owned
USING bm25 (id, user_id, item_id)
WITH (
    key_field = 'id',
    target_segment_count = 64,
    background_layer_sizes = '0',
    numeric_fields = '{
        "user_id": {"fast": true},
        "item_id": {"fast": true}
    }'
);

SELECT extversion AS pg_search_version
FROM pg_extension
WHERE extname = 'pg_search';
