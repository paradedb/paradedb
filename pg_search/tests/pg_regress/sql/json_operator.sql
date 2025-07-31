\i common/common_setup.sql

DROP TABLE IF EXISTS mock_items;
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

INSERT INTO mock_items (description, metadata) VALUES
('Computer mouse', '{"price": 100, "color": "white", "in_stock": true}'),
('Keyboard', '{"price": 150, "color": "black", "in_stock": false}'),
('Monitor', '{"price": 200, "color": "white", "in_stock": true}'),
('Printer', '{"price": 120, "color": "black", "in_stock": false}'),
('Speaker', '{"price": 80, "color": "white", "in_stock": true}');

-- CASE 1: fast, default tokenizer
CREATE INDEX search_idx ON mock_items
USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');

-- basic FTS query
SELECT description, metadata->>'color' as color, metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white'
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' as color, metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white'
ORDER BY id LIMIT 5;

-- should be pushed down
SELECT description, metadata->>'color' as color, metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND (metadata->>'price')::int > 100
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' as color, metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND (metadata->>'price')::int > 100
ORDER BY id LIMIT 5;

-- should be pushed down
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND metadata->>'price' IS NOT NULL
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND metadata->>'price' IS NOT NULL
ORDER BY id LIMIT 5;

-- should be pushed down
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND metadata->>'price' IS NULL
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND metadata->>'price' IS NULL
ORDER BY id LIMIT 5;

-- should be pushed down
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND (metadata->>'in_stock')::boolean IS TRUE
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND (metadata->>'in_stock')::boolean IS TRUE
ORDER BY id LIMIT 5;

-- without keyword, we can't push down to term set
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' IN ('white', 'black') AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' IN ('white', 'black') AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

-- should be pushed down
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE (metadata->>'price')::int IN (80, 100, 120) AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE (metadata->>'price')::int IN (80, 100, 120) AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

-- CASE 2: fast, keyword tokenizer
ALTER INDEX search_idx SET (json_fields='{"metadata": {"fast": true, "tokenizer": {"type": "keyword"}}}');
REINDEX INDEX search_idx;

-- with keyword, should be pushed down
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' IN ('white', 'black') AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

-- CASE 3: with incorrect types
INSERT INTO mock_items (description, metadata) VALUES
('Computer mouse', '{"price": "abc", "color": 123, "in_stock": 2}');

-- not pushed down, type mismatch
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' as color, metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND (metadata->>'price')::int > 100
ORDER BY id LIMIT 5;

-- not pushed down, type mismatch
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND metadata->>'price' IS NOT NULL
ORDER BY id LIMIT 5;

-- not pushed down, type mismatch
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND metadata->>'price' IS NULL
ORDER BY id LIMIT 5;

-- should error
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' @@@ 'white' AND (metadata->>'in_stock')::boolean IS TRUE
ORDER BY id LIMIT 5;

-- not pushed down, type mismatch
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color', metadata->>'in_stock' as in_stock FROM mock_items
WHERE metadata->>'color' IN ('white', 'black') AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

-- should error
SELECT description, metadata->>'color', metadata->>'price' as price FROM mock_items
WHERE (metadata->>'price')::int IN (80, 100, 120) AND id @@@ paradedb.all()
ORDER BY id LIMIT 5;

DROP TABLE mock_items;
