CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.max_term_agg_buckets TO 10;

DROP TABLE IF EXISTS products CASCADE;
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    rating INTEGER
);

INSERT INTO products (rating)
SELECT rating
FROM generate_series(1, 100) rating, generate_series(1, rating);

INSERT INTO products (rating)
VALUES (null);

CREATE INDEX products_idx ON products
USING bm25 (id, rating)
WITH (key_field='id');

-- These should not be pushed down

-- No LIMIT
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating;

-- Limit + offset exceeds max_term_agg_buckets
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 5 OFFSET 6;

-- Ordering on a non grouping column
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating, id
ORDER BY rating, id
LIMIT 5 OFFSET 5;

-- This should be pushed down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 5 OFFSET 5;

SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 5 OFFSET 5;

-- Ordering on a non-grouping column (need more buckets to sort correctly)
SET paradedb.max_term_agg_buckets TO 65000;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY 2
LIMIT 5;

SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY 2
LIMIT 5;

-- Reset for remaining tests
SET paradedb.max_term_agg_buckets TO 10;

-- Limit 0
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 0;

SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 0;

-- High limit (increase max_term_agg_buckets to allow more results)
SET paradedb.max_term_agg_buckets TO 65000;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 10000;

SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 10000;
