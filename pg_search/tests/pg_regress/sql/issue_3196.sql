\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan TO on;

CALL paradedb.create_paradedb_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX on mock_items USING paradedb (id, description, rating, category, (metadata::pdb.literal));

INSERT INTO mock_items (rating, metadata) VALUES (null, null), (null, null);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

SELECT COUNT(*) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(rating) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

SELECT COUNT(rating) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(metadata->>'color') FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

SELECT COUNT(metadata->>'color') FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(COALESCE(rating, 0)) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

SELECT COUNT(COALESCE(rating, 0)) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(COALESCE(metadata->>'color', 'red')) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

SELECT COUNT(COALESCE(metadata->>'color', 'red')) FROM mock_items WHERE id @@@ pdb.all()::pdb.const(0);

DROP TABLE mock_items;
