-- Tests score function behavior with mixed fast fields

\i common/mixedff_advanced_setup.sql

\echo 'Test: Score function behavior'

-- Test 1: Basic score function with text field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, score(score_test_idx), rating
FROM score_test
WHERE content @@@ 'technology'
ORDER BY score(score_test_idx) DESC
LIMIT 10;

-- Test 2: Score function with mixed field types in selection
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, rating, views, score(score_test_idx)
FROM score_test
WHERE content @@@ 'research'
ORDER BY rating DESC, score(score_test_idx) DESC
LIMIT 5;

-- Test 3: Score function with multiple conditions on different field types
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, score(score_test_idx)
FROM score_test
WHERE content @@@ 'technology' AND rating >= 4 AND is_featured = true
ORDER BY score(score_test_idx) DESC;

-- Test 4: Using score in a CTE with mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH scored_posts AS (
    SELECT title, author, rating, score(score_test_idx) as relevance
    FROM score_test
    WHERE content @@@ 'science OR research'
)
SELECT title, author, rating, relevance
FROM scored_posts
WHERE rating > 3
ORDER BY relevance DESC
LIMIT 10;

-- Test 5: Score function in subquery with mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT sp.title, sp.author, sp.relevance
FROM (
    SELECT title, author, score(score_test_idx) as relevance
    FROM score_test
    WHERE content @@@ 'technology' AND rating > 3
) sp
WHERE sp.relevance > 0.5
ORDER BY sp.relevance DESC;

-- Test 6: Score function with UNION
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT title, author, score(score_test_idx) as relevance
FROM score_test
WHERE content @@@ 'technology'
UNION ALL
SELECT title, author, score(score_test_idx) as relevance
FROM score_test
WHERE content @@@ 'science' AND NOT (title @@@ 'technology')
ORDER BY relevance DESC
LIMIT 10;

-- Test 7: Score function with JOIN using mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT a.title, a.author, a.rating, a.score, b.title as related_title
FROM (
    SELECT title, author, rating, score(score_test_idx) as score
    FROM score_test
    WHERE content @@@ 'technology'
    ORDER BY score DESC
    LIMIT 5
) a
JOIN (
    SELECT title, author
    FROM score_test
    WHERE author IN (SELECT author FROM score_test WHERE content @@@ 'technology')
) b ON a.author = b.author AND a.title <> b.title;

-- Test 8: Score function with CASE expression and mixed fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT 
    title, 
    author,
    rating,
    CASE 
        WHEN score(score_test_idx) > 0.8 THEN 'High Relevance'
        WHEN score(score_test_idx) > 0.5 THEN 'Medium Relevance'
        ELSE 'Low Relevance'
    END as relevance_category
FROM score_test
WHERE content @@@ 'research OR development'
ORDER BY score(score_test_idx) DESC
LIMIT 10;

-- Verify actual results of score function (not just execution method)
SELECT title, author, rating, score(score_test_idx) as relevance
FROM score_test
WHERE content @@@ 'technology' AND rating >= 4
ORDER BY relevance DESC
LIMIT 5;

-- Test combination of score function and different fast field types
SELECT title, author, rating, views, score(score_test_idx) as relevance
FROM score_test
WHERE content @@@ 'research' AND rating > 3 AND views > 1000
ORDER BY relevance DESC
LIMIT 3;

\i common/mixedff_advanced_cleanup.sql
