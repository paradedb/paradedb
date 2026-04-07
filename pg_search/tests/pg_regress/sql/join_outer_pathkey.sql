SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS product_tags_op CASCADE;
DROP TABLE IF EXISTS companies_op CASCADE;
DROP TABLE IF EXISTS products_op CASCADE;

CREATE TABLE companies_op (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE products_op (
    id INTEGER PRIMARY KEY,
    company_id INTEGER,
    description TEXT
);

-- No BM25 index on this table: when its EXISTS is flattened, the equi-join
-- condition pt.product_id = p.id adds an equivalence-class member for p.id
-- from a non-JoinScan source. The outer-only pathkey fix skips this member
-- so that ORDER BY p.id is still accepted.
CREATE TABLE product_tags_op (
    id INTEGER PRIMARY KEY,
    product_id INTEGER,
    tag TEXT
);

INSERT INTO companies_op VALUES (1, 'Acme Corp'), (2, 'Globex Inc'), (3, 'Initech');

INSERT INTO products_op VALUES
    (100, 1, 'A fine widget'),
    (101, 1, 'A cool gadget'),
    (200, 2, 'A neat gizmo'),
    (300, 3, 'A boring thing');

INSERT INTO product_tags_op VALUES
    (1, 100, 'popular'),
    (2, 200, 'popular'),
    (3, 300, 'niche');

CREATE INDEX companies_op_idx ON companies_op
USING bm25 (id, name) WITH (key_field = 'id');

CREATE INDEX products_op_idx ON products_op
USING bm25 (id, company_id, description) WITH (key_field = 'id');

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- 1. Outer-only pathkey: JoinScan for products SEMI companies, with a
--    flattened EXISTS on the non-BM25 product_tags table. The ORDER BY p.id
--    pathkey's equivalence class includes pt.product_id from the non-source
--    table. Without the fix, JoinScan is rejected; with it, the non-source
--    EC member is skipped and JoinScan is accepted.
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products_op p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies_op c
    WHERE c.name @@@ 'Acme OR Globex OR Initech'
)
AND EXISTS (
    SELECT 1
    FROM product_tags_op pt
    WHERE pt.product_id = p.id
      AND pt.tag = 'popular'
)
AND p.description @@@ 'widget OR gadget OR gizmo OR boring'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.description
FROM products_op p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies_op c
    WHERE c.name @@@ 'Acme OR Globex OR Initech'
)
AND EXISTS (
    SELECT 1
    FROM product_tags_op pt
    WHERE pt.product_id = p.id
      AND pt.tag = 'popular'
)
AND p.description @@@ 'widget OR gadget OR gizmo OR boring'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- Cleanup
-- =============================================================================
DROP TABLE product_tags_op CASCADE;
DROP TABLE companies_op CASCADE;
DROP TABLE products_op CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
