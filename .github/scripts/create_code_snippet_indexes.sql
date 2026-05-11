CREATE INDEX search_idx ON mock_items
USING bm25 (
  id,
  description,
  category,
  rating,
  in_stock,
  created_at,
  metadata,
  weight_range,
  (description::pdb.simple('alias=description_simple')),
  (lower(description)::pdb.literal('alias=literal_description'))
)
WITH (
  key_field = 'id',
  json_fields = '{"metadata":{"fast":true}}'
);

CREATE INDEX orders_idx ON orders
USING bm25 (order_id, product_id, order_quantity, order_total, customer_name)
WITH (key_field = 'order_id');
