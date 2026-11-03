-- An all-inner join of three or more relations runs as a left-deep chain in
-- the order the planner's row estimates favour: the pair estimated smallest
-- joined first, then whichever keeps the joined set smallest. The query text
-- puts the two large tables first; the chain starts from books and the two
-- matching authors.
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE jo_authors (id INT PRIMARY KEY, name TEXT, bio TEXT);
CREATE TABLE jo_books (id INT PRIMARY KEY, author_id INT, title TEXT);
CREATE TABLE jo_reviews (id INT PRIMARY KEY, book_id INT, body TEXT, rating INT);

CREATE INDEX jo_authors_idx ON jo_authors USING bm25 (id, (name::pdb.literal), bio);
CREATE INDEX jo_books_idx ON jo_books USING bm25 (id, author_id, title);
CREATE INDEX jo_reviews_idx ON jo_reviews USING bm25 (id, book_id, body, rating);

-- One segment per table, so the plans are stable.
SET paradedb.global_mutable_segment_rows = 0;
INSERT INTO jo_authors
SELECT i, 'author ' || i,
       CASE WHEN i % 25 = 0 THEN 'rare orchid specialist' ELSE 'common prose writer' END
FROM generate_series(1, 50) i;
INSERT INTO jo_books
SELECT i, (i % 50) + 1,
       'book ' || i || CASE WHEN i % 3 = 0 THEN ' about gardens' ELSE ' about cities' END
FROM generate_series(1, 500) i;
INSERT INTO jo_reviews
SELECT i, (i % 500) + 1,
       CASE WHEN i % 7 = 0 THEN 'lovely read' ELSE 'tedious read' END, i % 5
FROM generate_series(1, 5000) i;
RESET paradedb.global_mutable_segment_rows;

ANALYZE jo_authors;
ANALYZE jo_books;
ANALYZE jo_reviews;

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

-- Books join the two matching authors first, then the reviews.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

-- The selective table written last. The chain still starts from it, and the
-- physical plan follows.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_reviews r
JOIN jo_books b ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_reviews r
JOIN jo_books b ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

-- A non-equi ON condition between books and reviews moves up to the join
-- that sees both.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id AND r.rating > b.author_id % 3
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id AND r.rating > b.author_id % 3
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

-- Two ON conditions that meet at one join after the rebuild land there as
-- one filter.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id AND r.rating > b.author_id % 3
JOIN jo_authors a ON a.id = b.author_id AND r.rating <> b.author_id % 5
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id AND r.rating > b.author_id % 3
JOIN jo_authors a ON a.id = b.author_id AND r.rating <> b.author_id % 5
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

-- An ON condition over three relations links the authors only once books and
-- reviews have joined. The planner estimates books and the two authors as the
-- smallest pair, but that pair has no join condition of its own, so the chain
-- does not start there.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id > b.author_id + r.rating
WHERE a.bio @@@ 'orchid'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id > b.author_id + r.rating
WHERE a.bio @@@ 'orchid'
ORDER BY b.id, r.id LIMIT 5;

-- Comma joins carry their keys in WHERE; they take the same order.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_books b, jo_reviews r, jo_authors a
WHERE r.book_id = b.id AND a.id = b.author_id
  AND a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_books b, jo_reviews r, jo_authors a
WHERE r.book_id = b.id AND a.id = b.author_id
  AND a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
ORDER BY b.id, r.id LIMIT 5;

-- An outer join keeps the parse-tree nesting.
EXPLAIN (COSTS OFF)
SELECT b.id, r.id
FROM jo_books b
LEFT JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid'
ORDER BY b.id, r.id LIMIT 5;

SELECT b.id, r.id
FROM jo_books b
LEFT JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid'
ORDER BY b.id, r.id LIMIT 5;

-- Two inputs of one join stay in the order written; DataFusion picks the
-- build side.
EXPLAIN (COSTS OFF)
SELECT a.id, b.id
FROM jo_authors a
JOIN jo_books b ON a.id = b.author_id
WHERE b.title @@@ 'gardens'
ORDER BY a.id, b.id LIMIT 5;

SELECT a.id, b.id
FROM jo_authors a
JOIN jo_books b ON a.id = b.author_id
WHERE b.title @@@ 'gardens'
ORDER BY a.id, b.id LIMIT 5;

-- The aggregate path builds the same chain.
EXPLAIN (COSTS OFF, VERBOSE)
SELECT a.name, COUNT(*)
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
GROUP BY a.name
ORDER BY a.name;

SELECT a.name, COUNT(*)
FROM jo_books b
JOIN jo_reviews r ON r.book_id = b.id
JOIN jo_authors a ON a.id = b.author_id
WHERE a.bio @@@ 'orchid' AND r.body @@@ 'lovely'
GROUP BY a.name
ORDER BY a.name;

RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;
RESET max_parallel_workers_per_gather;

DROP TABLE jo_reviews;
DROP TABLE jo_books;
DROP TABLE jo_authors;
