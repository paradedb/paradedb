\i common/snippet_position_advanced_setup.sql

WITH book_snippets AS (
    SELECT
        b.id as book_id,
        paradedb.snippet(a.name) as author_snippet,
        paradedb.snippet_positions(a.name) as author_positions,
        paradedb.score(a.id) as author_score,
        paradedb.score(b.id) as book_score
    FROM books b
    JOIN authors a ON b.author_id = a.id
    WHERE b.content @@@ 'test' OR a.name @@@ 'Rowling'
)
SELECT
    bs.*,
    r.review,
    paradedb.snippet(r.review) as review_snippet,
    paradedb.snippet_positions(r.review) as review_positions,
    paradedb.score(r.id) as review_score
FROM book_snippets bs
LEFT JOIN reviews r ON r.book_id = bs.book_id
WHERE r.review @@@ 'test' AND r.review @@@ 'snippet'
ORDER BY bs.book_id, r.id;

SELECT
    b.id as book_id,
    paradedb.snippet(b.content) as book_snippet,
    paradedb.snippet_positions(b.content) as book_positions,
    paradedb.snippet(a.name) as author_snippet,
    paradedb.snippet_positions(a.name) as author_positions,
    paradedb.snippet(r.review) as review_snippet,
    paradedb.snippet_positions(r.review) as review_positions,
    paradedb.score(b.id) as book_score,
    paradedb.score(a.id) as author_score,
    paradedb.score(r.id) as review_score
FROM books b
JOIN authors a ON b.author_id = a.id
LEFT JOIN reviews r ON r.book_id = b.id
WHERE b.content @@@ 'test'
    OR a.name @@@ 'Rowling'
    OR r.review @@@ 'test'
    OR r.review @@@ 'snippet'
ORDER BY b.id, r.id;

\i common/snippet_position_advanced_cleanup.sql
