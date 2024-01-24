DROP EXTENSION IF EXISTS pg_bm25;
CREATE EXTENSION IF NOT EXISTS pg_bm25;

CALL paradedb.create_bm25_test_table(); -- creates table named "paradeb.bm25_test_table" by default

CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'search_config', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'aggregations', schema_name => 'paradedb');

CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
    key_field => 'id',
    text_fields => '{"description": {}, "category": {}}',
	numeric_fields => '{"rating": {}}',
	boolean_fields => '{"in_stock": {}}',
	json_fields => '{"metadata": {}}'
);

CALL paradedb.create_bm25(
	index_name => 'search_config',
	table_name => 'search_config',
	schema_name => 'paradedb',
    key_field => 'id',
    text_fields => '{"description": {}, "category": {}}',
	numeric_fields => '{"rating": {}}',
	boolean_fields => '{"in_stock": {}}',
	json_fields => '{"metadata": {}}'
);

CALL paradedb.create_bm25(
	index_name => 'bm25_search',
	table_name => 'bm25_search',
	schema_name => 'paradedb',
    key_field => 'id',
    text_fields => '{"description": {}, "category": {}}',
	numeric_fields => '{"rating": {}}',
	boolean_fields => '{"in_stock": {}}',
	json_fields => '{"metadata": {}}'
);

CALL paradedb.create_bm25(
	index_name => 'aggregations',
	table_name => 'aggregations',
	schema_name => 'paradedb',
    key_field => 'id',
    text_fields => '{"description": {"fast": true}, "category": {"fast": true}}',
	numeric_fields => '{"rating": {}}',
	boolean_fields => '{"in_stock": {}}',
	json_fields => '{"metadata": {}}'
);

