\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_bm25_test_table(
    schema_name => 'public',
    table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (description::pdb.simple('columnar=true')))
WITH (key_field = 'id');

SELECT * FROM paradedb.schema('search_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY description
LIMIT 5;

SELECT id, description FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY description
LIMIT 5;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT description, COUNT(*) FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY description
ORDER BY description
LIMIT 5;

SELECT description, COUNT(*) FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY description
ORDER BY description
LIMIT 5;

DROP INDEX search_idx;

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (lower(description)::pdb.simple('columnar=true')))
WITH (key_field = 'id');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, description FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY lower(description)
LIMIT 5;

SELECT id, description FROM mock_items
WHERE id @@@ pdb.all()
ORDER BY lower(description)
LIMIT 5;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT description, COUNT(*) FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY description
ORDER BY description
LIMIT 5;

SELECT description, COUNT(*) FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY description
ORDER BY description
LIMIT 5;

DROP INDEX search_idx;

CREATE INDEX search_idx ON mock_items
USING bm25 (id, (metadata::pdb.simple('columnar=true')))
WITH (key_field = 'id');

SELECT * FROM paradedb.schema('search_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT metadata->>'color', COUNT(*) FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY metadata->>'color'
ORDER BY metadata->>'color'
LIMIT 5;

SELECT metadata->>'color', COUNT(*) FROM mock_items
WHERE id @@@ pdb.all()
GROUP BY metadata->>'color'
ORDER BY metadata->>'color'
LIMIT 5;

DROP INDEX search_idx;
DROP TABLE mock_items;
