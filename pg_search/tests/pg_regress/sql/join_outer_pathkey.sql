SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS categories_op CASCADE;
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

CREATE INDEX product_tags_op_idx ON product_tags_op
USING bm25 (id, product_id, tag) WITH (key_field = 'id');

CREATE TABLE categories_op (
    id INTEGER PRIMARY KEY,
    product_id INTEGER,
    category_name TEXT
);

INSERT INTO categories_op VALUES
    (1, 100, 'Electronics'),
    (2, 101, 'Electronics'),
    (3, 200, 'Hardware'),
    (4, 300, 'Office');

SET paradedb.enable_join_custom_scan = on;

-- =============================================================================
-- JoinScan with NOT IN SubPlan extraction across planner roots.
--
-- The NOT IN subquery on product_tags is extracted as a SubPlan during
-- collect_join_sources_base_rel, creating an Anti join inside JoinScan.
-- Sources from the SubPlan's inner root and the main root can share the
-- same RTI number, so collect_required_fields must register join-key
-- fast fields on ALL matching sources (not just the first).
--
-- Without the fix this errors:
--   "Failed to build DataFusion logical plan: Missing right join-key column"
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products_op p
WHERE p.company_id IN (
    SELECT c.id
    FROM companies_op c
    WHERE c.name @@@ 'Acme OR Globex OR Initech'
)
AND p.id NOT IN (
    SELECT pt.product_id
    FROM product_tags_op pt
    WHERE pt.tag === 'niche'
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
AND p.id NOT IN (
    SELECT pt.product_id
    FROM product_tags_op pt
    WHERE pt.tag === 'niche'
)
AND p.description @@@ 'widget OR gadget OR gizmo OR boring'
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- Outer-only pathkey skip.
--
-- categories_op has no BM25 index, so JoinScan covers only products_op and
-- companies_op. ORDER BY cat.category_name references a relation outside the
-- JoinScan subtree and must be skipped during extract_orderby. The
-- count_relevant_pathkeys fix ensures the pathkeys length comparison in
-- finalize_clause_into_path accounts for skipped outer-only keys.
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description, cat.category_name
FROM products_op p
JOIN companies_op c ON c.id = p.company_id
JOIN categories_op cat ON cat.product_id = p.id
WHERE c.name @@@ 'Acme OR Globex'
AND p.description @@@ 'widget OR gadget OR gizmo'
ORDER BY cat.category_name, p.id
LIMIT 5;

SELECT p.id, p.description, cat.category_name
FROM products_op p
JOIN companies_op c ON c.id = p.company_id
JOIN categories_op cat ON cat.product_id = p.id
WHERE c.name @@@ 'Acme OR Globex'
AND p.description @@@ 'widget OR gadget OR gizmo'
ORDER BY cat.category_name, p.id
LIMIT 5;

-- =============================================================================
-- Pruned (NULL sentinel) column from Semi/Anti join.
--
-- When a Semi join prunes the inner side's columns (e.g. pt.tag from an IN
-- subquery), those Vars may still appear in PostgreSQL's reltarget.
-- JoinScan must emit NULL for these pruned positions. Verify that the
-- Pruned variant produces NULL without errors and that visible columns
-- from the outer side are correct.
-- =============================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT p.id, p.description
FROM products_op p
WHERE p.id IN (
    SELECT pt.product_id
    FROM product_tags_op pt
    WHERE pt.tag @@@ 'popular'
)
AND p.description @@@ 'widget OR gizmo'
ORDER BY p.id
LIMIT 5;

SELECT p.id, p.description
FROM products_op p
WHERE p.id IN (
    SELECT pt.product_id
    FROM product_tags_op pt
    WHERE pt.tag @@@ 'popular'
)
AND p.description @@@ 'widget OR gizmo'
ORDER BY p.id
LIMIT 5;

-- =============================================================================
-- Cleanup
-- =============================================================================
DROP TABLE categories_op CASCADE;
DROP TABLE product_tags_op CASCADE;
DROP TABLE companies_op CASCADE;
DROP TABLE products_op CASCADE;

RESET max_parallel_workers_per_gather;
RESET enable_indexscan;
RESET paradedb.enable_join_custom_scan;
