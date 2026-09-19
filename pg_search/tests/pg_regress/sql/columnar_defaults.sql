-- Non-text fields are columnar by default.

-- Setup
\i common/common_setup.sql

-- Create test table with various field types
DROP TABLE IF EXISTS data_records;
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

-- Insert test data
INSERT INTO data_records (title, category, price, in_stock, created_at, valid_period, quantity_range, tags)
SELECT
    'Product ' || i,
    CASE WHEN i % 4 = 0 THEN 'Electronics'
         WHEN i % 4 = 1 THEN 'Clothing'
         WHEN i % 4 = 2 THEN 'Books'
         ELSE 'Home'
    END,
    (random() * 1000)::numeric(10,2),
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
FROM generate_series(1, 1000) i;


DROP INDEX IF EXISTS records_no_fast_idx;
CREATE INDEX records_no_fast_idx ON data_records
USING paradedb (id, (title::pdb.simple), (category::pdb.literal), price, in_stock, created_at, valid_period, quantity_range, (tags::pdb.literal));


-- 'Test 1: ORDER BY title with LIMIT should use NormalScanExecState'

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY title
LIMIT 10;

-- 'Test 2: ORDER BY with LIMIT (should use TopKScanExecState)'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY id
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY category
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY price
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY in_stock
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY valid_period
LIMIT 10;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, title, category
FROM data_records
WHERE title ||| 'product'
ORDER BY created_at
LIMIT 10;

-- 'Test 3: ORDER BY with no LIMIT (should use ColumnarExecState)'

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, category, price, in_stock, created_at, valid_period, quantity_range, tags
FROM data_records
WHERE title ||| 'product'
ORDER BY id;

DROP TABLE data_records;

\i common/common_cleanup.sql
