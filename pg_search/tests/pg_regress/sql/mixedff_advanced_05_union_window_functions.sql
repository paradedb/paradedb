-- Tests UNION and window functions with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: UNION operations'

-- This test is disabled, because it has a variable oid in it.
-- -- Test 1: Basic UNION with mixed field types
-- EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
-- SELECT title, author, rating, price
-- FROM union_test_a
-- WHERE title @@@ 'Book A' AND rating > 4
-- UNION
-- SELECT title, author, rating, price
-- FROM union_test_b
-- WHERE title @@@ 'Book B' AND rating > 3
-- ORDER BY rating DESC, title;

SELECT title, author, rating, price
FROM union_test_a
WHERE title @@@ 'Book A' AND rating > 4
UNION
SELECT title, author, rating, price
FROM union_test_b
WHERE title @@@ 'Book B' AND rating > 3
ORDER BY rating DESC, title;

-- Test 2: UNION ALL with numeric fields for filtering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, price, year
FROM union_test_a
WHERE price < 30 AND year > 2000 and title @@@ 'Book A'
UNION ALL
SELECT title, price, year
FROM union_test_b
WHERE price < 45 AND year > 1982 and title @@@ 'Book B'
ORDER BY price;

SELECT title, price, year
FROM union_test_a
WHERE price < 30 AND year > 2000 and title @@@ 'Book A'
UNION ALL
SELECT title, price, year
FROM union_test_b
WHERE price < 45 AND year > 1982 and title @@@ 'Book B'
ORDER BY price;

-- Test 3: Window function - ROW_NUMBER() with mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, price, rating,
       ROW_NUMBER() OVER (PARTITION BY author, price ORDER BY rating DESC) as author_rank
FROM union_test_a
WHERE title @@@ 'Book A'
ORDER BY title, author, author_rank;

SELECT title, author, price, rating,
       ROW_NUMBER() OVER (PARTITION BY author, price ORDER BY rating DESC) as author_rank
FROM union_test_a
WHERE title @@@ 'Book A'
ORDER BY title, author, author_rank;

-- Test 4: Window function - Running average price by author
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, price,
       AVG(price) OVER (PARTITION BY author ORDER BY price) as running_avg_price
FROM union_test_a
WHERE author @@@ 'Author'
ORDER BY title, author, price;

SELECT title, author, price,
       AVG(price) OVER (PARTITION BY author ORDER BY price) as running_avg_price
FROM union_test_a
WHERE author @@@ 'Author'
ORDER BY title, author, price;

-- This test is disabled, because it has a variable oid in it.
-- -- Test 5: Window function with UNION and mixed filters
-- EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
-- WITH combined_books AS (
--     SELECT title, author, rating, 'A' as source
--     FROM union_test_a
--     WHERE rating > 3.5 and title @@@ 'Book A'
--     UNION
--     SELECT title, author, rating, 'B' as source
--     FROM union_test_b
--     WHERE rating > 2.5 and title @@@ 'Book A'
-- )
-- SELECT title, author, rating, source,
--        RANK() OVER (PARTITION BY author ORDER BY rating DESC) as author_rank
-- FROM combined_books
-- ORDER BY title, author, author_rank;

WITH combined_books AS (
    SELECT title, author, rating, 'A' as source
    FROM union_test_a
    WHERE rating > 3.5 and title @@@ 'Book A'
    UNION
    SELECT title, author, rating, 'B' as source
    FROM union_test_b
    WHERE rating > 2.5 and title @@@ 'Book A'
)
SELECT title, author, rating, source,
       RANK() OVER (PARTITION BY author ORDER BY rating DESC) as author_rank
FROM combined_books
ORDER BY title, author, author_rank;

-- Test 6: UNION with boolean and text fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, is_published
FROM union_test_a
WHERE is_published = true AND author @@@ 'Author 1'
UNION
SELECT title, author, is_published
FROM union_test_b
WHERE is_published = true AND author @@@ 'Author 1'
ORDER BY author, title;

SELECT title, author, is_published
FROM union_test_a
WHERE is_published = true AND author @@@ 'Author 1'
UNION
SELECT title, author, is_published
FROM union_test_b
WHERE is_published = true AND author @@@ 'Author 1'
ORDER BY author, title;

-- Test 7: Window functions with multiple partitions and mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT author, 
       AVG(rating) as avg_rating,
       AVG(price) as avg_price,
       COUNT(*) as book_count,
       RANK() OVER (ORDER BY AVG(rating) DESC) as rating_rank,
       RANK() OVER (ORDER BY AVG(price)) as price_rank
FROM union_test_a
WHERE author @@@ 'Author'
GROUP BY author
ORDER BY avg_rating DESC;

SELECT author, 
       AVG(rating) as avg_rating,
       AVG(price) as avg_price,
       COUNT(*) as book_count,
       RANK() OVER (ORDER BY AVG(rating) DESC) as rating_rank,
       RANK() OVER (ORDER BY AVG(price)) as price_rank
FROM union_test_a
WHERE author @@@ 'Author'
GROUP BY author
ORDER BY avg_rating DESC;

-- This test is disabled, because it has a variable oid in it.
-- -- Test 8: UNION with INTERSECT and different field types
-- EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
-- (SELECT author FROM union_test_a WHERE rating > 4.5 and title @@@ 'Book A')
-- INTERSECT
-- (SELECT author FROM union_test_b WHERE rating > 4.0 and title @@@ 'Book A');

(SELECT author FROM union_test_a WHERE rating > 4.5 and title @@@ 'Book A')
INTERSECT
(SELECT author FROM union_test_b WHERE rating > 4.0 and title @@@ 'Book A');

-- Verify actual results of UNION (not just execution method)
SELECT title, author, rating, price
FROM union_test_a
WHERE title @@@ 'Book A1' AND rating > 4
UNION
SELECT title, author, rating, price
FROM union_test_b
WHERE title @@@ 'Book B1' AND rating > 3
ORDER BY rating DESC, title
LIMIT 10;

-- Verify window function results
SELECT title, author, price, rating,
       ROW_NUMBER() OVER (PARTITION BY author, price ORDER BY rating DESC) as author_rank
FROM union_test_a
WHERE author @@@ 'Author 1'
ORDER BY title, author, price
LIMIT 5;

\i common/mixedff_advanced_cleanup.sql
