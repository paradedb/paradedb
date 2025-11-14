\i common/common_setup.sql

DROP TABLE IF EXISTS mock_items_jsonsort;
CREATE TABLE mock_items_jsonsort (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT,
    rating INT,
    in_stock BOOLEAN,
    created_at TIMESTAMP,
    weight_range NUMRANGE,
    metadata JSONB
);

INSERT INTO mock_items_jsonsort (description, metadata) VALUES
('Computer mouse', '{"price": 100, "color": "white", "in_stock": true}'),
('Keyboard', '{"price": 150, "color": "black", "in_stock": false}'),
('Monitor', '{"price": 200, "color": "white", "in_stock": true}'),
('Printer', '{"price": 120, "color": "black", "in_stock": false}'),
('Speaker', '{"price": 80, "color": "white", "in_stock": true}');

CREATE INDEX jsonsort_idx ON mock_items_jsonsort
USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');

-- Order by JSON key (price) in DESC using fast ordering
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'price' AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

SELECT description, metadata->>'price' AS price FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY price DESC
LIMIT 5;

-- Order by JSON key (color) ASC using fast ordering
EXPLAIN (FORMAT TEXT, VERBOSE, COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' AS color FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY color ASC, id ASC
LIMIT 5;

SELECT description, metadata->>'color' AS color FROM mock_items_jsonsort
WHERE id @@@ paradedb.all()
ORDER BY color ASC, id ASC
LIMIT 5;

DROP TABLE mock_items_jsonsort;

\i common/common_cleanup.sql