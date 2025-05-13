\i common/snippet_position_basic_setup.sql

SELECT id, paradedb.snippet(content), paradedb.snippet_positions(content)
FROM snippet_test
WHERE content @@@ 'test' OR content @@@ 'snippet';

SELECT id, paradedb.snippet(titles), paradedb.snippet_positions(titles)
FROM snippet_test
WHERE titles @@@ 'test' OR titles @@@ 'snippet';

SELECT id, paradedb.snippet(content) as content_snippet, paradedb.snippet_positions(content) as content_snippet_positions, paradedb.snippet(titles) as titles_snippet, paradedb.snippet_positions(titles) as titles_snippet_positions, paradedb.score(id) as score
FROM snippet_test
WHERE titles @@@ 'test' OR content @@@ 'ipsum'
ORDER BY score DESC
LIMIT 5;

SELECT id, paradedb.snippet(content) as content_snippet, paradedb.snippet_positions(content) as content_snippet_positions, paradedb.snippet(titles) as titles_snippet, paradedb.snippet_positions(titles) as titles_snippet_positions, paradedb.score(id) as score
FROM snippet_test
WHERE titles @@@ 'test' OR content @@@ 'ipsum'
ORDER BY id ASC
LIMIT 5;

\i common/snippet_position_basic_cleanup.sql
