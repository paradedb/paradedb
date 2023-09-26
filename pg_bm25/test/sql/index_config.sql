-- Invalid CREATE INDEX
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*));
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (invalid_field='{}');

-- Default text field
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (text_fields='{"description": {}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Text field with options
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (text_fields='{"description": {"fast": true, "tokenizer": "en_stem", "record": "freq", "normalizer": "raw"}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Multiple text fields
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (text_fields='{"description": {"fast": true, "tokenizer": "en_stem", "record": "freq", "normalizer": "raw"}, "category": {}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Default numeric field
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (numeric_fields='{"rating": {}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Numeric field with options
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (numeric_fields='{"rating": {"fast": false}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Default boolean field
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (boolean_fields='{"in_stock": {}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Boolean field with options
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (boolean_fields='{"in_stock": {"fast": false}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Default Json field
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (json_fields='{"metadata": {}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Json field with options
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (json_fields='{"metadata": {"fast": true, "expand_dots": false, "tokenizer": "raw", "normalizer": "raw"}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;

-- Multiple fields
CREATE INDEX idxindexconfig ON index_config USING bm25 ((index_config.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
SELECT * from paradedb.index_info('idxindexconfig');
DROP INDEX idxindexconfig;
