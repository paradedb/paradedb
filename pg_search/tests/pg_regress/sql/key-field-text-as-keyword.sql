DROP TABLE IF EXISTS public.key_field_text_raw;
CREATE TABLE public.key_field_text_raw
(
    id text not null primary key,
    data text
);

CREATE INDEX idx_key_field_raw on public.key_field_text_raw USING bm25 (id, data)
WITH (key_field = id, text_fields = '{"id": { "tokenizer": { "type": "keyword" } } }');
SELECT * FROM paradedb.schema('idx_key_field_raw') ORDER BY name;