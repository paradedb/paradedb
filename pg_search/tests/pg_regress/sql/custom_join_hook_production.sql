-- Test production scenarios and edge cases for custom join execution
-- This test validates robustness and handles real-world edge cases

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_join_coordination = true;

-- Test 1: Different data types and NULL handling
CREATE TABLE mixed_types_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    created_at TIMESTAMP,
    score NUMERIC(5,2),
    is_published BOOLEAN,
    metadata JSONB
);

CREATE TABLE mixed_types_reviews (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER,
    reviewer_name TEXT,
    review_text TEXT,
    rating INTEGER,
    review_date DATE,
    helpful_votes INTEGER DEFAULT 0
);

-- Insert data with various types and NULLs
INSERT INTO mixed_types_docs (title, content, created_at, score, is_published, metadata) VALUES 
    ('Document 1', 'Technology content with innovation', '2024-01-01 10:00:00', 95.50, true, '{"category": "tech", "priority": "high"}'),
    ('Document 2', 'Science research and analysis', '2024-01-02 11:00:00', 87.25, true, '{"category": "science", "priority": "medium"}'),
    ('Document 3', 'Business strategy overview', '2024-01-03 12:00:00', NULL, false, NULL),
    ('Document 4', NULL, '2024-01-04 13:00:00', 92.75, true, '{"category": "business"}'),
    (NULL, 'Content without title', NULL, 88.00, NULL, '{"priority": "low"}');

INSERT INTO mixed_types_reviews (doc_id, reviewer_name, review_text, rating, review_date, helpful_votes) VALUES 
    (1, 'John Doe', 'Excellent technology analysis', 5, '2024-01-05', 10),
    (1, NULL, 'Good innovation coverage', 4, '2024-01-06', NULL),
    (2, 'Jane Smith', 'Comprehensive science review', 5, '2024-01-07', 8),
    (3, 'Bob Wilson', NULL, 3, '2024-01-08', 2),
    (4, 'Alice Brown', 'Solid business insights', 4, NULL, 5),
    (999, 'Test User', 'Review for non-existent doc', 1, '2024-01-09', 0);

-- Create BM25 indexes
CREATE INDEX mixed_types_docs_idx ON mixed_types_docs USING bm25 (
    id, title, content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX mixed_types_reviews_idx ON mixed_types_reviews USING bm25 (
    id, doc_id, reviewer_name, review_text
) WITH (
    key_field = 'id',
    numeric_fields = '{"doc_id": {"fast": true}}',
    text_fields = '{"reviewer_name": {"tokenizer": {"type": "default"}}, "review_text": {"tokenizer": {"type": "default"}}}'
);

-- Test NULL handling in joins
SELECT d.title, r.reviewer_name, r.review_text, r.rating
FROM mixed_types_docs d
JOIN mixed_types_reviews r ON d.id = r.doc_id
WHERE d.content @@@ 'technology' AND r.review_text @@@ 'excellent';

-- Test 2: Complex join conditions with multiple predicates
CREATE TABLE complex_products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    price DECIMAL(10,2),
    manufacturer TEXT
);

CREATE TABLE complex_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    title TEXT,
    content TEXT,
    rating INTEGER,
    verified_purchase BOOLEAN
);

-- Insert test data
INSERT INTO complex_products (name, description, category, price, manufacturer) VALUES 
    ('Laptop Pro X1', 'High-performance laptop with advanced features', 'electronics', 1299.99, 'TechCorp'),
    ('Wireless Headphones', 'Premium noise-canceling headphones', 'electronics', 299.99, 'AudioTech'),
    ('Smart Watch', 'Fitness tracking smartwatch with GPS', 'wearables', 399.99, 'WearableTech'),
    ('Gaming Mouse', 'Precision gaming mouse with RGB lighting', 'electronics', 79.99, 'GameGear'),
    ('Coffee Machine', 'Automatic espresso machine with grinder', 'appliances', 599.99, 'BrewMaster');

INSERT INTO complex_reviews (product_id, title, content, rating, verified_purchase) VALUES 
    (1, 'Amazing Performance', 'This laptop delivers exceptional performance for professional work', 5, true),
    (1, 'Great Build Quality', 'Solid construction and premium materials', 4, true),
    (2, 'Excellent Sound', 'Outstanding audio quality with great noise cancellation', 5, true),
    (2, 'Comfortable Fit', 'Very comfortable for long listening sessions', 4, false),
    (3, 'Perfect for Fitness', 'Accurate tracking and long battery life', 5, true),
    (4, 'Responsive Gaming', 'Great precision and responsiveness for gaming', 4, true),
    (5, 'Perfect Coffee', 'Makes excellent espresso every time', 5, true);

-- Create BM25 indexes
CREATE INDEX complex_products_idx ON complex_products USING bm25 (
    id, name, description, category, manufacturer
) WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "default"}}, "description": {"tokenizer": {"type": "default"}}, "category": {"tokenizer": {"type": "default"}}, "manufacturer": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX complex_reviews_idx ON complex_reviews USING bm25 (
    id, product_id, title, content
) WITH (
    key_field = 'id',
    numeric_fields = '{"product_id": {"fast": true}}',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

-- Test complex search predicates
SELECT p.name, p.category, r.title, r.content, r.rating
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'performance OR quality' AND r.content @@@ 'excellent AND professional';

-- Test 3: Concurrent access simulation (multiple similar queries)
-- These would typically be run in parallel in a real scenario

SELECT COUNT(*) as concurrent_test_1
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop' AND r.content @@@ 'performance';

SELECT COUNT(*) as concurrent_test_2
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'headphones' AND r.content @@@ 'audio';

SELECT COUNT(*) as concurrent_test_3
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'smartwatch' AND r.content @@@ 'fitness';

-- Test 4: Memory pressure simulation with large result sets
CREATE TABLE memory_test_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    category TEXT
);

CREATE TABLE memory_test_comments (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER,
    comment_text TEXT,
    author TEXT
);

-- Insert larger dataset to test memory handling
INSERT INTO memory_test_docs (title, content, category)
SELECT 
    'Document ' || i,
    'Content with keywords: ' || 
    string_agg(
        CASE (j % 10)
            WHEN 0 THEN 'technology'
            WHEN 1 THEN 'innovation'
            WHEN 2 THEN 'research'
            WHEN 3 THEN 'analysis'
            WHEN 4 THEN 'development'
            WHEN 5 THEN 'solution'
            WHEN 6 THEN 'implementation'
            WHEN 7 THEN 'optimization'
            WHEN 8 THEN 'performance'
            ELSE 'efficiency'
        END, ' '
    ),
    CASE (i % 5)
        WHEN 0 THEN 'tech'
        WHEN 1 THEN 'business'
        WHEN 2 THEN 'science'
        WHEN 3 THEN 'education'
        ELSE 'general'
    END
FROM generate_series(1, 1000) i
CROSS JOIN generate_series(1, 5) j
GROUP BY i;

INSERT INTO memory_test_comments (doc_id, comment_text, author)
SELECT 
    (i % 1000) + 1,
    'Comment about ' ||
    CASE (i % 8)
        WHEN 0 THEN 'excellent technology'
        WHEN 1 THEN 'innovative solution'
        WHEN 2 THEN 'thorough research'
        WHEN 3 THEN 'detailed analysis'
        WHEN 4 THEN 'effective development'
        WHEN 5 THEN 'optimal performance'
        WHEN 6 THEN 'efficient implementation'
        ELSE 'comprehensive optimization'
    END,
    'User' || ((i % 100) + 1)
FROM generate_series(1, 3000) i;

-- Create BM25 indexes
CREATE INDEX memory_test_docs_idx ON memory_test_docs USING bm25 (
    id, title, content, category
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}, "category": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX memory_test_comments_idx ON memory_test_comments USING bm25 (
    id, doc_id, comment_text, author
) WITH (
    key_field = 'id',
    numeric_fields = '{"doc_id": {"fast": true}}',
    text_fields = '{"comment_text": {"tokenizer": {"type": "default"}}, "author": {"tokenizer": {"type": "default"}}}'
);

-- Test memory handling with large result sets
SELECT COUNT(*) as memory_pressure_test
FROM memory_test_docs d
JOIN memory_test_comments c ON d.id = c.doc_id
WHERE d.content @@@ 'technology OR innovation' AND c.comment_text @@@ 'excellent OR innovative';

-- Test 5: Error recovery scenarios

-- Test with invalid join conditions (should handle gracefully)
SELECT COUNT(*) as invalid_join_test
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'nonexistent_term_12345' AND r.content @@@ 'impossible_phrase_67890';

-- Test with very long search terms
SELECT COUNT(*) as long_search_test
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'this_is_a_very_long_search_term_that_should_not_match_anything_in_the_database_but_should_be_handled_gracefully' 
AND r.content @@@ 'another_extremely_long_search_phrase_that_tests_the_robustness_of_the_search_implementation';

-- Test 6: Special characters and encoding
CREATE TABLE encoding_test_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT
);

CREATE TABLE encoding_test_reviews (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER,
    review_text TEXT
);

-- Insert data with special characters
INSERT INTO encoding_test_docs (title, content) VALUES 
    ('Café & Restaurant', 'Délicious food with naïve service'),
    ('Résumé Builder', 'Professional résumé creation tool'),
    ('Math: π ≈ 3.14159', 'Mathematical constants and formulas: α, β, γ'),
    ('Emoji Test 😀', 'Content with emojis: 🚀 🎯 💡'),
    ('Quotes "Test"', 'Content with ''single'' and "double" quotes');

INSERT INTO encoding_test_reviews (doc_id, review_text) VALUES 
    (1, 'Excellent café with great ambiance'),
    (2, 'Perfect for creating professional résumés'),
    (3, 'Great mathematical reference with π calculations'),
    (4, 'Fun app with emoji support 😊'),
    (5, 'Handles quotes properly "very well"');

-- Create BM25 indexes
CREATE INDEX encoding_test_docs_idx ON encoding_test_docs USING bm25 (
    id, title, content
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX encoding_test_reviews_idx ON encoding_test_reviews USING bm25 (
    id, doc_id, review_text
) WITH (
    key_field = 'id',
    numeric_fields = '{"doc_id": {"fast": true}}',
    text_fields = '{"review_text": {"tokenizer": {"type": "default"}}}'
);

-- Test special character handling
SELECT d.title, r.review_text
FROM encoding_test_docs d
JOIN encoding_test_reviews r ON d.id = r.doc_id
WHERE d.content @@@ 'café OR résumé' AND r.review_text @@@ 'excellent OR professional';

-- Test 7: Performance under different load patterns

-- Burst of small queries (simulating high-frequency, low-complexity workload)
SELECT COUNT(*) FROM complex_products p JOIN complex_reviews r ON p.id = r.product_id WHERE p.name @@@ 'Laptop' AND r.title @@@ 'Amazing';
SELECT COUNT(*) FROM complex_products p JOIN complex_reviews r ON p.id = r.product_id WHERE p.name @@@ 'Headphones' AND r.title @@@ 'Excellent';
SELECT COUNT(*) FROM complex_products p JOIN complex_reviews r ON p.id = r.product_id WHERE p.name @@@ 'Watch' AND r.title @@@ 'Perfect';
SELECT COUNT(*) FROM complex_products p JOIN complex_reviews r ON p.id = r.product_id WHERE p.name @@@ 'Mouse' AND r.title @@@ 'Responsive';
SELECT COUNT(*) FROM complex_products p JOIN complex_reviews r ON p.id = r.product_id WHERE p.name @@@ 'Coffee' AND r.title @@@ 'Perfect';

-- Large complex query (simulating analytical workload)
SELECT 
    p.category,
    COUNT(*) as product_count,
    AVG(r.rating) as avg_rating,
    COUNT(DISTINCT r.id) as review_count
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'performance OR quality OR premium' 
AND r.content @@@ 'excellent OR outstanding OR amazing'
GROUP BY p.category
ORDER BY avg_rating DESC;

-- Test 8: Edge cases for semi-join optimization

-- Test with empty result sets
SELECT COUNT(*) as empty_result_test
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'zzz_nonexistent' AND r.content @@@ 'yyy_impossible';

-- Test with single result
SELECT p.name, r.title
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.name @@@ 'Laptop Pro X1' AND r.title @@@ 'Amazing Performance'
LIMIT 1;

-- Test with all results (broad search)
SELECT COUNT(*) as broad_result_test
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'with' AND r.content @@@ 'and';

-- Test 9: Explain plans for different scenarios
EXPLAIN (COSTS OFF, BUFFERS OFF, ANALYZE OFF)
SELECT p.name, r.title
FROM complex_products p
JOIN complex_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop performance' AND r.content @@@ 'exceptional professional';

EXPLAIN (COSTS OFF, BUFFERS OFF, ANALYZE OFF)
SELECT COUNT(*)
FROM memory_test_docs d
JOIN memory_test_comments c ON d.id = c.doc_id
WHERE d.content @@@ 'technology innovation' AND c.comment_text @@@ 'excellent innovative';

-- Test 10: Cleanup and resource management
-- These tests ensure proper cleanup after various scenarios

-- Force cleanup by dropping and recreating tables
DROP TABLE IF EXISTS temp_test_docs CASCADE;
CREATE TABLE temp_test_docs (id SERIAL PRIMARY KEY, content TEXT);
INSERT INTO temp_test_docs (content) VALUES ('test content');
DROP TABLE temp_test_docs CASCADE;

-- Test with transaction rollback
BEGIN;
CREATE TABLE rollback_test (id SERIAL PRIMARY KEY, data TEXT);
INSERT INTO rollback_test (data) VALUES ('test data');
ROLLBACK;

-- Cleanup all test tables
DROP TABLE mixed_types_docs CASCADE;
DROP TABLE mixed_types_reviews CASCADE;
DROP TABLE complex_products CASCADE;
DROP TABLE complex_reviews CASCADE;
DROP TABLE memory_test_docs CASCADE;
DROP TABLE memory_test_comments CASCADE;
DROP TABLE encoding_test_docs CASCADE;
DROP TABLE encoding_test_reviews CASCADE;

RESET paradedb.enable_join_coordination; 
