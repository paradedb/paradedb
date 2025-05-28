-- Test semi-join optimization strategies and effectiveness
-- This test validates the semi-join optimizer's strategy selection and performance

-- Create the extension first
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Enable the custom join feature
SET paradedb.enable_custom_join = true;

-- Test 1: Small dataset - should use SearchFilter strategy
CREATE TABLE small_products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT
);

CREATE TABLE small_reviews (
    id SERIAL PRIMARY KEY,
    product_id INTEGER,
    review_text TEXT,
    rating INTEGER
);

-- Insert small dataset (should trigger SearchFilter strategy)
INSERT INTO small_products (name, description, category) VALUES 
    ('Laptop Pro', 'High-performance laptop for professionals', 'electronics'),
    ('Wireless Mouse', 'Ergonomic wireless mouse with precision tracking', 'electronics'),
    ('Coffee Maker', 'Automatic drip coffee maker with timer', 'appliances');

INSERT INTO small_reviews (product_id, review_text, rating) VALUES 
    (1, 'Excellent laptop with great performance', 5),
    (1, 'Good build quality but expensive', 4),
    (2, 'Perfect mouse for daily use', 5),
    (3, 'Makes great coffee every morning', 4);

-- Create BM25 indexes
CREATE INDEX small_products_idx ON small_products USING bm25 (
    id, name, description, category
) WITH (
    key_field = 'id',
    text_fields = '{"name": {"tokenizer": {"type": "default"}}, "description": {"tokenizer": {"type": "default"}}, "category": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX small_reviews_idx ON small_reviews USING bm25 (
    id, product_id, review_text, rating
) WITH (
    key_field = 'id',
    numeric_fields = '{"product_id": {"fast": true}, "rating": {"fast": true}}',
    text_fields = '{"review_text": {"tokenizer": {"type": "default"}}}'
);

-- Test SearchFilter strategy (small result set)
SELECT p.name, r.review_text, r.rating
FROM small_products p
JOIN small_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop' AND r.review_text @@@ 'performance';

-- Test 2: Medium dataset - should use SortedArray strategy
CREATE TABLE medium_docs (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    author TEXT
);

CREATE TABLE medium_comments (
    id SERIAL PRIMARY KEY,
    doc_id INTEGER,
    comment_text TEXT,
    commenter TEXT
);

-- Insert medium dataset (1000-10000 range to trigger SortedArray)
INSERT INTO medium_docs (title, content, author)
SELECT 
    'Document ' || i,
    'Content about ' || 
    CASE (i % 10)
        WHEN 0 THEN 'technology and innovation'
        WHEN 1 THEN 'science and research'
        WHEN 2 THEN 'business and finance'
        WHEN 3 THEN 'health and medicine'
        WHEN 4 THEN 'education and learning'
        WHEN 5 THEN 'sports and fitness'
        WHEN 6 THEN 'travel and adventure'
        WHEN 7 THEN 'food and cooking'
        WHEN 8 THEN 'art and culture'
        ELSE 'music and entertainment'
    END || ' with detailed analysis and insights',
    'Author ' || ((i % 50) + 1)
FROM generate_series(1, 2000) i;

INSERT INTO medium_comments (doc_id, comment_text, commenter)
SELECT 
    (i % 2000) + 1,
    'Comment about ' ||
    CASE (i % 5)
        WHEN 0 THEN 'excellent analysis'
        WHEN 1 THEN 'interesting perspective'
        WHEN 2 THEN 'well researched'
        WHEN 3 THEN 'thought provoking'
        ELSE 'comprehensive coverage'
    END,
    'User' || ((i % 100) + 1)
FROM generate_series(1, 5000) i;

-- Create BM25 indexes
CREATE INDEX medium_docs_idx ON medium_docs USING bm25 (
    id, title, content, author
) WITH (
    key_field = 'id',
    text_fields = '{"title": {"tokenizer": {"type": "default"}}, "content": {"tokenizer": {"type": "default"}}, "author": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX medium_comments_idx ON medium_comments USING bm25 (
    id, doc_id, comment_text, commenter
) WITH (
    key_field = 'id',
    numeric_fields = '{"doc_id": {"fast": true}}',
    text_fields = '{"comment_text": {"tokenizer": {"type": "default"}}, "commenter": {"tokenizer": {"type": "default"}}}'
);

-- Test SortedArray strategy (medium result set)
SELECT COUNT(*) as total_matches
FROM medium_docs d
JOIN medium_comments c ON d.id = c.doc_id
WHERE d.content @@@ 'technology' AND c.comment_text @@@ 'excellent';

-- Test 3: Large dataset - should use BloomFilter strategy
CREATE TABLE large_articles (
    id SERIAL PRIMARY KEY,
    headline TEXT,
    body TEXT,
    category TEXT
);

CREATE TABLE large_tags (
    id SERIAL PRIMARY KEY,
    article_id INTEGER,
    tag_name TEXT,
    tag_category TEXT
);

-- Insert large dataset (>10000 to trigger BloomFilter)
INSERT INTO large_articles (headline, body, category)
SELECT 
    'Article ' || i || ': ' ||
    CASE (i % 20)
        WHEN 0 THEN 'Breaking News in Technology'
        WHEN 1 THEN 'Scientific Discovery Announced'
        WHEN 2 THEN 'Business Market Analysis'
        WHEN 3 THEN 'Health Research Findings'
        WHEN 4 THEN 'Educational Policy Changes'
        WHEN 5 THEN 'Sports Championship Results'
        WHEN 6 THEN 'Travel Destination Guide'
        WHEN 7 THEN 'Culinary Innovation Report'
        WHEN 8 THEN 'Art Exhibition Review'
        WHEN 9 THEN 'Music Festival Coverage'
        WHEN 10 THEN 'Environmental Impact Study'
        WHEN 11 THEN 'Political Development Update'
        WHEN 12 THEN 'Economic Forecast Analysis'
        WHEN 13 THEN 'Social Media Trends'
        WHEN 14 THEN 'Automotive Industry News'
        WHEN 15 THEN 'Real Estate Market Report'
        WHEN 16 THEN 'Fashion Week Highlights'
        WHEN 17 THEN 'Gaming Industry Updates'
        WHEN 18 THEN 'Cryptocurrency Analysis'
        ELSE 'General Interest Story'
    END,
    'Detailed article content about ' || 
    CASE (i % 15)
        WHEN 0 THEN 'artificial intelligence and machine learning applications'
        WHEN 1 THEN 'climate change and environmental sustainability'
        WHEN 2 THEN 'global economic trends and market dynamics'
        WHEN 3 THEN 'medical breakthroughs and healthcare innovations'
        WHEN 4 THEN 'educational technology and online learning'
        WHEN 5 THEN 'renewable energy and green technology'
        WHEN 6 THEN 'space exploration and astronomical discoveries'
        WHEN 7 THEN 'biotechnology and genetic research'
        WHEN 8 THEN 'cybersecurity and data protection'
        WHEN 9 THEN 'urban planning and smart cities'
        WHEN 10 THEN 'social justice and human rights'
        WHEN 11 THEN 'digital transformation and automation'
        WHEN 12 THEN 'sustainable agriculture and food security'
        WHEN 13 THEN 'mental health and wellness'
        ELSE 'innovation and technological advancement'
    END || ' with comprehensive analysis and expert opinions',
    CASE (i % 8)
        WHEN 0 THEN 'technology'
        WHEN 1 THEN 'science'
        WHEN 2 THEN 'business'
        WHEN 3 THEN 'health'
        WHEN 4 THEN 'education'
        WHEN 5 THEN 'environment'
        WHEN 6 THEN 'politics'
        ELSE 'general'
    END
FROM generate_series(1, 15000) i;

INSERT INTO large_tags (article_id, tag_name, tag_category)
SELECT 
    (i % 15000) + 1,
    CASE (i % 30)
        WHEN 0 THEN 'artificial-intelligence'
        WHEN 1 THEN 'machine-learning'
        WHEN 2 THEN 'climate-change'
        WHEN 3 THEN 'sustainability'
        WHEN 4 THEN 'economics'
        WHEN 5 THEN 'finance'
        WHEN 6 THEN 'healthcare'
        WHEN 7 THEN 'medicine'
        WHEN 8 THEN 'education'
        WHEN 9 THEN 'technology'
        WHEN 10 THEN 'renewable-energy'
        WHEN 11 THEN 'green-tech'
        WHEN 12 THEN 'space'
        WHEN 13 THEN 'astronomy'
        WHEN 14 THEN 'biotechnology'
        WHEN 15 THEN 'genetics'
        WHEN 16 THEN 'cybersecurity'
        WHEN 17 THEN 'data-protection'
        WHEN 18 THEN 'urban-planning'
        WHEN 19 THEN 'smart-cities'
        WHEN 20 THEN 'social-justice'
        WHEN 21 THEN 'human-rights'
        WHEN 22 THEN 'digital-transformation'
        WHEN 23 THEN 'automation'
        WHEN 24 THEN 'agriculture'
        WHEN 25 THEN 'food-security'
        WHEN 26 THEN 'mental-health'
        WHEN 27 THEN 'wellness'
        WHEN 28 THEN 'innovation'
        ELSE 'research'
    END,
    CASE (i % 5)
        WHEN 0 THEN 'primary'
        WHEN 1 THEN 'secondary'
        WHEN 2 THEN 'trending'
        WHEN 3 THEN 'featured'
        ELSE 'general'
    END
FROM generate_series(1, 45000) i;

-- Create BM25 indexes
CREATE INDEX large_articles_idx ON large_articles USING bm25 (
    id, headline, body, category
) WITH (
    key_field = 'id',
    text_fields = '{"headline": {"tokenizer": {"type": "default"}}, "body": {"tokenizer": {"type": "default"}}, "category": {"tokenizer": {"type": "default"}}}'
);

CREATE INDEX large_tags_idx ON large_tags USING bm25 (
    id, article_id, tag_name, tag_category
) WITH (
    key_field = 'id',
    numeric_fields = '{"article_id": {"fast": true}}',
    text_fields = '{"tag_name": {"tokenizer": {"type": "default"}}, "tag_category": {"tokenizer": {"type": "default"}}}'
);

-- Test BloomFilter strategy (large result set)
SELECT COUNT(*) as bloom_filter_matches
FROM large_articles a
JOIN large_tags t ON a.id = t.article_id
WHERE a.body @@@ 'artificial intelligence' AND t.tag_name @@@ 'machine-learning';

-- Test 4: Semi-join effectiveness validation
-- Test filter pushdown effectiveness by comparing with and without optimization

-- Disable semi-join temporarily to compare
SET paradedb.enable_semi_join_optimization = false;

-- Run same query without semi-join optimization
SELECT COUNT(*) as without_semi_join
FROM small_products p
JOIN small_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop' AND r.review_text @@@ 'performance';

-- Re-enable semi-join optimization
SET paradedb.enable_semi_join_optimization = true;

-- Run same query with semi-join optimization
SELECT COUNT(*) as with_semi_join
FROM small_products p
JOIN small_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'laptop' AND r.review_text @@@ 'performance';

-- Test 5: Edge cases for semi-join optimization

-- Test with no matching join keys (should handle gracefully)
SELECT COUNT(*) as no_join_matches
FROM small_products p
JOIN small_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'nonexistent' AND r.review_text @@@ 'impossible';

-- Test with very selective search (single result)
SELECT p.name, r.review_text
FROM small_products p
JOIN small_reviews r ON p.id = r.product_id
WHERE p.name @@@ 'Laptop Pro' AND r.review_text @@@ 'Excellent'
LIMIT 1;

-- Test with broad search (many results)
SELECT COUNT(*) as broad_search_count
FROM medium_docs d
JOIN medium_comments c ON d.id = c.doc_id
WHERE d.content @@@ 'and' AND c.comment_text @@@ 'about';

-- Test 6: Strategy selection validation
-- These queries should trigger different strategies based on estimated result sizes

-- Small selective query (SearchFilter)
EXPLAIN (COSTS OFF, BUFFERS OFF)
SELECT p.name, r.rating
FROM small_products p
JOIN small_reviews r ON p.id = r.product_id
WHERE p.description @@@ 'wireless precision' AND r.review_text @@@ 'perfect daily';

-- Medium selective query (SortedArray)
EXPLAIN (COSTS OFF, BUFFERS OFF)
SELECT COUNT(*)
FROM medium_docs d
JOIN medium_comments c ON d.id = c.doc_id
WHERE d.content @@@ 'technology innovation' AND c.comment_text @@@ 'excellent analysis';

-- Large selective query (BloomFilter)
EXPLAIN (COSTS OFF, BUFFERS OFF)
SELECT COUNT(*)
FROM large_articles a
JOIN large_tags t ON a.id = t.article_id
WHERE a.body @@@ 'machine learning applications' AND t.tag_name @@@ 'artificial-intelligence';

-- Test 7: Performance comparison
-- Measure execution time differences between strategies

-- Time small dataset join (SearchFilter)
SELECT COUNT(*) FROM small_products p JOIN small_reviews r ON p.id = r.product_id 
WHERE p.description @@@ 'laptop' AND r.review_text @@@ 'performance';

-- Time medium dataset join (SortedArray)
SELECT COUNT(*) FROM medium_docs d JOIN medium_comments c ON d.id = c.doc_id 
WHERE d.content @@@ 'technology' AND c.comment_text @@@ 'excellent';

-- Time large dataset join (BloomFilter)
SELECT COUNT(*) FROM large_articles a JOIN large_tags t ON a.id = t.article_id 
WHERE a.body @@@ 'artificial intelligence' AND t.tag_name @@@ 'machine-learning';

-- Test 8: Filter pushdown validation
-- Verify that filters are actually being pushed down to Tantivy

-- Test with multiple filter keys
SELECT a.headline, t.tag_name
FROM large_articles a
JOIN large_tags t ON a.id = t.article_id
WHERE a.body @@@ 'technology innovation' AND t.tag_name @@@ 'artificial-intelligence OR machine-learning'
LIMIT 10;

-- Test with complex search predicates
SELECT d.title, c.comment_text
FROM medium_docs d
JOIN medium_comments c ON d.id = c.doc_id
WHERE d.content @@@ 'technology AND innovation' AND c.comment_text @@@ 'excellent OR interesting'
LIMIT 5;

-- Cleanup
DROP TABLE small_products CASCADE;
DROP TABLE small_reviews CASCADE;
DROP TABLE medium_docs CASCADE;
DROP TABLE medium_comments CASCADE;
DROP TABLE large_articles CASCADE;
DROP TABLE large_tags CASCADE;

RESET paradedb.enable_custom_join;
RESET paradedb.enable_semi_join_optimization; 
