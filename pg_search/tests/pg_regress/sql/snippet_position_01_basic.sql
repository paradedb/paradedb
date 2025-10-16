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

\i common/snippet_position_basic_cleanup.sql
