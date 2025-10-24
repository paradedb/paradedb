\i common/common_setup.sql

CREATE TABLE numeric_pushdown(
    id SERIAL PRIMARY KEY,
    text_col TEXT,
    numeric_col NUMERIC,
    float_col FLOAT4,
    int_col INTEGER
);

INSERT INTO numeric_pushdown(text_col, numeric_col, float_col, int_col)
SELECT
    (ARRAY['Alice', 'Bob', 'Charlie', 'David', 'Eve'])[i % 5 + 1],
    (i % 5)::numeric,
    (i % 5)::float4,
    (i % 5)::integer
FROM generate_series(1, 100) i;

CREATE INDEX numeric_pushdown_idx ON numeric_pushdown USING bm25 (
    id, text_col, numeric_col, float_col, int_col
) WITH (key_field = 'id');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col = 1
ORDER BY id LIMIT 10;

SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col = 1
ORDER BY id LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col > 1
ORDER BY id LIMIT 10;

SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col > 1
ORDER BY id LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col::int > 1
ORDER BY id LIMIT 10;

SELECT * FROM numeric_pushdown
WHERE id @@@ paradedb.all()
AND numeric_col::int > 1
ORDER BY id LIMIT 10;

DROP TABLE numeric_pushdown;
