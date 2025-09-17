create table public.key_field_uuid_raw
(
    id uuid default gen_random_uuid() not null primary key,
    metadata_json jsonb DEFAULT '{}'::jsonb NOT NULL
);

CREATE INDEX test_search_index on public.key_field_uuid_raw
    USING bm25 (id, metadata_json)
    WITH (
    key_field = id,
    json_fields='{
"metadata_json": { "fast": true, "normalizer": "lowercase", "tokenizer": { "type": "raw" } },
"metadata_json_new": { "fast": true, "tokenizer": { "type": "keyword", "lowercase": true }, "column": "metadata_json" }
}', numeric_fields = '{}');
