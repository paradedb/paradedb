CREATE EXTENSION IF NOT EXISTS svector;
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;

CALL paradedb.create_search_test_table();

CREATE TABLE mock_items AS SELECT * FROM paradedb.search_test_table;
