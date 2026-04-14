CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS mock_items CASCADE;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'orders',
  table_type => 'Orders'
);

ALTER TABLE orders
ADD CONSTRAINT foreign_key_product_id
FOREIGN KEY (product_id)
REFERENCES mock_items(id);
