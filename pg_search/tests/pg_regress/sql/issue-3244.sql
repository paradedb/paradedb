CREATE EXTENSION IF NOT EXISTS pg_search;

SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS mock_items;

CREATE TABLE mock_items (
    id SERIAL PRIMARY KEY,
    metadata JSONB
);

CREATE INDEX mock_items_idx
ON mock_items
USING bm25 (id, metadata)
WITH (key_field = 'id');

INSERT INTO mock_items (metadata) VALUES
('{"color": "red"}'),
('{"color": "blue"}'),
('{"size": "large"}'),
('{"color": "green"}');

SELECT COUNT(metadata->>'color')
FROM mock_items
WHERE id @@@ paradedb.all();

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(metadata->>'color')
FROM mock_items
WHERE id @@@ paradedb.all();

RESET paradedb.enable_aggregate_custom_scan;
