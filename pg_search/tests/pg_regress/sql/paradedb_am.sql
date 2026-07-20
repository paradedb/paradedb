-- Regression test: a `USING paradedb` index drives every ParadeDB custom scan
-- (base, TopK, aggregate, join) through the `@@@` operator, exactly as a
-- `USING bm25` index does. `paradedb` is the primary name for the access method
-- historically known as `bm25`; both share the same handler.

CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;

-- Pin parallelism off so the plans do not depend on the local cluster's worker config.
SET max_parallel_workers_per_gather TO 0;

DROP TABLE IF EXISTS items CASCADE;
DROP TABLE IF EXISTS categories CASCADE;

CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    name TEXT,
    content TEXT,
    category_id INTEGER REFERENCES categories(id),
    rating INTEGER
);

INSERT INTO categories (id, name) VALUES
    (1, 'Electronics'),
    (2, 'Books'),
    (3, 'Clothing');

INSERT INTO items (id, name, content, category_id, rating) VALUES
    (101, 'Laptop', 'wireless portable computer', 1, 5),
    (102, 'Phone', 'wireless mobile device', 1, 4),
    (103, 'Novel', 'fiction book story', 2, 3),
    (104, 'Textbook', 'educational book reference', 2, 2),
    (105, 'Shirt', 'cotton casual wear', 3, 1),
    (106, 'Jacket', 'wireless heated outerwear', 3, 5);

-- v2 API: numeric fields (id, category_id, rating) are columnar/fast by default,
-- so no per-field options are needed for the aggregate/join/TopK paths.
CREATE INDEX items_paradedb_idx ON items USING paradedb (id, name, content, category_id, rating)
    WITH (key_field = 'id');

CREATE INDEX categories_paradedb_idx ON categories USING paradedb (id, name)
    WITH (key_field = 'id');

-- The index really is built with the `paradedb` access method.
SELECT am.amname
FROM pg_class c
JOIN pg_am am ON c.relam = am.oid
WHERE c.relname = 'items_paradedb_idx';

-- Base custom scan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM items WHERE content @@@ 'wireless' ORDER BY id;
SELECT id FROM items WHERE content @@@ 'wireless' ORDER BY id;

-- TopK scan: ORDER BY score + LIMIT pushed into the index.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM items WHERE content @@@ 'wireless' ORDER BY pdb.score(id) DESC, id LIMIT 2;
SELECT id FROM items WHERE content @@@ 'wireless' ORDER BY pdb.score(id) DESC, id LIMIT 2;

-- Aggregate custom scan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) FROM items WHERE content @@@ 'wireless';
SELECT count(*) FROM items WHERE content @@@ 'wireless';

-- Join custom scan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT i.id, c.name
FROM items i
JOIN categories c ON i.category_id = c.id
WHERE i.content @@@ 'wireless'
ORDER BY i.id
LIMIT 10;
SELECT i.id, c.name
FROM items i
JOIN categories c ON i.category_id = c.id
WHERE i.content @@@ 'wireless'
ORDER BY i.id
LIMIT 10;

DROP TABLE items CASCADE;
DROP TABLE categories CASCADE;

-- Single-index guard: because `paradedb` and `bm25` share a handler, they count
-- as the same index for the "one index per relation" rule. With a `bm25` index
-- already present, creating a `paradedb` index non-concurrently is rejected...
CREATE TABLE guard_test (id INTEGER PRIMARY KEY, content TEXT);
INSERT INTO guard_test (id, content) VALUES (1, 'wireless keyboard'), (2, 'wired mouse');

CREATE INDEX guard_bm25_idx ON guard_test USING bm25 (id, content) WITH (key_field = 'id');

CREATE INDEX guard_paradedb_idx ON guard_test USING paradedb (id, content) WITH (key_field = 'id');

-- ...but CREATE INDEX CONCURRENTLY bypasses the guard, so the two coexist.
CREATE INDEX CONCURRENTLY guard_paradedb_idx ON guard_test USING paradedb (id, content) WITH (key_field = 'id');

SELECT c.relname, am.amname
FROM pg_class c
JOIN pg_am am ON c.relam = am.oid
WHERE c.relname IN ('guard_bm25_idx', 'guard_paradedb_idx')
ORDER BY c.relname;

DROP TABLE guard_test CASCADE;
