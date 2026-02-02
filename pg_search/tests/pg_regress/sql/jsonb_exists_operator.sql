-- =====================================================================
-- Test JSON ? operator pushsown 
-- =====================================================================
-- The JSONB ? operator should be equivalent to paradedb.exists()

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

CREATE INDEX idx_jsonb_exists_test ON jsonb_exists_test USING bm25 (id, description, (data::pdb.literal))
WITH (key_field = 'id');

-- Test 1: Basic JSONB ? operator - should return rows where data has 'first_name' key
-- This should be equivalent to: id @@@ paradedb.exists('data.first_name')
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

-- Clean up
DROP TABLE IF EXISTS jsonb_exists_test;
