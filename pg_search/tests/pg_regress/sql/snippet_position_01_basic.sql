\i common/snippet_position_basic_setup.sql

SELECT id, pdb.snippet(content), pdb.snippet_positions(content)
FROM snippet_test
WHERE content @@@ 'test' OR content @@@ 'snippet';

SELECT id, pdb.snippet(titles), pdb.snippet_positions(titles)
FROM snippet_test
WHERE titles @@@ 'test' OR titles @@@ 'snippet';

SELECT id, pdb.snippet(content) as content_snippet, pdb.snippet_positions(content) as content_snippet_positions, pdb.snippet(titles) as titles_snippet, pdb.snippet_positions(titles) as titles_snippet_positions, pdb.score(id) as score
FROM snippet_test
WHERE titles @@@ 'test' OR content @@@ 'ipsum'
ORDER BY score DESC
LIMIT 5;

SELECT id, pdb.snippet(content) as content_snippet, pdb.snippet_positions(content) as content_snippet_positions, pdb.snippet(titles) as titles_snippet, pdb.snippet_positions(titles) as titles_snippet_positions, pdb.score(id) as score
FROM snippet_test
WHERE titles @@@ 'test' OR content @@@ 'ipsum'
ORDER BY id ASC
LIMIT 5;

-- Test accessing array elements: [i][1] for start, [i][2] for end
-- Note: PostgreSQL treats both integer[] and integer[][] as the same type (integer[] / _int4).
-- The dimensionality is stored in the array metadata, not the type system.
-- For 2D arrays: use array[i][j] for individual elements, or array[i:i][j:j] for slicing.
SELECT 
    id,
    pdb.snippet_positions(content) as all_positions,
    (pdb.snippet_positions(content))[1:1][1:2] as first_position,
    (pdb.snippet_positions(content))[1][1] as first_start,
    (pdb.snippet_positions(content))[1][2] as first_end,
    (pdb.snippet_positions(content))[2][1] as second_start,
    (pdb.snippet_positions(content))[2][2] as second_end
FROM snippet_test
WHERE content @@@ 'test'
ORDER BY id
LIMIT 3;

\i common/snippet_position_basic_cleanup.sql
