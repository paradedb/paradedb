-- =====================================================================
-- pdb.agg() on the DataFusion aggregate backend
-- =====================================================================
-- Covers pdb.agg() over joins and over a single table routed to
-- DataFusion, with the Tantivy backend's output on the same data as the
-- reference where a query can run on both.

\i common/common_setup.sql

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS pa_products, pa_tags CASCADE;
CREATE TABLE pa_products (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    brand TEXT,
    price FLOAT,
    rating INTEGER,
    created_at TIMESTAMP,
    in_stock BOOLEAN,
    metadata JSONB,
    price_num NUMERIC(10, 2)
);
CREATE TABLE pa_tags (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    tag_name TEXT,
    weight INTEGER
);

CREATE INDEX pa_products_idx ON pa_products
USING paradedb (id, description, category, brand, price, rating, created_at, in_stock, metadata, price_num)
WITH (
    key_field = 'id',
    text_fields = '{"description": {}, "category": {"fast": true}, "brand": {"fast": true}, "cat_kw": {"column": "category", "fast": true, "tokenizer": {"type": "keyword"}}}',
    numeric_fields = '{"price": {"fast": true}, "rating": {"fast": true}, "price_num": {"fast": true}}',
    boolean_fields = '{"in_stock": {"fast": true}}',
    json_fields = '{"metadata": {"fast": true}}'
);

CREATE INDEX pa_tags_idx ON pa_tags
USING paradedb (id, product_id, tag_name, weight)
WITH (
    key_field = 'id',
    numeric_fields = '{"product_id": {"fast": true}, "weight": {"fast": true}}',
    text_fields = '{"tag_name": {"fast": true}}'
);

-- Two batches per table so each index has several segments, which the MPP
-- section needs to spread work across producers.
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO pa_products (description, category, brand, price, rating, created_at, in_stock, metadata, price_num) VALUES
    ('Laptop with fast processor', 'Electronics', 'Apple', 1299.99, 5, '2024-01-01 10:00:00', true, '{"color": "silver", "qty": 3}', 1299.99),
    ('Gaming laptop with RGB', 'Electronics', 'Dell', 1499.99, 5, '2024-01-02 10:00:00', false, '{"color": "black", "qty": 1}', 1499.99),
    ('Budget laptop', 'Electronics', 'HP', 499.99, 3, '2024-01-03 10:00:00', true, '{"color": "black", "qty": 7}', 499.99),
    ('Wireless keyboard', 'Electronics', 'Logitech', 79.99, 4, '2024-01-04 10:00:00', true, '{"color": "black", "qty": 9}', 79.99);
INSERT INTO pa_products (description, category, brand, price, rating, created_at, in_stock, metadata, price_num) VALUES
    ('Running shoes', 'Sports', 'Nike', 89.99, 5, '2024-01-05 10:00:00', true, '{"color": "red", "qty": 2}', 89.99),
    ('Basketball shoes', 'Sports', 'Adidas', 119.99, 4, '2024-01-06 10:00:00', false, '{"color": "red", "qty": 1}', 119.99),
    ('Winter jacket', 'Clothing', 'North Face', 199.99, 4, '2024-01-07 10:00:00', true, '{"color": "blue", "qty": 4}', 199.99),
    ('Toy laptop', 'Toys', 'Fisher Price', 29.99, NULL, '2024-01-08 10:00:00', true, NULL, NULL);

INSERT INTO pa_tags (product_id, tag_name, weight) VALUES
    (1, 'tech', 10), (1, 'computer', 5),
    (2, 'tech', 10), (2, 'gaming', 7),
    (3, 'tech', 10), (3, 'budget', 1),
    (4, 'tech', 3);
INSERT INTO pa_tags (product_id, tag_name, weight) VALUES
    (5, 'fitness', 4), (5, 'running', 6),
    (6, 'fitness', 4),
    (7, 'outdoor', 2),
    (8, 'tech', 1), (8, 'kids', 2);

RESET paradedb.global_mutable_segment_rows;

-- Row estimates feed the MPP planner; unanalyzed tables leave them unknown.
ANALYZE pa_products;
ANALYZE pa_tags;

-- =====================================================================
-- SECTION 1: terms over a join
-- =====================================================================

-- Test 1.1: terms per SQL group
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, pdb.agg('{"terms": {"field": "tag_name"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, pdb.agg('{"terms": {"field": "tag_name"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 1.2: scalar query, nested terms with metric sub-aggregations
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "category"}, "aggs": {"tags": {"terms": {"field": "tag_name", "size": 2}, "aggs": {"w": {"sum": {"field": "weight"}}}}, "uniq_brands": {"cardinality": {"field": "brand"}}, "avg_price": {"avg": {"field": "price"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard';

SELECT pdb.agg('{"terms": {"field": "category"}, "aggs": {"tags": {"terms": {"field": "tag_name", "size": 2}, "aggs": {"w": {"sum": {"field": "weight"}}}}, "uniq_brands": {"cardinality": {"field": "brand"}}, "avg_price": {"avg": {"field": "price"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard';

-- Test 1.3: standard aggregates alongside pdb.agg(), with HAVING
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*), SUM(t.weight), pdb.agg('{"terms": {"field": "tag_name"}}') AS tags
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;

SELECT p.category, COUNT(*), SUM(t.weight), pdb.agg('{"terms": {"field": "tag_name"}}') AS tags
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard'
GROUP BY p.category
HAVING COUNT(*) > 1
ORDER BY p.category;

-- Test 1.4: order by a metric sub-aggregation, min_doc_count, size
SELECT pdb.agg('{"terms": {"field": "tag_name", "order": {"total_w": "desc"}, "min_doc_count": 2, "size": 2}, "aggs": {"total_w": {"sum": {"field": "weight"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard';

-- Test 1.5: per-aggregate FILTER
SELECT p.category, pdb.agg('{"terms": {"field": "tag_name"}}') FILTER (WHERE t.weight > 5)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard'
GROUP BY p.category
ORDER BY p.category;

-- Test 1.6: bool and datetime keys, datetime metrics
SELECT pdb.agg('{"terms": {"field": "in_stock"}, "aggs": {"first": {"min": {"field": "created_at"}}, "last": {"max": {"field": "created_at"}}}}'),
       pdb.agg('{"terms": {"field": "created_at", "size": 3, "order": {"_key": "asc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.7: missing on a numeric key
SELECT pdb.agg('{"terms": {"field": "rating", "missing": 0, "order": {"_key": "desc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.8: ORDER BY an aggregate with LIMIT stays correct without TopK pushdown
SELECT p.category, COUNT(*), pdb.agg('{"terms": {"field": "tag_name"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard'
GROUP BY p.category
ORDER BY COUNT(*) DESC, p.category
LIMIT 2;

-- Test 1.9: a NULL key gets its own bucket. The Tantivy backend gives it one
-- too through its `missing` sentinel, rendered as `null` for text keys and
-- leaked as an extreme value for numeric ones.
SELECT pdb.agg('{"terms": {"field": "rating", "order": {"_key": "asc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.10: a `missing` literal takes the column's type
SELECT pdb.agg('{"terms": {"field": "tag_name", "missing": 0}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT pdb.agg('{"terms": {"field": "rating", "missing": "none"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.11: cardinality on a float field counts exactly; bool and datetime
-- take the HLL
SELECT pdb.agg('{"cardinality": {"field": "price"}}'),
       pdb.agg('{"cardinality": {"field": "in_stock"}}'),
       pdb.agg('{"cardinality": {"field": "created_at"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.12: an aliased index field resolves by its index name
SELECT pdb.agg('{"terms": {"field": "cat_kw", "order": {"_key": "asc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.13: `min_doc_count: 0` has no grouped-scan equivalent, and `size: 0`
-- keeps no buckets
SELECT pdb.agg('{"terms": {"field": "tag_name", "min_doc_count": 0}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT pdb.agg('{"terms": {"field": "tag_name", "size": 0}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 1.14: a self-join resolves each side through its alias
SELECT pdb.agg('{"terms": {"field": "a.category"}}'), pdb.agg('{"cardinality": {"field": "b.brand"}}')
FROM pa_products a JOIN pa_products b ON a.category = b.category
WHERE a.description @@@ 'laptop';

-- Test 1.15: a LEFT JOIN's unmatched side lands in the NULL bucket
SELECT pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}')
FROM pa_products p LEFT JOIN pa_tags t ON p.id = t.product_id AND t.weight > 5
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.16: terms on a text-valued JSON sub-field, alone and under a JSON
-- GROUP BY expression
SELECT pdb.agg('{"terms": {"field": "metadata.color", "order": {"_key": "asc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

SELECT p.metadata->>'color' AS color, pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.metadata->>'color'
ORDER BY color;

-- A JSON sub-field holding numbers is turned down at plan time
SELECT pdb.agg('{"terms": {"field": "metadata.qty"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.17: NUMERIC fields as metrics and as terms keys
SELECT pdb.agg('{"terms": {"field": "category", "order": {"_key": "asc"}}, "aggs": {"total": {"sum": {"field": "price_num"}}, "mean": {"avg": {"field": "price_num"}}, "lo": {"min": {"field": "price_num"}}, "hi": {"max": {"field": "price_num"}}, "n": {"cardinality": {"field": "price_num"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

SELECT pdb.agg('{"terms": {"field": "price_num", "order": {"_key": "desc"}, "size": 3}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes';

-- Test 1.18: terms on the GROUP BY column itself, and two GROUP BY columns
SELECT p.category, pdb.agg('{"terms": {"field": "category"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

SELECT p.category, p.in_stock, pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category, p.in_stock
ORDER BY p.category, p.in_stock;

-- =====================================================================
-- SECTION 2: empty inputs
-- =====================================================================

-- Test 2.1: scalar query over no rows answers with one row
SELECT pdb.agg('{"terms": {"field": "tag_name"}}'), pdb.agg('{"sum": {"field": "weight"}}'), pdb.agg('{"avg": {"field": "weight"}}'), COUNT(*)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent';

-- Test 2.2: grouped query over no rows answers with no rows
SELECT p.category, pdb.agg('{"terms": {"field": "tag_name"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent'
GROUP BY p.category;

-- Test 2.3: HAVING judges the scalar row, including the one made for an
-- empty input
SELECT pdb.agg('{"terms": {"field": "tag_name"}}'), COUNT(*)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent'
HAVING COUNT(*) > 0;

SELECT pdb.agg('{"terms": {"field": "tag_name"}}'), COUNT(*)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'nonexistent'
HAVING COUNT(*) = 0;

SELECT pdb.agg('{"sum": {"field": "weight"}}'), COUNT(*)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop'
HAVING COUNT(*) < 3;

SELECT pdb.agg('{"sum": {"field": "weight"}}'), COUNT(*)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop'
HAVING COUNT(*) > 3;

-- =====================================================================
-- SECTION 3: field resolution and visibility
-- =====================================================================

-- Test 3.1: a field name present in both tables must be qualified
SELECT pdb.agg('{"cardinality": {"field": "id"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

SELECT pdb.agg('{"cardinality": {"field": "p.id"}}'), pdb.agg('{"value_count": {"field": "t.id"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 3.2: an aggregation the backend does not run is an error, not a fallback
SELECT pdb.agg('{"range": {"field": "price", "ranges": [{"to": 100}]}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- Test 3.3: raw visibility skips the heap check; nothing is deleted here so the
-- result matches
SELECT p.category, pdb.agg('{"terms": {"field": "tag_name"}}'::jsonb, 'raw')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes'
GROUP BY p.category
ORDER BY p.category;

-- Test 3.4: raw visibility sees a row deleted in this transaction
BEGIN;
DELETE FROM pa_tags WHERE tag_name = 'gaming';
SELECT pdb.agg('{"terms": {"field": "tag_name"}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';
SELECT pdb.agg('{"terms": {"field": "tag_name"}}'::jsonb, 'raw')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';
ROLLBACK;

-- Test 3.5: `threshold` is one decision for the whole join, judged on the
-- largest table's estimate
BEGIN;
DELETE FROM pa_tags WHERE tag_name = 'gaming';
DELETE FROM pa_products WHERE brand = 'HP';
SET LOCAL paradedb.visibility_threshold TO 10;
SELECT pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}'::jsonb, 'threshold'),
       pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}'::jsonb, 'threshold')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';
SET LOCAL paradedb.visibility_threshold TO 1000;
SELECT pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}'::jsonb, 'threshold'),
       pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}'::jsonb, 'threshold')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';
ROLLBACK;

-- Test 3.6: conflicting visibility settings are rejected
SELECT pdb.agg('{"sum": {"field": "weight"}}'::jsonb, 'raw'), pdb.agg('{"sum": {"field": "price"}}'::jsonb)
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop';

-- =====================================================================
-- SECTION 4: single table routed to DataFusion
-- =====================================================================

-- Test 4.1: reference result on the Tantivy backend. A nested `aggs` under
-- GROUP BY does not run there, so the metric is a second call.
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}'), pdb.agg('{"sum": {"field": "price"}}'), COUNT(*)
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

SELECT category, pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}'), pdb.agg('{"sum": {"field": "price"}}'), COUNT(*)
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

-- Test 4.2: the same query on DataFusion, forced through the bucket cap
SET paradedb.max_term_agg_buckets TO 1;

EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}'), pdb.agg('{"sum": {"field": "price"}}'), COUNT(*)
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

SELECT category, pdb.agg('{"terms": {"field": "brand", "order": {"_key": "asc"}}}'), pdb.agg('{"sum": {"field": "price"}}'), COUNT(*)
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

-- Test 4.3: a spec that does not lower stays on the single-table path, whether
-- the aggregation kind or a field is the reason
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, pdb.agg('{"range": {"field": "price", "ranges": [{"to": 100}]}}')
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, pdb.agg('{"sum": {"field": "metadata.qty"}}')
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

SELECT category, pdb.agg('{"sum": {"field": "metadata.qty"}}')
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

RESET paradedb.max_term_agg_buckets;

-- Test 4.4: a NUMERIC field in the spec routes a single-table query to
-- DataFusion, since Tantivy cannot aggregate the decimal storage
EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT category, pdb.agg('{"sum": {"field": "price_num"}}'), pdb.agg('{"terms": {"field": "price_num", "order": {"_key": "asc"}}}')
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

SELECT category, pdb.agg('{"sum": {"field": "price_num"}}'), pdb.agg('{"terms": {"field": "price_num", "order": {"_key": "asc"}}}')
FROM pa_products
WHERE description @@@ 'laptop OR shoes'
GROUP BY category
ORDER BY category;

-- =====================================================================
-- SECTION 5: MPP
-- =====================================================================
-- Grouping sets and the HLL sketch both cross the worker boundary. The plan is
-- printed with workers off, so its shape does not depend on the machine; the
-- queries then run with workers on.

SET paradedb.enable_join_custom_scan TO on;
SET paradedb.mpp_min_rows TO 0;

EXPLAIN (FORMAT TEXT, COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.category, COUNT(*), pdb.agg('{"terms": {"field": "tag_name"}, "aggs": {"u": {"cardinality": {"field": "weight"}}, "s": {"sum": {"field": "weight"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard'
GROUP BY p.category
ORDER BY p.category;

SET max_parallel_workers_per_gather TO 3;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

SELECT p.category, COUNT(*), pdb.agg('{"terms": {"field": "tag_name"}, "aggs": {"u": {"cardinality": {"field": "weight"}}, "s": {"sum": {"field": "weight"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard'
GROUP BY p.category
ORDER BY p.category;

SELECT pdb.agg('{"terms": {"field": "category"}, "aggs": {"u": {"cardinality": {"field": "brand"}}}}')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard';

-- The leader's visibility decision reaches the workers
BEGIN;
DELETE FROM pa_tags WHERE tag_name = 'gaming';
SELECT pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}'::jsonb, 'transaction')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard';
SELECT pdb.agg('{"terms": {"field": "tag_name", "order": {"_key": "asc"}}}'::jsonb, 'raw')
FROM pa_products p JOIN pa_tags t ON p.id = t.product_id
WHERE p.description @@@ 'laptop OR shoes OR jacket OR keyboard';
ROLLBACK;

DROP TABLE pa_products, pa_tags CASCADE;
