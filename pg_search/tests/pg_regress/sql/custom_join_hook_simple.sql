-- Test basic join hook registration and callback
-- This test verifies that our custom join hook is being registered and called

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_join_coordination = true;

-- Create simple test tables
CREATE TABLE test_table_a (
    id SERIAL PRIMARY KEY,
    name TEXT
);

CREATE TABLE test_table_b (
    id SERIAL PRIMARY KEY,
    a_id INTEGER,
    value TEXT
);

-- Insert test data
INSERT INTO test_table_a (name) VALUES ('A1'), ('A2'), ('A3');
INSERT INTO test_table_b (a_id, value) VALUES (1, 'B1'), (2, 'B2'), (3, 'B3');

-- Test: Simple INNER JOIN 
-- This should trigger our join hook and show debug output
-- The hook should be called but skip due to missing BM25 indexes
SELECT a.id, a.name, b.value
FROM test_table_a a
JOIN test_table_b b ON a.id = b.a_id
WHERE a.id = 1;

-- Cleanup
DROP TABLE test_table_a CASCADE;
DROP TABLE test_table_b CASCADE; 

RESET paradedb.enable_join_coordination;
