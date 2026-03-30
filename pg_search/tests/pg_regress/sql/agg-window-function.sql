-- Minimal repro: pdb.agg() as a window function fails with
--   "pdb.agg() must be handled by ParadeDB's custom scan"
-- The planner places a WindowAgg node above the Custom Scan,
-- so pg_search never intercepts the pdb.agg() call.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS items CASCADE;

CREATE TABLE items (
    id SERIAL PRIMARY KEY,
    description TEXT,
    category TEXT
);

INSERT INTO items (description, category) VALUES
    ('red shoes',    'footwear'),
    ('blue shoes',   'footwear'),
    ('green jacket', 'apparel'),
    ('red hat',      'apparel'),
    ('blue scarf',   'apparel');

CREATE INDEX items_idx ON items
USING bm25 (id, description, (category::pdb.literal))
WITH (key_field = 'id');

-- This works: pdb.agg() as a regular aggregate
SELECT pdb.agg('{"terms": {"field": "category"}}'::jsonb)
FROM items
WHERE id @@@ pdb.all();

-- This fails: pdb.agg() OVER () without ORDER BY + LIMIT
SELECT
    id,
    description,
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM items
WHERE id @@@ pdb.all();

-- This works: pdb.agg() OVER () with ORDER BY indexed column + LIMIT
SELECT
    id,
    description,
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM items
WHERE id @@@ pdb.all()
ORDER BY id
LIMIT 3;

-- This fails: pdb.agg() OVER () with ORDER BY pdb.score() + LIMIT
-- This is the most natural search pattern (top results by relevance + facets)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT
    id,
    description,
    pdb.score(id) as score,
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM items
WHERE description @@@ pdb.all()
ORDER BY pdb.score(id) DESC
LIMIT 3;

SELECT
    id,
    description,
    pdb.score(id) as score,
    pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER ()
FROM items
WHERE description @@@ pdb.all()
ORDER BY pdb.score(id) DESC
LIMIT 3;

DROP TABLE items CASCADE;
