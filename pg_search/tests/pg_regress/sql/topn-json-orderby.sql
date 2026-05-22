\i common/common_setup.sql

DROP TABLE IF EXISTS mock_items_jsonsort;
CREATE TABLE mock_items_jsonsort (
    id SERIAL PRIMARY KEY,
    description TEXT,
    metadata JSONB
);

INSERT INTO mock_items_jsonsort (description, metadata) VALUES
('Computer mouse', '{"price": 100, "color": "white", "in_stock": true,  "released": "2021-05-01"}'),
('Keyboard',       '{"price": 150, "color": "black", "in_stock": false, "released": "2020-01-15"}'),
('Monitor',        '{"price": 200, "color": "white", "in_stock": true,  "released": "2022-08-20"}'),
('Printer',        '{"price": 120, "color": "black", "in_stock": false, "released": "2019-11-30"}'),
('Speaker',        '{"price":  80, "color": "white", "in_stock": true,  "released": "2023-03-10"}');

CREATE INDEX jsonsort_idx ON mock_items_jsonsort
USING bm25 (id, description, metadata)
WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');

-- Text sub-key (color) -> Str fast field. ORDER BY metadata->>'color' is TEXT,
-- and Tantivy stores it as Str, so pushdown is safe.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' AS color FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY color ASC, id ASC
LIMIT 5;

SELECT description, metadata->>'color' AS color FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY color ASC, id ASC
LIMIT 5;

-- Numeric sub-key sorted as TEXT must NOT push down to Tantivy: PG's lexicographic
-- text sort ('80', '200', '150', '120', '100' DESC) differs from Tantivy's numeric
-- order. The planner gate rejects the JSON sort, so PG performs the sort and the
-- output reflects text ordering.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'price' AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

SELECT description, metadata->>'price' AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

-- Explicit ::int cast on the same key matches the I64 fast field type, so pushdown
-- is allowed and the result is in numeric order.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'price')::int AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

SELECT description, (metadata->>'price')::int AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

-- Explicit ::numeric cast maps to F64 expected type, but the stored fast field is
-- I64 (integer JSON values). The probe disagrees, so pushdown is rejected and PG
-- performs the sort itself.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'price')::numeric AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

SELECT description, (metadata->>'price')::numeric AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

-- Boolean cast matches the Bool fast field type.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'in_stock')::boolean AS in_stock FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY in_stock DESC, id ASC
LIMIT 5;

SELECT description, (metadata->>'in_stock')::boolean AS in_stock FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY in_stock DESC, id ASC
LIMIT 5;

DROP TABLE mock_items_jsonsort;

\i common/common_cleanup.sql
