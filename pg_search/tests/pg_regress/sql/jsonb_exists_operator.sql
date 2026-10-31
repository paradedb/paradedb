-- =====================================================================
-- Test JSON ? operator pushsown 
-- =====================================================================
-- The index cannot answer ? exactly, so it runs as a heap filter in the custom scan

DROP TABLE IF EXISTS jsonb_exists_test;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE jsonb_exists_test (
    id SERIAL PRIMARY KEY,
    description TEXT,
    data JSONB
);

INSERT INTO jsonb_exists_test (description, data) VALUES ('Marketing manager', '{"first_name": "John", "last_name": "Smith"}');
INSERT INTO jsonb_exists_test (description, data) VALUES ('Sales manager', '{"first_name": "Jane"}');
INSERT INTO jsonb_exists_test (description, data) VALUES ('Engineer', '{"last_name": "Wilson"}');
INSERT INTO jsonb_exists_test (description, data) VALUES ('CEO', NULL);
INSERT INTO jsonb_exists_test (description, data) VALUES ('CTO', '{"first_name": "Jim", "last_name": "Johnson"}');
INSERT INTO jsonb_exists_test (description, data) VALUES ('Intern', '{"address": {"city": "New York", "zip": "10001"}}');

CREATE INDEX idx_jsonb_exists_test ON jsonb_exists_test USING paradedb (id, description, (data::pdb.literal));

-- Test 1: Basic JSONB ? operator - should return rows where data has 'first_name' key
EXPLAIN SELECT * FROM jsonb_exists_test WHERE data ? 'first_name' AND id @@@ pdb.all() ORDER BY id;
SELECT * FROM jsonb_exists_test WHERE data ? 'first_name' AND id @@@ pdb.all() ORDER BY id;

-- Test 2: JSONB ? operator with OR condition
EXPLAIN SELECT * FROM jsonb_exists_test WHERE data ? 'last_name' OR description ||| 'CEO' ORDER BY id;
SELECT * FROM jsonb_exists_test WHERE data ? 'last_name' OR description ||| 'CEO' ORDER BY id;

-- Test 3: JSONB ? operator with AND condition
EXPLAIN SELECT * FROM jsonb_exists_test WHERE data ? 'first_name' AND data ? 'last_name'  AND id @@@ pdb.all() ORDER BY id;
SELECT * FROM jsonb_exists_test WHERE data ? 'first_name' AND data ? 'last_name'  AND id @@@ pdb.all() ORDER BY id;

-- Test 5: JSONB ? with nested path using -> operator
-- data->'address' ? 'city' checks if 'city' key exists in data.address
EXPLAIN SELECT * FROM jsonb_exists_test WHERE data->'address' ? 'city' AND id @@@ pdb.all() ORDER BY id;
SELECT * FROM jsonb_exists_test WHERE data->'address' ? 'city' AND id @@@ pdb.all() ORDER BY id;

-- Test 7: NOT EXISTS using JSONB ? operator
EXPLAIN SELECT * FROM jsonb_exists_test WHERE NOT (data ? 'first_name') AND id @@@ pdb.all() ORDER BY id;
SELECT * FROM jsonb_exists_test WHERE NOT (data ? 'first_name') AND id @@@ pdb.all() ORDER BY id;

-- Test 8: ? matches PostgreSQL for arrays, string scalars, dotted keys,
-- null values and arrays of objects (#7027)
CREATE TABLE jsonb_exists_shapes (id int PRIMARY KEY, body text, data jsonb);
INSERT INTO jsonb_exists_shapes VALUES
  (1, 'array foo', '["foo","bar"]'),
  (2, 'array bar', '["bar"]'),
  (3, 'object foo', '{"foo":1,"nested":{"foo":true}}'),
  (4, 'object other', '{"other":1}'),
  (5, 'scalar foo', '"foo"'),
  (6, 'sql null', NULL),
  (7, 'literal dot', '{"a.b":1}'),
  (8, 'nested dot', '{"a":{"b":1}}'),
  (9, 'null value', '{"foo":null}'),
  (10, 'array of objects', '[{"foo":1}]');
CREATE INDEX jsonb_exists_shapes_idx ON jsonb_exists_shapes USING paradedb (id, body, data)
  WITH (json_fields = '{"data": {"fast": true}}');
SET paradedb.planner_warnings = 'off';

EXPLAIN (COSTS OFF) SELECT id FROM jsonb_exists_shapes WHERE data ? 'foo' AND id @@@ pdb.all();
SELECT array_agg(id ORDER BY id) FROM jsonb_exists_shapes WHERE data ? 'foo' AND id @@@ pdb.all();
SELECT array_agg(id ORDER BY id) FROM jsonb_exists_shapes WHERE data ? 'bar' AND id @@@ pdb.all();
SELECT array_agg(id ORDER BY id) FROM jsonb_exists_shapes WHERE data ? 'a.b' AND id @@@ pdb.all();

RESET paradedb.planner_warnings;
DROP TABLE jsonb_exists_shapes;

-- Clean up
DROP TABLE IF EXISTS jsonb_exists_test;
