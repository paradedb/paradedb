\i common/snippet_json_advanced_setup.sql

WITH book_snippets AS (
    SELECT
        b.id as book_id,
        pdb.snippet(a.metadata->>'text') as author_snippet,
        pdb.snippet_positions(a.metadata->>'text') as author_positions,
        pdb.snippet(b.metadata->>'content') as book_content_snippet,
        pdb.snippet_positions(b.metadata->>'content') as book_content_positions,
        pdb.score(b.id) as book_score,
        pdb.score(a.id) as author_score
    FROM books b
    JOIN authors a ON b.author_id = a.id
    WHERE b.id @@@ paradedb.parse('metadata.content:test') OR a.id @@@ paradedb.parse('metadata.text:Harry')
)
SELECT
    bs.*,
    r.metadata->>'review' as review_text,
    pdb.snippet(r.metadata->>'review') as review_snippet,
    pdb.snippet_positions(r.metadata->>'review') as review_positions,
    pdb.score(r.id) as review_score
FROM book_snippets bs
LEFT JOIN reviews r ON r.book_id = bs.book_id
WHERE r.id @@@ paradedb.parse('metadata.review:test') AND r.id @@@ paradedb.parse('metadata.review:snippet')
ORDER BY bs.book_id, r.id;

-- Test comprehensive snippet functionality across all JSON fields
SELECT
    b.id as book_id,
    a.name as author_name,
    pdb.snippet(b.metadata->>'content') as book_snippet,
    pdb.snippet_positions(b.metadata->>'content') as book_positions,
    pdb.snippet(a.metadata->>'text') as author_snippet,
    pdb.snippet_positions(a.metadata->>'text') as author_positions,
    pdb.snippet(r.metadata->>'review') as review_snippet,
    pdb.snippet_positions(r.metadata->>'review') as review_positions,
    pdb.score(b.id) as book_score,
    pdb.score(a.id) as author_score,
    pdb.score(r.id) as review_score
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
    pdb.snippet(a.metadata->>'text') as text_snippet,
    pdb.snippet_positions(a.metadata->>'text') as text_positions,
    pdb.score(a.id) as author_score
FROM authors a
WHERE a.id @@@ paradedb.parse('metadata.text:author') AND a.id @@@ paradedb.parse('metadata.text:novels')
ORDER BY a.id;

-- Test snippet with JSON array fields (titles)
SELECT
    b.id,
    b.metadata->>'titles' as titles,
    pdb.snippet(b.metadata->>'content') as content_snippet,
    pdb.snippet_positions(b.metadata->>'content') as content_positions,
    pdb.score(b.id) as book_score
FROM books b
WHERE b.id @@@ paradedb.parse('metadata.content:function') OR b.id @@@ paradedb.parse('metadata.titles:test')
ORDER BY b.id;

-- Test complex JSON path queries with snippet
SELECT
    a.name as author_name,
    (a.metadata->>'age')::int as author_age,
    pdb.snippet(a.metadata->>'text') as author_bio_snippet,
    b.id as book_id,
    pdb.snippet(b.metadata->>'content') as book_content_snippet,
    r.id as review_id,
    pdb.snippet(r.metadata->>'review') as review_snippet,
    pdb.score(a.id) as author_score,
    pdb.score(b.id) as book_score,
    pdb.score(r.id) as review_score
FROM authors a
JOIN books b ON b.author_id = a.id
LEFT JOIN reviews r ON r.book_id = b.id
WHERE (a.id @@@ paradedb.parse('metadata.age:55'))
    AND (a.id @@@ paradedb.parse('metadata.text:author') OR b.id @@@ paradedb.parse('metadata.content:test'))
ORDER BY a.id, b.id, r.id;

\i common/snippet_json_advanced_cleanup.sql
