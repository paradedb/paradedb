-- Default tokenizer
CALL paradedb.create_bm25(
	index_name => 'tokenizer_config',
	table_name => 'tokenizer_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {}}'
);
SELECT * FROM tokenizer_config.search('description:earbud');
CALL paradedb.drop_bm25('tokenizer_config');

-- en_stem
CALL paradedb.create_bm25(
	index_name => 'tokenizer_config',
	table_name => 'tokenizer_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {"tokenizer": { "type": "en_stem" }}}'
);
SELECT * FROM tokenizer_config.search('description:earbud');
CALL paradedb.drop_bm25('tokenizer_config');

-- ngram
CALL paradedb.create_bm25(
	index_name => 'tokenizer_config',
	table_name => 'tokenizer_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {"tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 8, "prefix_only": false}}}'
);

SELECT * FROM tokenizer_config.search('description:boa');
CALL paradedb.drop_bm25('tokenizer_config');

-- chinese_compatible
CALL paradedb.create_bm25(
	index_name => 'tokenizer_config',
	table_name => 'tokenizer_config',
	schema_name => 'paradedb',
	key_field => 'id',
	text_fields => '{"description": {"tokenizer": {"type": "chinese_compatible"}, "record": "position"}}'
);
INSERT INTO tokenizer_config (description, rating, category) VALUES ('电脑', 4, 'Electronics');
SELECT * FROM tokenizer_config.search('description:电脑');
CALL paradedb.drop_bm25('tokenizer_config');

