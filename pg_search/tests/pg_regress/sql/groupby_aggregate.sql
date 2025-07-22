-- Test GROUP BY functionality in aggregate custom scan

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS public.products;

-- Create test table with various data types
CALL paradedb.create_bm25_test_table(table_name => 'products', schema_name => 'public');

INSERT INTO public.products (description, rating, category, in_stock) VALUES
('Laptop with fast processor', 5, 'Electronics', true),
('Gaming laptop with RGB', 5, 'Electronics', true),
('Budget laptop for students', 3, 'Electronics', false),
('Wireless keyboard and mouse', 4, 'Electronics', true),
('Mechanical keyboard RGB', 5, 'Electronics', true),
('Ergonomic keyboard', 4, 'Electronics', false),
('Running shoes for athletes', 5, 'Sports', true),
('Casual shoes for walking', 4, 'Sports', true),
('Formal shoes leather', 3, 'Sports', false),
('Winter jacket warm', 4, 'Clothing', true),
('Summer jacket light', 3, 'Clothing', true),
('Rain jacket waterproof', 4, 'Clothing', false);

-- Create the index (id field must be included in the column list)
CREATE INDEX products_idx ON public.products 
USING bm25 (id, description, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}}',
    numeric_fields='{"rating": {"fast": true}}'
);

-- Test 1: Basic GROUP BY with COUNT(*)
SELECT rating, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'laptop' 
GROUP BY rating 
ORDER BY rating;

-- Test 2: GROUP BY with multiple search terms
SELECT rating, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'keyboard' 
GROUP BY rating 
ORDER BY rating DESC;

-- Test 3: GROUP BY with no matching results
SELECT rating, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'nonexistent' 
GROUP BY rating 
ORDER BY rating;

-- Test 4: GROUP BY with all matching results
SELECT rating, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'for' 
GROUP BY rating 
ORDER BY rating;

-- Test 5: Verify the aggregate scan is being used
EXPLAIN (COSTS OFF, VERBOSE) 
SELECT rating, COUNT(*) 
FROM public.products 
WHERE description @@@ 'shoes' 
GROUP BY rating;

-- Test 6: Test without GROUP BY (should still use aggregate scan)
SELECT COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'jacket';

-- Test 6b: Another non-GROUP BY aggregate to ensure it works
SELECT COUNT(*) AS total_laptops
FROM public.products 
WHERE description @@@ 'laptop';

-- Test 6c: Non-GROUP BY with broader search
SELECT COUNT(*) AS electronics_count
FROM public.products 
WHERE category = 'Electronics' AND description @@@ 'laptop OR keyboard OR mouse';

-- Test 7: Test GROUP BY without WHERE clause (should NOT use aggregate scan)
EXPLAIN (COSTS OFF, VERBOSE) 
SELECT rating, COUNT(*) 
FROM public.products 
GROUP BY rating;

-- Test 8: Test GROUP BY with non-fast field (should fail to use aggregate scan)
DROP INDEX products_idx;
CREATE INDEX products_idx ON public.products 
USING bm25 (id, description, rating)
WITH (
    key_field='id',
    text_fields='{"description": {}}',
    numeric_fields='{"rating": {"fast": false}}'
);

EXPLAIN (COSTS OFF, VERBOSE) 
SELECT rating, COUNT(*) 
FROM public.products 
WHERE description @@@ 'laptop' 
GROUP BY rating;

-- Test 9: GROUP BY on string field (category) - need to make it a fast field
DROP INDEX products_idx;
CREATE INDEX products_idx ON public.products 
USING bm25 (id, description, rating, category)
WITH (
    key_field='id',
    text_fields='{"description": {}, "category": {"fast": true}}',
    numeric_fields='{"rating": {"fast": true}}'
);

-- Test 9a: GROUP BY category with search
SELECT category, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'laptop OR keyboard' 
GROUP BY category 
ORDER BY category;

-- Test 9b: GROUP BY category - all electronics
SELECT category, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'laptop OR keyboard OR mouse' 
GROUP BY category 
ORDER BY count DESC, category;

-- Test 9c: GROUP BY category with specific search terms
SELECT category, COUNT(*) AS count
FROM public.products 
WHERE description @@@ 'jacket' 
GROUP BY category;

-- Test 9d: Verify aggregate scan is used for string GROUP BY
EXPLAIN (COSTS OFF, VERBOSE) 
SELECT category, COUNT(*) 
FROM public.products 
WHERE description @@@ 'shoes OR jacket' 
GROUP BY category;

-- Test 10: More realistic example - create a larger dataset
CREATE TABLE public.product_reviews (
    id SERIAL PRIMARY KEY,
    product_name TEXT,
    review_text TEXT,
    sentiment TEXT,
    rating INTEGER,
    reviewer_location TEXT
);

INSERT INTO public.product_reviews (product_name, review_text, sentiment, rating, reviewer_location) VALUES
('iPhone 15', 'Amazing camera quality and battery life', 'positive', 5, 'New York'),
('iPhone 15', 'Good phone but expensive', 'neutral', 4, 'California'),
('iPhone 15', 'Battery drains too fast', 'negative', 2, 'Texas'),
('Samsung Galaxy', 'Great display and features', 'positive', 5, 'New York'),
('Samsung Galaxy', 'Android is smooth and customizable', 'positive', 5, 'California'),
('Samsung Galaxy', 'Too many pre-installed apps', 'negative', 3, 'Florida'),
('MacBook Pro', 'Perfect for development work', 'positive', 5, 'California'),
('MacBook Pro', 'Expensive but worth it', 'positive', 4, 'New York'),
('MacBook Pro', 'Keyboard issues after one year', 'negative', 2, 'Texas'),
('Dell XPS', 'Good value for money', 'positive', 4, 'Florida'),
('Dell XPS', 'Linux compatibility is excellent', 'positive', 5, 'California'),
('Dell XPS', 'Customer support needs improvement', 'negative', 2, 'New York');

-- Create index with multiple fast text fields
CREATE INDEX reviews_idx ON public.product_reviews 
USING bm25 (id, product_name, review_text, sentiment, rating, reviewer_location)
WITH (
    key_field='id',
    text_fields='{
        "product_name": {"fast": true}, 
        "review_text": {}, 
        "sentiment": {"fast": true},
        "reviewer_location": {"fast": true}
    }',
    numeric_fields='{"rating": {"fast": true}}'
);

-- Test 10a: GROUP BY sentiment for battery-related reviews
SELECT sentiment, COUNT(*) AS review_count
FROM public.product_reviews 
WHERE review_text @@@ 'battery' 
GROUP BY sentiment 
ORDER BY sentiment;

-- Test 10b: GROUP BY product for positive reviews
SELECT product_name, COUNT(*) AS positive_reviews
FROM public.product_reviews 
WHERE review_text @@@ 'excellent OR amazing OR perfect' AND sentiment = 'positive'
GROUP BY product_name 
ORDER BY positive_reviews DESC;

-- Test 10c: GROUP BY location for laptop reviews
SELECT reviewer_location, COUNT(*) AS laptop_reviews
FROM public.product_reviews 
WHERE product_name @@@ 'macbook OR xps' 
GROUP BY reviewer_location 
ORDER BY laptop_reviews DESC;

-- Test 10d: Verify the plan for complex string GROUP BY
EXPLAIN (COSTS OFF, VERBOSE) 
SELECT product_name, sentiment, COUNT(*) 
FROM public.product_reviews 
WHERE review_text @@@ 'expensive' 
GROUP BY product_name, sentiment;

-- Note: This should fail as we don't support multi-column GROUP BY yet
-- But let's see what happens

-- Clean up
DROP TABLE public.product_reviews CASCADE;
DROP TABLE public.products CASCADE; 
