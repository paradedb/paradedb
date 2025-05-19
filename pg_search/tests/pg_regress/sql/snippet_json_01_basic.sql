\i common/snippet_json_basic_setup.sql

SELECT id,
       paradedb.snippet(metadata_jsonb->'details'->'author'->>'description'),
       paradedb.snippet_positions(metadata_jsonb->'details'->'author'->>'description')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_jsonb.details.author.description:test');

SELECT id,
       paradedb.snippet(metadata_jsonb#>'{details,author,description}'),
       paradedb.snippet_positions(metadata_jsonb#>'{details,author,description}')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_jsonb.details.author.description:test');

SELECT id,
       paradedb.snippet(metadata_jsonb#>>'{details,author,description}'),
       paradedb.snippet_positions(metadata_jsonb#>>'{details,author,description}')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_jsonb.details.author.description:test');

\i common/snippet_json_basic_cleanup.sql
