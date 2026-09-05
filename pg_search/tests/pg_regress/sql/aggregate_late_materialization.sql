-- An aggregate over a join keeps its string group key deferred through the join
-- and checks visibility in the scan. A deleted row must not reach the groups,
-- serial or MPP.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

CREATE TABLE alm_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    price FLOAT
);
CREATE TABLE alm_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT
);

INSERT INTO alm_products (description, category, price) VALUES
    ('laptop fast', 'Electronics', 999.99),
    ('laptop gaming', 'Electronics', 1299.99),
    ('shoes running', 'Sports', 89.99),
    ('shoes trail', 'Sports', 119.99),
    ('jacket winter', 'Clothing', 129.99);
INSERT INTO alm_tags (product_id, tag_name) VALUES
    (1, 'tech'), (2, 'tech'), (3, 'fitness'), (4, 'fitness'), (5, 'outdoor');

CREATE INDEX alm_products_idx ON alm_products
USING bm25 (id, description, category, price)
WITH (key_field='id', text_fields='{"description": {}, "category": {"fast": true}}', numeric_fields='{"price": {"fast": true}}');
CREATE INDEX alm_tags_idx ON alm_tags
USING bm25 (id, product_id, tag_name)
WITH (key_field='id', numeric_fields='{"product_id": {"fast": true}}', text_fields='{"tag_name": {"fast": true}}');

-- Delete a matched row so visibility actually filters. The deleted product must
-- not appear in the aggregate.
DELETE FROM alm_products WHERE id = 2;

-- Serial.
SET max_parallel_workers_per_gather TO 0;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.category, COUNT(*)
FROM alm_products p JOIN alm_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, COUNT(*)
FROM alm_products p JOIN alm_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

-- Same result must hold on the MPP path.
SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

SELECT p.category, COUNT(*)
FROM alm_products p JOIN alm_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket'
GROUP BY p.category
ORDER BY p.category;

DROP TABLE alm_products, alm_tags;
