-- Tests for SELECT DISTINCT pushdown into the BM25 index.
--
-- Plain SELECT DISTINCT on a fast field maps to a GROUP BY without
-- aggregates. The AggregateScan hook fires at UPPERREL_DISTINCT in
-- addition to UPPERREL_GROUP_AGG. DISTINCT always routes to DataFusion:
-- no bucket cap, correct NULL semantics.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- Setup
-- =============================================================================

DROP TABLE IF EXISTS dist_products CASCADE;

CREATE TABLE dist_products (
    id       SERIAL PRIMARY KEY,
    name     TEXT,
    category TEXT,
    brand    TEXT,
    rating   INTEGER,
    price    FLOAT,
    in_stock BOOLEAN
);

INSERT INTO dist_products (name, category, brand, rating, price, in_stock)
VALUES
    ('Laptop Pro',            'Electronics', 'BrandA', 5, 999.99,  true),
    ('Gaming Laptop',         'Electronics', 'BrandB', 4, 1299.99, true),
    ('Office Laptop',         'Electronics', 'BrandA', 3, 799.99,  false),
    ('Wireless Keyboard',     'Accessories', 'BrandC', 4, 79.99,   true),
    ('Mechanical Keyboard',   'Accessories', 'BrandA', 5, 149.99,  true),
    ('Running Shoes',         'Sports',      'BrandD', 5, 89.99,   true),
    ('Winter Jacket',         'Clothing',    'BrandE', 4, 129.99,  true),
    ('Summer Jacket',         'Clothing',    'BrandE', 3, 59.99,   true),
    ('Noise Headphones',      'Electronics', 'BrandB', 5, 299.99,  true),
    ('Budget Headphones',     'Electronics', 'BrandC', 3, 49.99,   true),
    ('Yoga Mat',              'Sports',      'BrandD', 4, 39.99,   true),
    ('Desk Chair',            'Furniture',   'BrandF', 5, 499.99,  true),
    ('Standing Desk',         'Furniture',   'BrandF', 4, 699.99,  false),
    ('USB Hub',               'Accessories', 'BrandC', 4, 29.99,   true),
    ('Monitor 4K',            'Electronics', 'BrandB', 5, 599.99,  true);

-- Index: category and brand are fast text fields; name is not fast;
-- rating and price are fast numeric fields.
CREATE INDEX dist_products_idx ON dist_products
USING bm25 (id, name, category, brand, rating, price)
WITH (
    key_field = 'id',
    text_fields  = '{"name": {}, "category": {"fast": true}, "brand": {"fast": true}}',
    numeric_fields = '{"rating": {"fast": true}, "price": {"fast": true}}'
);

ANALYZE dist_products;

-- =============================================================================
-- TEST 1: Basic DISTINCT on a fast text field
-- Expect: Custom Scan (ParadeDB Aggregate Scan) in the EXPLAIN plan.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR jacket OR shoes'
ORDER BY category;

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR jacket OR shoes'
ORDER BY category;

-- Correctness: must match native PG (scan without custom scan)
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR jacket OR shoes'
ORDER BY category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 2: DISTINCT on a fast numeric field
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT rating
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY rating;

SELECT DISTINCT rating
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY rating;

-- =============================================================================
-- TEST 3: DISTINCT with ORDER BY + LIMIT
-- Exercises the TopK / LIMIT-aware code path inside AggregateScan.
--
-- The EXPLAIN below uses an integer sort key: group-column TopK is eligible
-- for non-collatable keys in every environment, so the pushed-down plan is
-- deterministic. (For text keys, eligibility depends on the database collation
-- being byte-ordered, which is environment-specific and not asserted here.)
-- =============================================================================

-- Integer sort key: TopK pushes down (SortExec TopK + lim=[K]).
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT rating
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR jacket OR shoes OR headphones OR hub OR monitor OR mat OR chair OR desk'
ORDER BY rating
LIMIT 2;

SELECT DISTINCT rating
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR jacket OR shoes OR headphones OR hub OR monitor OR mat OR chair OR desk'
ORDER BY rating
LIMIT 2;

-- Text sort key + LIMIT: result checked against native PG (no EXPLAIN — text
-- TopK eligibility depends on the database collation).
SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR jacket OR shoes OR headphones OR hub OR monitor OR mat OR chair OR desk'
ORDER BY category
LIMIT 3;

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR jacket OR shoes OR headphones OR hub OR monitor OR mat OR chair OR desk'
ORDER BY category
LIMIT 3;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 4: Multi-column DISTINCT
-- DISTINCT on (category, brand) - both are fast fields.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category, brand
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
ORDER BY category, brand;

SELECT DISTINCT category, brand
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
ORDER BY category, brand;

-- Correctness: compare with native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category, brand
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
ORDER BY category, brand;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 5: DISTINCT on a non-fast field - must fall back gracefully
-- 'name' has no fast:true, so the custom scan cannot push down the
-- deduplication. It should fall back to native PG without an error.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT name
FROM dist_products
WHERE name @@@ 'laptop'
ORDER BY name;

SELECT DISTINCT name
FROM dist_products
WHERE name @@@ 'laptop'
ORDER BY name;

-- With paradedb.check_aggregate_scan = false the decline warning must be
-- suppressed; the query still runs correctly via native PG.
SET paradedb.check_aggregate_scan = false;

SELECT DISTINCT name
FROM dist_products
WHERE name @@@ 'laptop'
ORDER BY name;

RESET paradedb.check_aggregate_scan;

-- =============================================================================
-- TEST 6: DISTINCT ON - must fall back gracefully (DISTINCT ON is not supported)
-- DISTINCT ON is not a plain GROUP BY and cannot be modelled as an aggregate.
-- The planner should decline and fall back to native PG execution.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT ON (category) category, name
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY category, name;

SELECT DISTINCT ON (category) category, name
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY category, name;

-- With paradedb.check_aggregate_scan = false the decline warning must be
-- suppressed; the query still runs correctly via native PG.
SET paradedb.check_aggregate_scan = false;

SELECT DISTINCT ON (category) category, name
FROM dist_products
WHERE name @@@ 'laptop OR keyboard'
ORDER BY category, name;

RESET paradedb.check_aggregate_scan;

-- =============================================================================
-- TEST 7: DISTINCT with empty result set
-- WHERE clause matches nothing; DISTINCT must return zero rows.
-- =============================================================================

SELECT DISTINCT category
FROM dist_products
WHERE name @@@ 'nonexistent_xyz_term'
ORDER BY category;

-- =============================================================================
-- TEST 8: DISTINCT across entire table (no WHERE clause with @@@)
-- With no BM25 search predicate the aggregate scan still claims the query
-- via an `all` query, consistent with GROUP BY pushdown.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category
FROM dist_products
ORDER BY category;

SELECT DISTINCT category
FROM dist_products
ORDER BY category;

-- =============================================================================
-- TEST 9: SELECT DISTINCT on a JOIN — DataFusion path
-- =============================================================================

DROP TABLE IF EXISTS dist_regions CASCADE;
CREATE TABLE dist_regions (
    id       SERIAL PRIMARY KEY,
    category TEXT,
    region   TEXT
);

INSERT INTO dist_regions (category, region) VALUES
    ('Electronics', 'North'),
    ('Accessories', 'South'),
    ('Sports',      'North'),
    ('Clothing',    'East'),
    ('Furniture',   'West');

CREATE INDEX dist_regions_idx ON dist_regions
USING bm25 (id, category, region)
WITH (
    key_field = 'id',
    text_fields = '{"category": {"fast": true}, "region": {"fast": true}}'
);

ANALYZE dist_regions;

-- Disable JoinScan so AggregateScan (DataFusion) owns the DISTINCT deduplication.
SET paradedb.enable_join_custom_scan TO off;

-- EXPLAIN: should show Custom Scan (ParadeDB Aggregate Scan) with Backend: DataFusion.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
ORDER BY dp.category;

SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
ORDER BY dp.category;

-- Correctness: results must match native PG.
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
ORDER BY dp.category;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

DROP TABLE IF EXISTS dist_regions CASCADE;

-- =============================================================================
-- TEST 10: SELECT DISTINCT on a pdb.literal (raw tokenizer) fast field
-- =============================================================================

DROP TABLE IF EXISTS dist_urls CASCADE;

CREATE TABLE dist_urls (
    id   SERIAL PRIMARY KEY,
    name TEXT,
    url  TEXT
);

INSERT INTO dist_urls (name, url) VALUES
    ('Page A', 'https://example.com/a'),
    ('Page B', 'https://example.com/b'),
    ('Page C', 'https://example.com/a'),
    ('Page D', 'https://example.com/c'),
    ('Page E', 'https://example.com/b');

-- Index the url column using the pdb.literal cast expression (raw tokenizer, fast field).
CREATE INDEX dist_urls_idx ON dist_urls
USING bm25 (id, name, (url::pdb.literal))
WITH (key_field = 'id');

ANALYZE dist_urls;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT url
FROM dist_urls
WHERE name @@@ 'Page'
ORDER BY url;

SELECT DISTINCT url
FROM dist_urls
WHERE name @@@ 'Page'
ORDER BY url;

-- Correctness: must match native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT url
FROM dist_urls
WHERE name @@@ 'Page'
ORDER BY url;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS dist_urls CASCADE;

-- =============================================================================
-- TEST 11: NULL semantics — DISTINCT on a nullable fast field
-- =============================================================================

DROP TABLE IF EXISTS dist_nullable CASCADE;

CREATE TABLE dist_nullable (
    id       SERIAL PRIMARY KEY,
    name     TEXT,
    category TEXT        -- nullable: some rows will have NULL
);

INSERT INTO dist_nullable (name, category) VALUES
    ('Laptop',    'Electronics'),
    ('Keyboard',  'Accessories'),
    ('Widget',    NULL),              -- NULL category
    ('Gadget',    'Electronics'),
    ('Thingamajig', NULL);            -- second NULL category

CREATE INDEX dist_nullable_idx ON dist_nullable
USING bm25 (id, name, category)
WITH (
    key_field   = 'id',
    text_fields = '{"name": {}, "category": {"fast": true}}'
);

ANALYZE dist_nullable;

-- EXPLAIN must show Backend: DataFusion (always-DataFusion for DISTINCT).
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category
FROM dist_nullable
WHERE name @@@ 'laptop OR keyboard OR widget OR gadget OR thingamajig'
ORDER BY category;

-- DISTINCT must return: Accessories, Electronics, NULL (three rows)
SELECT DISTINCT category
FROM dist_nullable
WHERE name @@@ 'laptop OR keyboard OR widget OR gadget OR thingamajig'
ORDER BY category;

-- NULL rows must be included; expected count is 2.
SELECT COUNT(*)
FROM dist_nullable
WHERE name @@@ 'laptop OR keyboard OR widget OR gadget OR thingamajig'
  AND category IS NULL;

-- Correctness: compare with native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category
FROM dist_nullable
WHERE name @@@ 'laptop OR keyboard OR widget OR gadget OR thingamajig'
ORDER BY category;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS dist_nullable CASCADE;

-- =============================================================================
-- TEST 12: High-cardinality DISTINCT — DataFusion returns all distinct values
--
-- max_term_agg_buckets is pinned low so the test is independent of the GUC
-- default. All 20 distinct values must be returned.
-- =============================================================================

SET paradedb.max_term_agg_buckets = 10;

DROP TABLE IF EXISTS dist_highcard CASCADE;

CREATE TABLE dist_highcard (
    id   SERIAL PRIMARY KEY,
    name TEXT,
    url  TEXT
);

INSERT INTO dist_highcard (name, url)
SELECT
    'Page ' || i,
    'https://example.com/page/' || i
FROM generate_series(1, 20) AS i;

CREATE INDEX dist_highcard_idx ON dist_highcard
USING bm25 (id, name, (url::pdb.literal))
WITH (key_field = 'id');

ANALYZE dist_highcard;

-- EXPLAIN must show Backend: DataFusion (DISTINCT always routes to DataFusion).
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT url
FROM dist_highcard
WHERE name @@@ 'Page'
ORDER BY url;

-- Verify all 20 distinct values are returned, not just 10.
SELECT COUNT(*)
FROM (
    SELECT DISTINCT url
    FROM dist_highcard
    WHERE name @@@ 'Page'
) t;

RESET paradedb.max_term_agg_buckets;

DROP TABLE IF EXISTS dist_highcard CASCADE;

-- =============================================================================
-- TEST 13: DISTINCT on an expression — declines gracefully to native PG
-- =============================================================================

DROP TABLE IF EXISTS dist_expr CASCADE;

CREATE TABLE dist_expr (
    id       SERIAL PRIMARY KEY,
    name     TEXT,
    category TEXT,
    brand    TEXT
);

INSERT INTO dist_expr (name, category, brand) VALUES
    ('Laptop Pro',     'Electronics', 'BrandA'),
    ('Gaming Laptop',  'electronics', 'BrandB'),
    ('Office Laptop',  'Electronics', 'BrandA'),
    ('Wireless Mouse', 'accessories', 'BrandC'),
    ('USB Hub',        'Accessories', 'BrandC');

CREATE INDEX dist_expr_idx ON dist_expr
USING bm25 (id, name, category, brand)
WITH (
    key_field = 'id',
    text_fields = '{"name": {}, "category": {"fast": true}, "brand": {"fast": true}}'
);

ANALYZE dist_expr;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT upper(category)
FROM dist_expr
WHERE name @@@ 'laptop OR mouse OR hub'
ORDER BY 1;

-- Expected: 2 rows — 'ACCESSORIES', 'ELECTRONICS'.
SELECT DISTINCT upper(category)
FROM dist_expr
WHERE name @@@ 'laptop OR mouse OR hub'
ORDER BY 1;

-- Correctness: compare with native PG.
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT upper(category)
FROM dist_expr
WHERE name @@@ 'laptop OR mouse OR hub'
ORDER BY 1;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Concatenation: 4 distinct rows from mixed-case category combined with brand.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT category || '-' || brand
FROM dist_expr
WHERE name @@@ 'laptop OR mouse OR hub'
ORDER BY 1;

-- Expected: 4 rows — 'Accessories-BrandC', 'Electronics-BrandA',
-- 'accessories-BrandC', 'electronics-BrandB'.
SELECT DISTINCT category || '-' || brand
FROM dist_expr
WHERE name @@@ 'laptop OR mouse OR hub'
ORDER BY 1;

SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT category || '-' || brand
FROM dist_expr
WHERE name @@@ 'laptop OR mouse OR hub'
ORDER BY 1;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS dist_expr CASCADE;

-- =============================================================================
-- TEST 14: DISTINCT and GROUP BY together (single table, Tantivy path)
-- GROUP BY output is already unique per group, so the DISTINCT is a semantic
-- no-op, but the shape must still push the GROUP BY down with PG's Unique
-- planned above the grouped output.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT brand, COUNT(*)
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
GROUP BY brand
ORDER BY brand;

SELECT DISTINCT brand, COUNT(*)
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
GROUP BY brand
ORDER BY brand;

-- Correctness: compare with native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT brand, COUNT(*)
FROM dist_products
WHERE name @@@ 'laptop OR keyboard OR headphones'
GROUP BY brand
ORDER BY brand;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 15: JOIN + DISTINCT ON over GROUP BY — GROUP BY pushdown is preserved
-- DISTINCT ON is only rejected at the DISTINCT stage. The GROUP BY stage must
-- still push down to DataFusion, with PG running the DISTINCT ON above.
-- =============================================================================

DROP TABLE IF EXISTS dist_regions CASCADE;
CREATE TABLE dist_regions (
    id       SERIAL PRIMARY KEY,
    category TEXT,
    region   TEXT
);

INSERT INTO dist_regions (category, region) VALUES
    ('Electronics', 'North'),
    ('Accessories', 'South'),
    ('Sports',      'North'),
    ('Clothing',    'East'),
    ('Furniture',   'West');

CREATE INDEX dist_regions_idx ON dist_regions
USING bm25 (id, category, region)
WITH (
    key_field = 'id',
    text_fields = '{"category": {"fast": true}, "region": {"fast": true}}'
);

ANALYZE dist_regions;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT ON (dp.category) dp.category, COUNT(*)
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
GROUP BY dp.category
ORDER BY dp.category;

SELECT DISTINCT ON (dp.category) dp.category, COUNT(*)
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
GROUP BY dp.category
ORDER BY dp.category;

-- Correctness: compare with native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT ON (dp.category) dp.category, COUNT(*)
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
GROUP BY dp.category
ORDER BY dp.category;

SET paradedb.enable_aggregate_custom_scan TO on;

-- =============================================================================
-- TEST 16: JOIN + DISTINCT without LIMIT under enable_join_custom_scan = on
-- JoinScan requires a top-level LIMIT, so it declines this shape and builds
-- no path. AggregateScan must still claim it: the deferral asks whether a
-- JoinScan path exists on the joinrel, not whether JoinScan is enabled.
-- =============================================================================

SET paradedb.enable_join_custom_scan TO on;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
ORDER BY dp.category;

SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
ORDER BY dp.category;

-- =============================================================================
-- TEST 17: JOIN + DISTINCT + LIMIT that JoinScan declines for its own reason
-- A volatile predicate makes JoinScan reject the LIMIT pushdown, so no
-- JoinScan path reaches the joinrel. AggregateScan must pick the shape up
-- instead of letting it fall back to native PG.
-- =============================================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
  AND dp.id + random() > 0
ORDER BY dp.category
LIMIT 10;

SELECT DISTINCT dp.category
FROM dist_products dp
JOIN dist_regions dr ON dp.category = dr.category
WHERE dp.name @@@ 'laptop OR jacket OR shoes'
  AND dp.id + random() > 0
ORDER BY dp.category
LIMIT 10;

RESET paradedb.enable_join_custom_scan;

DROP TABLE IF EXISTS dist_regions CASCADE;

-- =============================================================================
-- TEST 18: type-changing cast — declines to native PG
-- The fast field holds the bigint, the query asks for its text rendering.
-- Grouping the raw field would hand the tuple slot the wrong type.
-- =============================================================================

DROP TABLE IF EXISTS dist_cast CASCADE;

CREATE TABLE dist_cast (
    id   SERIAL PRIMARY KEY,
    name TEXT,
    n    BIGINT
);

INSERT INTO dist_cast (name, n) VALUES
    ('alpha', 1),
    ('beta',  2),
    ('gamma', 1);

CREATE INDEX dist_cast_idx ON dist_cast
USING bm25 (id, name, n)
WITH (
    key_field = 'id',
    text_fields = '{"name": {}}',
    numeric_fields = '{"n": {"fast": true}}'
);

ANALYZE dist_cast;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT n::text
FROM dist_cast
WHERE id @@@ paradedb.all()
ORDER BY 1;

SELECT DISTINCT n::text
FROM dist_cast
WHERE id @@@ paradedb.all()
ORDER BY 1;

-- Correctness: must match native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT n::text
FROM dist_cast
WHERE id @@@ paradedb.all()
ORDER BY 1;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS dist_cast CASCADE;

-- =============================================================================
-- TEST 19: JSON container — declines to native PG
-- The fast field indexes the array elements, the query deduplicates whole
-- arrays. Grouping on the elements would collapse ["a"] and ["a","b"].
-- =============================================================================

DROP TABLE IF EXISTS dist_json CASCADE;

CREATE TABLE dist_json (
    id   SERIAL PRIMARY KEY,
    name TEXT,
    meta JSONB
);

INSERT INTO dist_json (name, meta) VALUES
    ('alpha', '{"tags": ["a", "b"]}'),
    ('beta',  '{"tags": ["a", "b"]}'),
    ('gamma', '{"tags": ["a"]}');

CREATE INDEX dist_json_idx ON dist_json
USING bm25 (id, name, meta)
WITH (
    key_field = 'id',
    text_fields = '{"name": {}}',
    json_fields = '{"meta": {"fast": true}}'
);

ANALYZE dist_json;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT meta->'tags'
FROM dist_json
WHERE id @@@ paradedb.all()
ORDER BY 1;

SELECT DISTINCT meta->'tags'
FROM dist_json
WHERE id @@@ paradedb.all()
ORDER BY 1;

-- Correctness: must match native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT DISTINCT meta->'tags'
FROM dist_json
WHERE id @@@ paradedb.all()
ORDER BY 1;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS dist_json CASCADE;

-- =============================================================================
-- TEST 20: nondeterministic collation — declines to native PG
-- A case-insensitive ICU collation calls 'a' and 'A' equal. The aggregation
-- backend groups by bytes and would keep them apart, and no node above the
-- scan can merge groups it already emitted separately. Which of the two
-- survives the dedup is up to Postgres, so this asserts the count.
-- =============================================================================

DROP TABLE IF EXISTS dist_ci CASCADE;

CREATE COLLATION IF NOT EXISTS dist_ci_coll (
    provider = icu,
    locale = 'und-u-ks-level2',
    deterministic = false
);

CREATE TABLE dist_ci (
    id    SERIAL PRIMARY KEY,
    name  TEXT,
    value TEXT COLLATE dist_ci_coll
);

INSERT INTO dist_ci (name, value) VALUES
    ('one',   'a'),
    ('two',   'A'),
    ('three', 'b');

CREATE INDEX dist_ci_idx ON dist_ci
USING bm25 (id, name, value)
WITH (
    key_field = 'id',
    text_fields = '{"name": {}, "value": {"fast": true}}'
);

ANALYZE dist_ci;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT value
FROM dist_ci
WHERE id @@@ paradedb.all();

SELECT COUNT(*) FROM (
    SELECT DISTINCT value
    FROM dist_ci
    WHERE id @@@ paradedb.all()
) t;

-- Correctness: must match native PG
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT COUNT(*) FROM (
    SELECT DISTINCT value
    FROM dist_ci
    WHERE id @@@ paradedb.all()
) t;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS dist_ci CASCADE;
DROP COLLATION IF EXISTS dist_ci_coll;

-- =============================================================================
-- Cleanup
-- =============================================================================

DROP TABLE IF EXISTS dist_products CASCADE;
