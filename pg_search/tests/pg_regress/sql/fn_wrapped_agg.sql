-- Test pdb.agg() wrapped in functions and CTEs
-- This test verifies that pdb.agg() works correctly when:
-- 1. Wrapped in another function call (like jsonb_pretty)
-- 2. Used in a CTE

-- Cleanup if exists
DROP TABLE IF EXISTS fn_wrapped_agg_logs CASCADE;

-- First, create the test table
CREATE TABLE fn_wrapped_agg_logs (
    log_id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    timestamp TIMESTAMP DEFAULT NOW()
);

-- Create a BM25 index
CREATE INDEX fn_wrapped_agg_logs_idx ON fn_wrapped_agg_logs
USING bm25 (log_id, description, category)
WITH (key_field = 'log_id');

-- Insert test data
INSERT INTO fn_wrapped_agg_logs (description, category) VALUES
    ('error in application', 'app'),
    ('error in database', 'db'),
    ('warning message', 'app'),
    ('error in network', 'network'),
    ('info message', 'app');

-- Test 1: Basic pdb.agg() with window function (should work)
SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () 
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error'                                          
ORDER BY timestamp DESC LIMIT 3;

-- Test 2: pdb.agg() wrapped in jsonb_pretty (currently fails)
SELECT *, jsonb_pretty(pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()) 
FROM fn_wrapped_agg_logs
WHERE description @@@ 'error'                                          
ORDER BY timestamp DESC LIMIT 3;

-- Test 3: pdb.agg() in CTE (currently fails)
WITH agg AS (
    SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () 
    FROM fn_wrapped_agg_logs
    WHERE description @@@ 'error'                                          
    ORDER BY timestamp DESC LIMIT 3
)
SELECT * FROM agg;

-- Test 4: pdb.agg() wrapped in CTE and then wrapped in function (currently fails)
WITH agg AS (
    SELECT *, pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as agg_result
    FROM fn_wrapped_agg_logs
    WHERE description @@@ 'error'                                          
    ORDER BY timestamp DESC LIMIT 3
)
SELECT *, jsonb_pretty(agg_result) FROM agg;

-- Cleanup
DROP TABLE fn_wrapped_agg_logs CASCADE;

