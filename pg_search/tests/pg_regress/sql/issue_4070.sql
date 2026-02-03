\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items USING bm25 (id, (description::pdb.literal), rating) WITH (key_field = id);
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * from mock_items where rating @@@ '4';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE rating @@@ 'IN [1 2]';
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE id @@@ pdb.all() AND rating = 4;
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE id @@@ pdb.all() AND rating IN (1, 2);

DROP TABLE mock_items;
