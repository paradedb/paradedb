-- Test cases demonstrating OR decomposition for cross-table search queries
-- These tests specifically target the pattern: a.col1 @@@ 'test1' OR b.col2 @@@ 'test2'
-- where PostgreSQL puts the condition in joininfo but we can safely decompose it

-- Load the pg_search extension
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Setup test tables for cross-table OR conditions
DROP TABLE IF EXISTS table_a;
DROP TABLE IF EXISTS table_b;
DROP TABLE IF EXISTS table_c;

CREATE TABLE table_a (
    id SERIAL PRIMARY KEY,
    content TEXT,
    category TEXT,
    value INTEGER
);

CREATE TABLE table_b (
    id SERIAL PRIMARY KEY,
    content TEXT,
    description TEXT,
    priority INTEGER
);

CREATE TABLE table_c (
    id SERIAL PRIMARY KEY,
    content TEXT,
    notes TEXT,
    score INTEGER
);

-- Insert test data
INSERT INTO table_a (content, category, value) VALUES
('smartphone technology advanced', 'electronics', 100),
('laptop computer powerful', 'electronics', 200),
('book fiction story', 'literature', 50),
('game console modern', 'electronics', 150);

INSERT INTO table_b (content, description, priority) VALUES
('advanced features smartphone', 'mobile capabilities', 1),
('high performance specs', 'laptop specifications', 2),
('storytelling narrative', 'book content description', 3),
('gaming experience immersive', 'console entertainment', 1);

INSERT INTO table_c (content, notes, score) VALUES
('technology innovation', 'cutting edge tech notes', 95),
('performance benchmarks', 'speed test results', 88),
('narrative structure', 'story analysis notes', 77),
('entertainment value', 'fun factor assessment', 92);

-- Create BM25 indexes
CREATE INDEX table_a_bm25_idx ON table_a USING bm25 (id, content, category) WITH (key_field = 'id');
CREATE INDEX table_b_bm25_idx ON table_b USING bm25 (id, content, description) WITH (key_field = 'id');
CREATE INDEX table_c_bm25_idx ON table_c USING bm25 (id, content, notes) WITH (key_field = 'id');

-- =============================================================================
-- CORE TEST: Basic cross-table OR decomposition
-- =============================================================================
SELECT 'Core Test: Basic cross-table OR - a.content @@@ X OR b.content @@@ Y' as test_name;

-- This is the fundamental pattern we want to support:
-- PostgreSQL cannot push this to baserestrictinfo due to cross-table nature
-- But we can decompose it: extract a.content @@@ 'smartphone' for table_a, 
-- b.content @@@ 'performance' for table_b
SELECT 
    a.content as a_content,
    b.content as b_content,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score
FROM table_a a
CROSS JOIN table_b b
WHERE (a.content @@@ 'smartphone' OR b.content @@@ 'performance')
ORDER BY a.id, b.id, a_score DESC, b_score DESC;

-- =============================================================================
-- MULTI-RELATION TEST: Three-table OR decomposition
-- =============================================================================
SELECT 'Multi-Relation Test: Three-table OR - a @@@ X OR b @@@ Y OR c @@@ Z' as test_name;

-- Test OR decomposition across three relations
-- Should extract: a.content @@@ 'laptop' for table_a, 
-- b.content @@@ 'gaming' for table_b, c.content @@@ 'innovation' for table_c
SELECT 
    a.content as a_content,
    b.content as b_content,
    c.content as c_content,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score,
    paradedb.score(c.id) as c_score
FROM table_a a
CROSS JOIN table_b b
CROSS JOIN table_c c
WHERE (a.content @@@ 'laptop' OR b.content @@@ 'gaming' OR c.content @@@ 'innovation')
ORDER BY a.id, b.id, c.id, a_score DESC, b_score DESC, c_score DESC;

-- =============================================================================
-- MULTIPLE CONDITIONS PER RELATION TEST
-- =============================================================================
SELECT 'Multiple Conditions Test: Multiple search terms per relation in OR' as test_name;

-- Test multiple search conditions for the same relation within OR
-- Should extract: (a.content @@@ 'smartphone' OR a.category @@@ 'electronics') for table_a,
-- b.content @@@ 'performance' for table_b
SELECT 
    a.content as a_content,
    a.category as a_category,
    b.content as b_content,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score
FROM table_a a
CROSS JOIN table_b b
WHERE (a.content @@@ 'smartphone' OR a.category @@@ 'electronics' OR b.content @@@ 'performance')
ORDER BY a.id, b.id, a_score DESC, b_score DESC;

-- =============================================================================
-- MIXED SEARCH AND NON-SEARCH TEST
-- =============================================================================
SELECT 'Mixed Conditions Test: Search and non-search predicates in OR' as test_name;

-- Test OR with both search and non-search conditions
-- Should extract: a.content @@@ 'laptop' for table_a (search),
-- and handle a.value > 150 and b.priority = 1 appropriately
SELECT 
    a.content as a_content,
    a.value as a_value,
    b.content as b_content,
    b.priority as b_priority,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score
FROM table_a a
CROSS JOIN table_b b
WHERE (a.content @@@ 'laptop' OR a.value > 150 OR b.priority = 1)
ORDER BY a.id, b.id, a_score DESC, b_score DESC;

-- =============================================================================
-- NESTED AND/OR TEST
-- =============================================================================
SELECT 'Nested Logic Test: Complex AND/OR combinations' as test_name;

-- Test complex nested boolean logic
-- Should handle: (a.content @@@ 'smartphone' AND a.value > 50) OR (b.content @@@ 'gaming' AND b.priority = 1)
SELECT 
    a.content as a_content,
    a.value as a_value,
    b.content as b_content,
    b.priority as b_priority,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score
FROM table_a a
CROSS JOIN table_b b
WHERE (a.content @@@ 'smartphone' AND a.value > 50) 
   OR (b.content @@@ 'gaming' AND b.priority = 1)
ORDER BY a.id, b.id, a_score DESC, b_score DESC;

-- =============================================================================
-- UNSAFE CONDITIONS TEST (should be rejected)
-- =============================================================================
SELECT 'Unsafe Conditions Test: Conditions that should be rejected' as test_name;

-- Test conditions that cannot be safely decomposed
-- This should be rejected because the comparison spans tables
SELECT 
    a.content as a_content,
    b.content as b_content,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score
FROM table_a a
CROSS JOIN table_b b
WHERE (a.content @@@ 'smartphone' OR a.value = b.priority)  -- Mixed safe and unsafe
ORDER BY a.id, b.id, a_score DESC, b_score DESC;

-- =============================================================================
-- JOIN-BASED TESTS (more realistic scenarios)
-- =============================================================================
SELECT 'Join-Based Test: OR decomposition with actual joins' as test_name;

-- Test OR decomposition in more realistic join scenarios
-- This is more likely to trigger joininfo conditions
SELECT 
    a.content as a_content,
    b.content as b_content,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score
FROM table_a a
JOIN table_b b ON a.value = b.priority * 100  -- Join condition
WHERE (a.content @@@ 'electronics' OR b.description @@@ 'capabilities')
ORDER BY a.id, b.id, a_score DESC, b_score DESC;

-- =============================================================================
-- COMPLEX REAL-WORLD SCENARIO
-- =============================================================================
SELECT 'Real-World Test: Complex multi-table search with various conditions' as test_name;

-- Complex scenario combining multiple search patterns
SELECT 
    a.content as a_content,
    b.content as b_content,
    c.content as c_content,
    paradedb.score(a.id) as a_score,
    paradedb.score(b.id) as b_score,
    paradedb.score(c.id) as c_score
FROM table_a a
LEFT JOIN table_b b ON a.value > b.priority * 50
LEFT JOIN table_c c ON b.priority = c.score / 30
WHERE (a.content @@@ 'technology' OR a.category @@@ 'electronics')
   OR (b.content @@@ 'performance' OR b.description @@@ 'specifications')
   OR (c.content @@@ 'innovation' OR c.notes @@@ 'cutting')
ORDER BY a.id, b.id, c.id, a_score DESC, b_score DESC, c_score DESC;

-- =============================================================================
-- VERIFICATION TESTS: Compare with single-table queries
-- =============================================================================
SELECT 'Verification: Single-table queries for comparison' as test_name;

-- Verify that our decomposition produces equivalent scoring to direct queries
SELECT 'Single table A - smartphone' as query_type, 
       a.content, paradedb.score(a.id) as score
FROM table_a a 
WHERE a.content @@@ 'smartphone'
UNION ALL
SELECT 'Single table B - performance' as query_type,
       b.content, paradedb.score(b.id) as score  
FROM table_b b
WHERE b.content @@@ 'performance'
ORDER BY b.id, score DESC;

-- =============================================================================
-- SUMMARY
-- =============================================================================
SELECT 'Summary: OR decomposition tests completed' as summary;

-- Expected behavior:
-- 1. Basic cross-table OR should extract search predicates for each relation
-- 2. Multi-relation OR should handle 3+ tables correctly  
-- 3. Multiple conditions per relation should be combined appropriately
-- 4. Mixed search/non-search should extract what's safe
-- 5. Nested logic should be handled intelligently
-- 6. Unsafe conditions should be properly rejected
-- 7. Real join scenarios should work with proper decomposition
-- 8. Results should be semantically equivalent to single-table queries

-- Cleanup
DROP TABLE IF EXISTS table_c;
DROP TABLE IF EXISTS table_b;
DROP TABLE IF EXISTS table_a; 
