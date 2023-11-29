CREATE EXTENSION IF NOT EXISTS pg_bm25;

CALL paradedb.create_bm25_test_table(); -- creates table named "paradeb.bm25_test_table" by default

CALL paradedb.create_bm25_test_table(table_name => 'index_config', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'search_config', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'tokenizer_config', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');
CALL paradedb.create_bm25_test_table(table_name => 'aggregations', schema_name => 'paradedb');

CREATE INDEX idxmockitems ON paradedb.bm25_test_table USING bm25((bm25_test_table.*)) WITH (key_field='id', text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxsearchconfig ON search_config USING bm25 ((search_config.*)) WITH (key_field='id', text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxbm25search ON bm25_search USING bm25 ((bm25_search.*)) WITH (key_field='id', text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxaggregations ON bm25_search USING bm25 ((bm25_search.*)) WITH (key_field='id', text_fields='{"description": {"fast": true}, "category": {"fast": true}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
