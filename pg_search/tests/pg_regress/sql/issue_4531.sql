-- Issue #4531: IN (SELECT ...) OR col IS NULL prevents JoinScan
-- by blocking subquery flattening.
--
-- When a query uses `col IN (SELECT ...) OR col IS NULL`, PostgreSQL keeps
-- the subquery as a SubPlan instead of flattening it into a join. This test
-- verifies that JoinScan now handles this pattern via LeftMark join.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan TO OFF;
SET paradedb.enable_join_custom_scan = on;

-- Setup tables
CREATE TABLE suppliers_4531 (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE products_4531 (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER
);

INSERT INTO suppliers_4531 VALUES (1, 'Alpha'), (2, 'Beta');
INSERT INTO products_4531 VALUES
    (1, 'Widget',  'A fine widget',  1),
    (2, 'Gadget',  'A cool gadget',  1),
    (3, 'Gizmo',   'A neat gizmo',   NULL),
    (4, 'Doohicky','Another widget',  2),
    (5, 'Thingamajig', 'Yet another widget', 999);

CREATE INDEX ON products_4531 USING bm25 (id, name, description, supplier_id)
    WITH (key_field='id', numeric_fields='{"supplier_id": {"fast": true}}');
CREATE INDEX ON suppliers_4531 USING bm25 (id, name)
    WITH (key_field='id');

-- ============================================================
-- Test 1: The original failing pattern — OR IS NULL
-- Should now use JoinScan (LeftMark join)
-- ============================================================

-- EXPLAIN to verify JoinScan activates
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

-- Actual query execution
SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

-- ============================================================
-- Test 2: Without OR IS NULL — should use regular JoinScan (Semi)
-- (This already worked before)
-- ============================================================

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND p.supplier_id IN (SELECT s.id FROM suppliers_4531 s)
ORDER BY p.id DESC LIMIT 10;

SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND p.supplier_id IN (SELECT s.id FROM suppliers_4531 s)
ORDER BY p.id DESC LIMIT 10;

-- ============================================================
-- Test 3: All NULLs — edge case where all supplier_ids are NULL
-- ============================================================

-- Temporarily set all supplier_ids to NULL
UPDATE products_4531 SET supplier_id = NULL;

SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

-- Restore original data
UPDATE products_4531 SET supplier_id = 1 WHERE id IN (1, 2);
UPDATE products_4531 SET supplier_id = NULL WHERE id = 3;
UPDATE products_4531 SET supplier_id = 2 WHERE id = 4;
UPDATE products_4531 SET supplier_id = 999 WHERE id = 5;

-- ============================================================
-- Test 4: No matching suppliers — only NULL rows should pass
-- ============================================================
DELETE FROM suppliers_4531;

SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

-- Restore suppliers
INSERT INTO suppliers_4531 VALUES (1, 'Alpha'), (2, 'Beta');

-- ============================================================
-- Test 5: NULL supplier_id row that ALSO matches 'widget'
-- Ensures the LeftMark join correctly passes NULL outer keys.
-- ============================================================
INSERT INTO products_4531 VALUES (6, 'NullWidget', 'A null widget', NULL);

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

-- ============================================================
-- Verify: same query with JoinScan OFF must return same results
-- ============================================================
SET paradedb.enable_join_custom_scan = off;

SELECT p.id
FROM products_4531 p
WHERE p.description @@@ 'widget'
  AND (p.supplier_id IS NULL OR p.supplier_id IN (SELECT s.id FROM suppliers_4531 s))
ORDER BY p.id DESC LIMIT 10;

SET paradedb.enable_join_custom_scan = on;

-- Cleanup
DROP TABLE products_4531 CASCADE;
DROP TABLE suppliers_4531 CASCADE;
