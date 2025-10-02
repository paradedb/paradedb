\i common/snippet_json_advanced_setup.sql

WITH book_snippets AS (
    SELECT
        b.id as book_id,
        paradedb.snippet(a.metadata->>'text') as author_snippet,
        paradedb.snippet_positions(a.metadata->>'text') as author_positions,
        paradedb.snippet(b.metadata->>'content') as book_content_snippet,
        paradedb.snippet_positions(b.metadata->>'content') as book_content_positions,
        paradedb.score(b.id) as book_score,
        paradedb.score(a.id) as author_score
    FROM books b
    JOIN authors a ON b.author_id = a.id
    WHERE b.id @@@ paradedb.parse('metadata.content:test') OR a.id @@@ paradedb.parse('metadata.text:Harry')
)
SELECT
    bs.*,
    r.metadata->>'review' as review_text,
    paradedb.snippet(r.metadata->>'review') as review_snippet,
    paradedb.snippet_positions(r.metadata->>'review') as review_positions,
    paradedb.score(r.id) as review_score
FROM book_snippets bs
LEFT JOIN reviews r ON r.book_id = bs.book_id
WHERE r.id @@@ paradedb.parse('metadata.review:test') AND r.id @@@ paradedb.parse('metadata.review:snippet')
ORDER BY bs.book_id, r.id;

-- Test comprehensive snippet functionality across all JSON fields
SELECT
    b.id as book_id,
    a.name as author_name,
    paradedb.snippet(b.metadata->>'content') as book_snippet,
    paradedb.snippet_positions(b.metadata->>'content') as book_positions,
    paradedb.snippet(a.metadata->>'text') as author_snippet,
    paradedb.snippet_positions(a.metadata->>'text') as author_positions,
    paradedb.snippet(r.metadata->>'review') as review_snippet,
    paradedb.snippet_positions(r.metadata->>'review') as review_positions,
    paradedb.score(b.id) as book_score,
    paradedb.score(a.id) as author_score,
    paradedb.score(r.id) as review_score
FROM books b
JOIN authors a ON b.author_id = a.id
LEFT JOIN reviews r ON r.book_id = b.id
WHERE b.id @@@ paradedb.parse('metadata.content:test')
    OR a.id @@@ paradedb.parse('metadata.text:fantasy')
    OR r.id @@@ paradedb.parse('metadata.review:test')
    OR r.id @@@ paradedb.parse('metadata.review:snippet')
ORDER BY b.id, r.id;

-- Test snippet with multiple search terms in JSON fields
SELECT
    a.name,
    a.metadata->>'age' as age,
    paradedb.snippet(a.metadata->>'text') as text_snippet,
    paradedb.snippet_positions(a.metadata->>'text') as text_positions,
    paradedb.score(a.id) as author_score
FROM authors a
WHERE a.id @@@ paradedb.parse('metadata.text:author') AND a.id @@@ paradedb.parse('metadata.text:novels')
ORDER BY a.id;

-- Test snippet with JSON array fields (titles)
SELECT
    b.id,
    b.metadata->>'titles' as titles,
    paradedb.snippet(b.metadata->>'content') as content_snippet,
    paradedb.snippet_positions(b.metadata->>'content') as content_positions,
    paradedb.score(b.id) as book_score
FROM books b
WHERE b.id @@@ paradedb.parse('metadata.content:function') OR b.id @@@ paradedb.parse('metadata.titles:test')
ORDER BY b.id;

-- Test complex JSON path queries with snippet
SELECT
    a.name as author_name,
    (a.metadata->>'age')::int as author_age,
    paradedb.snippet(a.metadata->>'text') as author_bio_snippet,
    b.id as book_id,
    paradedb.snippet(b.metadata->>'content') as book_content_snippet,
    r.id as review_id,
    paradedb.snippet(r.metadata->>'review') as review_snippet,
    paradedb.score(a.id) as author_score,
    paradedb.score(b.id) as book_score,
    paradedb.score(r.id) as review_score
FROM authors a
JOIN books b ON b.author_id = a.id
LEFT JOIN reviews r ON r.book_id = b.id
WHERE (a.id @@@ paradedb.parse('metadata.age:55'))
    AND (a.id @@@ paradedb.parse('metadata.text:author') OR b.id @@@ paradedb.parse('metadata.content:test'))
ORDER BY a.id, b.id, r.id;

\i common/snippet_json_advanced_cleanup.sql
