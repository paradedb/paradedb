CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

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

