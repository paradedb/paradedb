CREATE INDEX search_idx ON mock_items
USING paradedb (
  id,
  description,
  category,
  rating,
  in_stock,
  created_at,
  metadata,
  weight_range,
  embedding vector_cosine_ops,
  (description::pdb.simple('alias=description_simple')),
  (lower(description)::pdb.literal('alias=literal_description'))
)
WITH (
  key_field = 'id',
  json_fields = '{"metadata":{"fast":true}}',
  vector_fields = '{"embedding":{"dims":8,"quantization":false}}'
);

CREATE INDEX orders_idx ON orders
USING paradedb (order_id, product_id, order_quantity, order_total, customer_name)
WITH (key_field = 'order_id');
