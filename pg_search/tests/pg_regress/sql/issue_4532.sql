SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS departments CASCADE;
DROP TABLE IF EXISTS companies CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS regions CASCADE;

CREATE TABLE regions (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE companies (
    id INTEGER PRIMARY KEY,
    region_id INTEGER,
    name TEXT
);

CREATE TABLE departments (
    id INTEGER PRIMARY KEY,
    company_id INTEGER,
    name TEXT
);

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    company_id INTEGER,
    description TEXT
);

INSERT INTO regions VALUES (1, 'North America'), (2, 'Europe');

INSERT INTO companies VALUES
    (1, 1, 'Acme Corp'),
    (2, 2, 'Globex Inc'),
    (3, 1, 'Initech');

INSERT INTO departments VALUES
    (10, 1, 'Engineering'),
    (20, 1, 'Sales'),
    (30, 2, 'Engineering'),
    (40, 3, 'Marketing');

INSERT INTO products VALUES
    (100, 1, 'A fine widget'),
    (101, 1, 'A cool gadget'),
    (200, 2, 'A neat gizmo'),
    (300, 3, 'A boring thing');

CREATE INDEX regions_idx ON regions
USING bm25 (id, name) WITH (key_field = 'id');

CREATE INDEX companies_idx ON companies
USING bm25 (id, region_id, name) WITH (key_field = 'id');

CREATE INDEX departments_idx ON departments
USING bm25 (id, company_id, name) WITH (key_field = 'id');

CREATE INDEX products_idx ON products
USING bm25 (id, company_id, description) WITH (key_field = 'id');

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- 1. Nested SEMI join: p SEMI (c SEMI d)
--    The inner semi prunes d.company_id, so the outer key that originally
--    references d.company_id must be rewritten to c.id via equivalence.
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies c
    WHERE c.id IN (
        SELECT d.company_id
        FROM departments d
        WHERE d.name @@@ 'Engineering'
    )
)
AND p.description @@@ 'widget OR gadget OR gizmo'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.description
FROM products p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies c
    WHERE c.id IN (
        SELECT d.company_id
        FROM departments d
        WHERE d.name @@@ 'Engineering'
    )
)
AND p.description @@@ 'widget OR gadget OR gizmo'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- 2. Nested ANTI join: products from companies that have NO Marketing dept
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products p
WHERE p.description @@@ 'widget OR gadget OR gizmo OR boring'
AND EXISTS (
    SELECT 1
    FROM companies c
    WHERE c.id = p.company_id
    AND NOT EXISTS (
        SELECT 1
        FROM departments d
        WHERE d.company_id = c.id
        AND d.name @@@ 'Marketing'
    )
)
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.description
FROM products p
WHERE p.description @@@ 'widget OR gadget OR gizmo OR boring'
AND EXISTS (
    SELECT 1
    FROM companies c
    WHERE c.id = p.company_id
    AND NOT EXISTS (
        SELECT 1
        FROM departments d
        WHERE d.company_id = c.id
        AND d.name @@@ 'Marketing'
    )
)
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- 3. Mixed SEMI + ANTI nesting: products from companies with Engineering
--    but without Marketing (using EXISTS/NOT EXISTS for proper join conversion)
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products p
WHERE p.description @@@ 'widget OR gadget OR gizmo OR boring'
AND EXISTS (
    SELECT 1
    FROM companies c
    WHERE c.id = p.company_id
    AND EXISTS (
        SELECT 1
        FROM departments d
        WHERE d.company_id = c.id
        AND d.name @@@ 'Engineering'
    )
    AND NOT EXISTS (
        SELECT 1
        FROM departments d
        WHERE d.company_id = c.id
        AND d.name @@@ 'Marketing'
    )
)
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.description
FROM products p
WHERE p.description @@@ 'widget OR gadget OR gizmo OR boring'
AND EXISTS (
    SELECT 1
    FROM companies c
    WHERE c.id = p.company_id
    AND EXISTS (
        SELECT 1
        FROM departments d
        WHERE d.company_id = c.id
        AND d.name @@@ 'Engineering'
    )
    AND NOT EXISTS (
        SELECT 1
        FROM departments d
        WHERE d.company_id = c.id
        AND d.name @@@ 'Marketing'
    )
)
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- 4. Flat (non-nested) semi join baseline: no pruning needed
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies c
    WHERE c.name @@@ 'Acme'
)
AND p.description @@@ 'widget OR gadget'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.description
FROM products p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies c
    WHERE c.name @@@ 'Acme'
)
AND p.description @@@ 'widget OR gadget'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- 5. Three-level nesting: p SEMI ((c SEMI r) SEMI d)
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies c
    WHERE c.region_id IN (
        SELECT r.id
        FROM regions r
        WHERE r.name @@@ 'America'
    )
    AND c.id IN (
        SELECT d.company_id
        FROM departments d
        WHERE d.name @@@ 'Engineering'
    )
)
AND p.description @@@ 'widget OR gadget OR gizmo OR boring'
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.description
FROM products p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies c
    WHERE c.region_id IN (
        SELECT r.id
        FROM regions r
        WHERE r.name @@@ 'America'
    )
    AND c.id IN (
        SELECT d.company_id
        FROM departments d
        WHERE d.name @@@ 'Engineering'
    )
)
AND p.description @@@ 'widget OR gadget OR gizmo OR boring'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- Cleanup
-- =============================================================================
DROP TABLE products CASCADE;
DROP TABLE departments CASCADE;
DROP TABLE companies CASCADE;
DROP TABLE regions CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
