\i common/snippet_json_basic_setup.sql

SELECT id,
       pdb.snippet(metadata_jsonb->'details'->'author'->>'description'),
       pdb.snippet_positions(metadata_jsonb->'details'->'author'->>'description')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_jsonb.details.author.description:test');

SELECT id,
       pdb.snippet(metadata_jsonb#>'{details,author,description}'),
       pdb.snippet_positions(metadata_jsonb#>'{details,author,description}')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_jsonb.details.author.description:test');

SELECT id,
       pdb.snippet(metadata_jsonb#>>'{details,author,description}'),
       pdb.snippet_positions(metadata_jsonb#>>'{details,author,description}')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_jsonb.details.author.description:test');

SELECT id,
       pdb.snippet(metadata_json->'tags'),
       pdb.snippet_positions(metadata_json->'tags')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_json.tags:snippet');

SELECT id,
       pdb.snippet(metadata_json#>'{tags}'),
       pdb.snippet_positions(metadata_json#>'{tags}')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_json.tags:snippet');

SELECT id,
       pdb.snippet(metadata_json#>>'{tags}'),
       pdb.snippet_positions(metadata_json#>>'{tags}')
FROM snippet_test
WHERE id @@@ paradedb.parse('metadata_json.tags:snippet');

\i common/snippet_json_basic_cleanup.sql
