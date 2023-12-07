-- Invalid create_bm25
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config'
);
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	key_field => 'id'
);
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	key_field => 'id',
	invalid_field => '{}'		
);

-- Default text field
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Text field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {"fast": true, "tokenizer": { "type": "en_stem" }, "record": "freq", "normalizer": "raw"}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Multiple text fields
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {fast: true, tokenizer: { type: "en_stem" }, record: "freq", normalizer: "raw"}, category: {}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Default numeric field
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	numeric_fields => '{"rating": {}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Numeric field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	numeric_fields => '{"rating": {"fast": false}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Default boolean field
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	boolean_fields => '{"in_stock": {}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Boolean field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	boolean_fields => '{"in_stock": {"fast": false}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Default Json field
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	json_fields => '{"metadata": {}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Json field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	json_fields => '{metadata: {fast: true, expand_dots: false, tokenizer: { type: "raw" }, normalizer: "raw"}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');

-- Multiple fields
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{description: {}, category: {}}',
	numeric_fields => '{rating: {}}',
	boolean_fields => '{in_stock: {}}',
	json_fields => '{metadata: {}}'
);
SELECT * from paradedb.schema_bm25('index_config_bm25_index');
CALL paradedb.drop_bm25('index_config');
