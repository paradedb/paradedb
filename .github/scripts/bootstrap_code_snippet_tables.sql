CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS mock_items CASCADE;

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX mock_items_bm25_idx ON mock_items
USING bm25 (
  id,
  description,
  category,
  rating,
  in_stock,
  created_at,
  metadata,
  weight_range,
  (description::pdb.simple('alias=description_simple'))
)
WITH (
  key_field = 'id',
  json_fields = '{"metadata":{"fast":true}}'
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

CREATE INDEX orders_idx ON orders
USING bm25 (order_id, product_id, order_quantity, order_total, customer_name)
WITH (key_field = 'order_id');
