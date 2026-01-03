-- Test IS NULL predicate pushdown to BM25 index
-- This tests that IS NULL on indexed fields is properly pushed to Tantivy
-- (similar to how IS NOT NULL creates PushdownIsNotNull)

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS products CASCADE;

CREATE TABLE products (
    id BIGINT PRIMARY KEY,
    category_id INTEGER,
    deleted_at TIMESTAMP
);

-- Insert data: some with NULL deleted_at, some with non-NULL
INSERT INTO products SELECT i, i % 10, NULL FROM generate_series(1, 300) i;
INSERT INTO products SELECT i, i % 10, '2024-01-01 00:00:00'::timestamp FROM generate_series(301, 1000) i;

-- Create index with deleted_at as an indexed field
CREATE INDEX idx_products_bm25 ON products
USING bm25 (id, category_id, deleted_at)
WITH (key_field='id');

-- IS NOT NULL works (pushes to index via PushdownIsNotNull)
-- Should show Custom Scan with Tantivy Query containing "exists"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM products WHERE deleted_at IS NOT NULL AND id @@@ paradedb.all();

SELECT COUNT(*) FROM products WHERE deleted_at IS NOT NULL AND id @@@ paradedb.all();

-- IS NULL should also be pushed to the index (as NOT EXISTS)
-- Should show Custom Scan with Tantivy Query containing "must_not exists"
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM products WHERE deleted_at IS NULL AND id @@@ paradedb.all();

SELECT COUNT(*) FROM products WHERE deleted_at IS NULL AND id @@@ paradedb.all();

-- Combined test: verify counts are complementary (should sum to total)
SELECT 
    (SELECT COUNT(*) FROM products WHERE deleted_at IS NULL AND id @@@ paradedb.all()) AS null_count,
    (SELECT COUNT(*) FROM products WHERE deleted_at IS NOT NULL AND id @@@ paradedb.all()) AS not_null_count,
    (SELECT COUNT(*) FROM products WHERE id @@@ paradedb.all()) AS total_count;

-- Test IS NULL with additional predicates
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM products 
WHERE deleted_at IS NULL 
  AND category_id = 5 
  AND id @@@ paradedb.all();

SELECT COUNT(*) FROM products 
WHERE deleted_at IS NULL 
  AND category_id = 5 
  AND id @@@ paradedb.all();

DROP INDEX idx_products_bm25;

-- ============================================================
-- PART 2: Partial index with WHERE deleted_at IS NULL
-- When the query includes the partial index predicate, Custom Scan should work
-- ============================================================

CREATE INDEX idx_products_bm25_partial ON products
USING bm25 (id, category_id)
WITH (key_field='id')
WHERE deleted_at IS NULL;

-- Query that includes the partial index predicate (deleted_at IS NULL)
-- should use Custom Scan because predicate_implied_by(index_pred, query) returns true
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT * FROM products 
WHERE deleted_at IS NULL 
  AND category_id = 5 
  AND id @@@ paradedb.all();

SELECT COUNT(*) FROM products 
WHERE deleted_at IS NULL 
  AND category_id = 5 
  AND id @@@ paradedb.all();

-- Cleanup
DROP TABLE products CASCADE;
