CREATE EXTENSION IF NOT EXISTS pg_bm25;

CREATE TABLE index_config AS SELECT * FROM paradedb.mock_items;
CREATE TABLE search_config AS SELECT * FROM paradedb.mock_items;
CREATE TABLE bm25_search AS SELECT * FROM paradedb.mock_items;

CREATE INDEX idxmockitems ON paradedb.mock_items USING bm25((mock_items.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');;
CREATE INDEX idxsearchconfig ON search_config USING bm25 ((search_config.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
CREATE INDEX idxbm25search ON bm25_search USING bm25 ((bm25_search.*)) WITH (text_fields='{"description": {}, "category": {}}', numeric_fields='{"rating": {}}', boolean_fields='{"in_stock": {}}', json_fields='{"metadata": {}}');
