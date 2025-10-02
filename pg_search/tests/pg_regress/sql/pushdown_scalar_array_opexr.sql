\i common/common_setup.sql

-- This flag is set to off by default, so we need to enable it to
-- test the pushdown of scalar array expressions.
SET paradedb.enable_custom_scan_without_operator = on;

CREATE TABLE scalar_array_pushdown(
    id SERIAL PRIMARY KEY,
    uuid_col UUID,
    text_col TEXT,
    int_col INTEGER,
    date_col DATE,
    ts_col TIMESTAMP
);

INSERT INTO scalar_array_pushdown(uuid_col, text_col, int_col, date_col, ts_col)
SELECT
    (ARRAY['550e8400-e29b-41d4-a716-446655440000', '550e8400-e29b-41d4-a716-446655440001', '550e8400-e29b-41d4-a716-446655440002', '550e8400-e29b-41d4-a716-446655440003', '550e8400-e29b-41d4-a716-446655440004'])[floor(random() * 5 + 1)]::uuid,
    (ARRAY['Alice', 'Bob', 'Charlie', 'David', 'Eve'])[i % 5 + 1],
    (i % 5)::integer,
    '2023-01-01'::date + ((i % 365) || ' days')::interval,
    '2023-01-01'::timestamp + ((i % 365) || ' days')::interval
FROM generate_series(1, 1000) i;

-- Non-keyword text fields are not pushed down
CREATE INDEX scalar_array_pushdown_idx ON scalar_array_pushdown USING bm25 (
    id, uuid_col, text_col, int_col, date_col, ts_col
) WITH (
    key_field = 'id',
    text_fields = '{"uuid_col": { "tokenizer": {"type": "whitespace"} } }'
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE uuid_col = ANY(ARRAY['550e8400-e29b-41d4-a716-446655440000'::uuid, '550e8400-e29b-41d4-a716-446655440001'::uuid])
ORDER BY id
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE text_col = ANY(ARRAY['Alice', 'Bob'])
ORDER BY id
LIMIT 10;

DROP INDEX scalar_array_pushdown_idx;

-- Now test pushdown
CREATE INDEX scalar_array_pushdown_idx ON scalar_array_pushdown USING bm25 (
    id, uuid_col, text_col, int_col, date_col, ts_col
) WITH (
    key_field = 'id',
    text_fields = '{"text_col": { "tokenizer": {"type": "keyword"} } }'
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE uuid_col = ANY(ARRAY['550e8400-e29b-41d4-a716-446655440000'::uuid, '550e8400-e29b-41d4-a716-446655440001'::uuid])
ORDER BY id
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE text_col = ANY(ARRAY['Alice', 'Bob'])
ORDER BY id
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE int_col = ANY(ARRAY[0, 1])
ORDER BY id
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE date_col = ANY(ARRAY['2023-01-01'::date, '2023-01-02'::date])
ORDER BY id
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE ts_col = ANY(ARRAY['2023-01-01'::timestamp, '2023-01-02'::timestamp])
ORDER BY id
LIMIT 10;

-- Test pushdown with other clauses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM scalar_array_pushdown
WHERE uuid_col = ANY(ARRAY['550e8400-e29b-41d4-a716-446655440000'::uuid, '550e8400-e29b-41d4-a716-446655440001'::uuid])
AND text_col IN ('Alice', 'Bob')
OR text_col @@@ 'Alice'
AND int_col > 2
ORDER BY id
LIMIT 10;

RESET paradedb.enable_custom_scan_without_operator;

DROP INDEX scalar_array_pushdown_idx;
DROP TABLE scalar_array_pushdown;
