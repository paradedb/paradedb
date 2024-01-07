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
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Text field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {"fast": true, "tokenizer": { "type": "en_stem" }, "record": "freq", "normalizer": "raw"}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Multiple text fields
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {fast: true, tokenizer: { type: "en_stem" }, record: "freq", normalizer: "raw"}, category: {}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Default numeric field
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	numeric_fields => '{"rating": {}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Numeric field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	numeric_fields => '{"rating": {"fast": false}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Boolean field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	boolean_fields => '{"in_stock": {"fast": false}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Default Json field
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	json_fields => '{"metadata": {}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

-- Json field with options
CALL paradedb.create_bm25(
	index_name => 'index_config',
	table_name => 'index_config',
	schema_name => 'paradedb',
	key_field => 'id',
	json_fields => '{metadata: {fast: true, expand_dots: false, tokenizer: { type: "raw" }, normalizer: "raw"}}'
);
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');

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
SELECT * from index_config.schema();
CALL paradedb.drop_bm25('index_config', schema_name => 'paradedb');
