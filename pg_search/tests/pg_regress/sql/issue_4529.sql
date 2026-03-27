SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS typmod_products CASCADE;
DROP TABLE IF EXISTS typmod_suppliers CASCADE;

CREATE TABLE typmod_suppliers (
    id   INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE typmod_products (
    id          INTEGER PRIMARY KEY,
    name        TEXT,
    description TEXT,
    supplier_id INTEGER
);

INSERT INTO typmod_suppliers VALUES (1, 'Alpha'), (2, 'Beta');
INSERT INTO typmod_products VALUES
    (1, 'Widget',  'A fine widget',  1),
    (2, 'Gadget',  'A cool gadget',  1),
    (3, 'Gizmo',   'A neat gizmo',   2);

-- Index using typmod syntax — the field IS fast (verified below)
CREATE INDEX typmod_products_idx ON typmod_products
    USING bm25 (id, ((name)::pdb.literal_normalized), description, supplier_id)
    WITH (key_field = 'id');

CREATE INDEX typmod_suppliers_idx ON typmod_suppliers
    USING bm25 (id, ((name)::pdb.literal_normalized))
    WITH (key_field = 'id');

SET paradedb.enable_join_custom_scan = on;

SELECT name, fast FROM paradedb.schema('typmod_products_idx') WHERE name = 'name';
SELECT name, fast FROM paradedb.schema('typmod_suppliers_idx') WHERE name = 'name';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT s.name
FROM typmod_products p
JOIN typmod_suppliers s ON p.supplier_id = s.id
WHERE p.description === 'widget'
ORDER BY s.name
LIMIT 10;

SELECT DISTINCT s.name
FROM typmod_products p
JOIN typmod_suppliers s ON p.supplier_id = s.id
WHERE p.description === 'widget'
ORDER BY s.name
LIMIT 10;

DROP TABLE typmod_products CASCADE;
DROP TABLE typmod_suppliers CASCADE;
RESET max_parallel_workers_per_gather;
RESET paradedb.enable_join_custom_scan;
