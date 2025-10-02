DROP TABLE IF EXISTS data_records;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE data_records (
    id SERIAL PRIMARY KEY,
    title TEXT,
    category TEXT,
    price NUMERIC,
    in_stock BOOLEAN,
    created_at TIMESTAMP,
    valid_period TSTZRANGE,
    quantity_range NUMRANGE,
    tags TEXT[]
);

INSERT INTO data_records (title, category, price, in_stock, created_at, valid_period, quantity_range, tags)
SELECT
    'Product ' || i,
    CASE WHEN i % 4 = 0 THEN 'Electronics'
         WHEN i % 4 = 1 THEN 'Clothing'
         WHEN i % 4 = 2 THEN 'Books'
         ELSE 'Home'
    END,
    (i * 1000)::numeric(10,2),
    i % 3 = 0,
    '2023-01-01'::timestamp + ((i % 365) || ' days')::interval,
    tstzrange(
        '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval,
        '2023-01-01'::timestamptz + ((i % 365) || ' days')::interval + '1 month'::interval
    ),
    numrange((i % 10) * 10, (i % 10 + 1) * 10),
    ARRAY[
        'tag' || (i % 5),
        'tag' || (i % 7),
        'tag' || (i % 3)
    ]
FROM generate_series(1, 20) i;

DROP INDEX IF EXISTS records_no_fast_idx;
CREATE INDEX records_no_fast_idx ON data_records
USING bm25 (
    id, title, category, price, in_stock, created_at, valid_period, quantity_range, tags
) WITH (
    key_field = 'id'
);

SELECT id, title, valid_period
FROM data_records
WHERE title @@@ 'product'
ORDER BY valid_period
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, valid_period
FROM data_records
WHERE title @@@ 'product'
ORDER BY valid_period
LIMIT 10;

SELECT id, title, quantity_range
FROM data_records
WHERE title @@@ 'product'
ORDER BY quantity_range
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, quantity_range
FROM data_records
WHERE title @@@ 'product'
ORDER BY quantity_range
LIMIT 10;

SELECT id, title, quantity_range, valid_period
FROM data_records
WHERE title @@@ 'product'
ORDER BY quantity_range, valid_period
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, quantity_range, valid_period
FROM data_records
WHERE title @@@ 'product'
ORDER BY quantity_range, valid_period
LIMIT 10;

SELECT id, title, price, valid_period
FROM data_records
WHERE title @@@ 'product'
ORDER BY price ASC, valid_period ASC
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, price, valid_period
FROM data_records
WHERE title @@@ 'product'
ORDER BY price ASC, valid_period ASC
LIMIT 10;

DROP TABLE data_records;
