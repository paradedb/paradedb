\i common/snippet_position_basic_setup.sql

SELECT id, paradedb.snippet(content), paradedb.snippet_positions(content)
FROM snippet_test
WHERE content @@@ 'test' OR content @@@ 'snippet';

SELECT id, paradedb.snippet(titles), paradedb.snippet_positions(titles)
FROM snippet_test
WHERE titles @@@ 'test' OR titles @@@ 'snippet';

SELECT id, paradedb.snippet(metadata, 'test'), paradedb.snippet_positions(metadata, 'test')
FROM snippet_test
WHERE metadata @@@ 'test';

\i common/snippet_position_basic_cleanup.sql
