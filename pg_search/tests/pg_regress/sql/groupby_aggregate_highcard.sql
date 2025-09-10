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

CREATE INDEX products_idx ON products
USING bm25 (id, rating)
WITH (key_field='id');

-- This should show a warning about the maximum number of buckets/groups being exceeded
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating;

SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
OFFSET 50;

-- This should not be pushed down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 5 OFFSET 6;

SELECT rating, COUNT(*) FROM products
WHERE id @@@ paradedb.all()
GROUP BY rating
ORDER BY rating
LIMIT 5 OFFSET 6;

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
