CREATE EXTENSION IF NOT EXISTS pg_bm25;

SELECT paradedb.create_bm25_test_table();

CREATE TABLE index_config AS SELECT * FROM paradedb.bm25_test_table;
CREATE TABLE search_config AS SELECT * FROM paradedb.bm25_test_table;
CREATE TABLE tokenizer_config AS SELECT * FROM paradedb.bm25_test_table;
CREATE TABLE bm25_search AS SELECT * FROM paradedb.bm25_test_table;
CREATE TABLE aggregations AS SELECT * FROM paradedb.bm25_test_table;

CREATE INDEX idxmockitems ON paradedb.bm25_test_table USING bm25((bm25_test_table.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxsearchconfig ON search_config USING bm25 ((search_config.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxbm25search ON bm25_search USING bm25 ((bm25_search.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxaggregations ON bm25_search USING bm25 ((bm25_search.*)) WITH (text_fields='{"description": {"fast": true}, "category": {"fast": true}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
