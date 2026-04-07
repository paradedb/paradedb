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
    WHERE pt.id @@@ pdb.all()
      AND pt.tag = 'niche'
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
    WHERE pt.id @@@ pdb.all()
      AND pt.tag = 'niche'
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
