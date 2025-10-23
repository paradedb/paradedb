\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items_1'
);

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items_2'
);

CREATE INDEX mock_items_1_idx ON mock_items_1
USING bm25 (id, description, category)
WITH (key_field='id');

CREATE INDEX mock_items_2_idx ON mock_items_2
USING bm25 (id, description, category)
WITH (key_field='id');

SELECT relname, layer_size FROM pdb.index_layer_info WHERE relname = 'mock_items_1_idx' OR relname = 'mock_items_2_idx';
SELECT * FROM paradedb.combined_layer_sizes('mock_items_1_idx');
SELECT * FROM paradedb.combined_layer_sizes('mock_items_2_idx');

ALTER INDEX mock_items_1_idx SET (layer_sizes = '0');
ALTER INDEX mock_items_1_idx SET (background_layer_sizes = '10kb, 100kb, 1mb, 100mb');

SELECT relname, layer_size FROM pdb.index_layer_info WHERE relname = 'mock_items_1_idx' OR relname = 'mock_items_2_idx';
SELECT * FROM paradedb.combined_layer_sizes('mock_items_1_idx');

ALTER INDEX mock_items_1_idx SET (layer_sizes = '10kb, 100kb');
ALTER INDEX mock_items_1_idx SET (background_layer_sizes = '10kb, 100kb, 1mb, 100mb, 1gb');

SELECT relname, layer_size FROM pdb.index_layer_info WHERE relname = 'mock_items_1_idx' OR relname = 'mock_items_2_idx';
SELECT * FROM paradedb.combined_layer_sizes('mock_items_1_idx');

DROP TABLE mock_items_1;
DROP TABLE mock_items_2;
