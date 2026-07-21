\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, ((metadata->>'color')::pdb.ngram(2, 3)))
WITH (key_field='id');

SELECT * FROM paradedb.schema('search_idx') ORDER BY name;

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*) FROM mock_items WHERE metadata->>'color' @@@ 'white';
SELECT COUNT(*) FROM mock_items WHERE metadata->>'color' @@@ 'white';

DROP TABLE mock_items;

-- Test Top-K optimization with an index expression projecting a field from JSON
CREATE TABLE json_topk_test (id SERIAL PRIMARY KEY, metadata JSONB, name TEXT);
INSERT INTO json_topk_test (metadata, name) VALUES ('{"rating": 10}', 'foo'), ('{"rating": 20}', 'foo'), ('{"rating": 30}', 'bar');

CREATE INDEX json_topk_idx ON json_topk_test
USING bm25 (id, name, (((metadata->>'rating')::int)::pdb.alias('rating')))
WITH (key_field='id', sort_by='rating DESC NULLS LAST');

-- EXPLAIN to check if TopKScanExecState is used
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT id, (metadata->>'rating')::int AS rating
FROM json_topk_test
WHERE name @@@ 'foo'
ORDER BY (metadata->>'rating')::int DESC NULLS LAST
LIMIT 2;

-- Verify results
SELECT id, (metadata->>'rating')::int AS rating
FROM json_topk_test
WHERE name @@@ 'foo'
ORDER BY (metadata->>'rating')::int DESC NULLS LAST
LIMIT 2;

DROP TABLE json_topk_test;
RESET paradedb.enable_aggregate_custom_scan;
