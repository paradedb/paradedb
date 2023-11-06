CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

SELECT paradedb.create_search_test_table();

CREATE TABLE mock_items AS SELECT * FROM paradedb.mock_items;
