DROP EXTENSION IF EXISTS pg_search CASCADE;
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CALL paradedb.create_search_test_table();

CREATE TABLE IF NOT EXISTS mock_items AS SELECT * FROM paradedb.search_test_table;
