-- Tests Reciprocal Rank Fusion on joined tables

\i common/common_setup.sql

-- Create test tables
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items_rrf',
  table_type => 'Items'
);
CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'orders_rrf',
  table_type => 'Orders'
);

ALTER TABLE orders_rrf
ADD CONSTRAINT foreign_key_product_id
FOREIGN KEY (product_id)
REFERENCES mock_items_rrf(id);

CREATE INDEX orders_rrf_idx ON orders_rrf
USING bm25 (order_id, product_id, order_quantity, order_total, customer_name)
WITH (key_field = 'order_id');

CREATE INDEX mock_items_rrf_idx ON mock_items_rrf
USING bm25 (id, description, category, rating)
WITH (key_field = 'id');

-- RRF Query
PREPARE rrf_query(text, text) AS
WITH order_search AS (
  SELECT order_id, RANK() OVER (ORDER BY score DESC) AS rank
  FROM (
    SELECT order_id, pdb.score(order_id) AS score
    FROM orders_rrf
    WHERE customer_name ||| $1
    ORDER BY pdb.score(order_id) DESC
    LIMIT 20
  )
),
product_search AS (
  SELECT o.order_id, RANK() OVER (ORDER BY score DESC) AS rank
  FROM (
    SELECT id, pdb.score(id) AS score
    FROM mock_items_rrf
    WHERE description ||| $2
    ORDER BY pdb.score(id) DESC
    LIMIT 20
  ) m
  JOIN orders_rrf o ON o.product_id = m.id
),
rrf AS (
  SELECT order_id, 1.0 / (60 + rank) AS s FROM order_search
  UNION ALL
  SELECT order_id, 1.0 / (60 + rank) AS s FROM product_search
)
SELECT
  o.order_id,
  o.customer_name,
  m.description,
  sum(rrf.s) AS score
FROM rrf
JOIN orders_rrf o USING (order_id)
JOIN mock_items_rrf m ON o.product_id = m.id
GROUP BY o.order_id, o.customer_name, m.description
ORDER BY score DESC, o.order_id
LIMIT 5;

EXPLAIN (COSTS OFF)
EXECUTE rrf_query('Johnson', 'running shoes');

EXECUTE rrf_query('Johnson', 'running shoes');


-- Cleanup
DEALLOCATE rrf_query;
DROP TABLE orders_rrf CASCADE;
DROP TABLE mock_items_rrf CASCADE;
