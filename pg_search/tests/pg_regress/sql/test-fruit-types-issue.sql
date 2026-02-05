-- Test for "incompatible fruit types in tree" issue (#2963)

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = ON;

-- Create test table with mixed field types (exact reproduction)
CREATE TABLE users (
    id    SERIAL8 NOT NULL PRIMARY KEY,
    uuid  UUID NOT NULL,
    name  TEXT,
    color VARCHAR,
    age   INTEGER,
    price FLOAT,
    rating INTEGER
);

-- Create BM25 index with fast fields (exact reproduction)
CREATE INDEX idxusers ON users USING bm25 (id, uuid, name, color, age, price, rating)
WITH (
    key_field = 'id',
    text_fields = '{
        "uuid": { "tokenizer": { "type": "keyword" }, "fast": true },
        "name": { "tokenizer": { "type": "keyword" }, "fast": true },
        "color": { "tokenizer": { "type": "keyword" }, "fast": true }
    }',
    numeric_fields = '{
        "age": { "fast": true },
        "price": { "fast": true },
        "rating": { "fast": true }
    }'
);

-- Insert test data (deterministic version instead of gen_random_uuid)
INSERT INTO users (uuid, name, color, age, price, rating)
SELECT
    ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid,
    CASE (i % 3) WHEN 0 THEN 'alice' WHEN 1 THEN 'bob' ELSE 'charlie' END,
    'blue',
    20 + (i % 30),
    (100 + i * 10)::numeric(10,2),
    (i % 5) + 1
FROM generate_series(1, 100) AS i;

-- =====================================================================
-- Test the specific failing query from the issue (#2963)
-- =====================================================================
-- This query was supposed to trigger "incompatible fruit types in tree" error
-- if the issue still existed
EXPLAIN (VERBOSE, COSTS OFF)
SELECT name, SUM(price), MAX(rating), AVG(age) 
FROM users 
WHERE color @@@ 'blue' 
GROUP BY name;

-- execute the failing query
SELECT name, SUM(price), MAX(rating), AVG(age) 
FROM users 
WHERE color @@@ 'blue' 
GROUP BY name
ORDER BY name;

-- =====================================================================
-- Additional related test cases
-- =====================================================================

-- Test 4: COUNT + SUM + MAX (different combination)
EXPLAIN (VERBOSE, COSTS OFF)
SELECT name, COUNT(*), SUM(price), MAX(rating) 
FROM users 
WHERE color @@@ 'blue' 
GROUP BY name;

SELECT name, COUNT(*), SUM(price), MAX(rating) 
FROM users 
WHERE color @@@ 'blue' 
GROUP BY name
ORDER BY name;

-- Test 5: COUNT + MAX + AVG (different combination)
EXPLAIN (VERBOSE, COSTS OFF)
SELECT name, COUNT(*), MAX(rating), AVG(age) 
FROM users 
WHERE color @@@ 'blue' 
GROUP BY name;

SELECT name, COUNT(*), MAX(rating), AVG(age) 
FROM users 
WHERE color @@@ 'blue' 
GROUP BY name
ORDER BY name;

-- Clean up
DROP TABLE users;
