-- Additional coverage for JSON sub-path Top K pushdown (see topn-json-orderby.sql).
--
-- These cases exercise the JsonSortGate code paths that are not covered by the
-- primary regression file:
--   * JSON string value with a ::timestamp cast (probed leaf is Str, expected
--     is Date) -> pushdown rejected
--   * Float leaf type (Type::F64) with true decimal JSON numbers -> pushdown
--   * Probe miss when the requested JSON key is absent from every document
--   * Non-fast JSON field (fast: false) -> is_raw_sortable returns false
--   * DESC sort direction on a Str leaf
--   * JSON sub-path as a secondary sort key
--   * Aggregate scan ORDER BY (metadata->>'k')::cast LIMIT (aggregatescan/orderby.rs)

\i common/common_setup.sql

SET paradedb.enable_aggregate_custom_scan = true;

DROP TABLE IF EXISTS mock_items_jsonsort_edges;
CREATE TABLE mock_items_jsonsort_edges (
    id SERIAL PRIMARY KEY,
    description TEXT,
    metadata JSONB,
    raw_metadata JSONB
);

INSERT INTO mock_items_jsonsort_edges (description, metadata, raw_metadata) VALUES
('Computer mouse', '{"price":100,   "color":"white","released":"2021-05-01","weight":1.5,  "tier":1}',
                    '{"hidden":"a"}'),
('Keyboard',       '{"price":150,   "color":"black","released":"2020-01-15","weight":2.25, "tier":2}',
                    '{"hidden":"b"}'),
('Monitor',        '{"price":200,   "color":"white","released":"2022-08-20","weight":7.0,  "tier":1}',
                    '{"hidden":"c"}'),
('Printer',        '{"price":120,   "color":"black","released":"2019-11-30","weight":4.4,  "tier":3}',
                    '{"hidden":"d"}'),
('Speaker',        '{"price":80,    "color":"white","released":"2023-03-10","weight":3.1,  "tier":2}',
                    '{"hidden":"e"}');

CREATE INDEX jsonsort_edges_idx ON mock_items_jsonsort_edges
USING bm25 (id, description, metadata, raw_metadata)
WITH (
    key_field='id',
    json_fields='{"metadata": {"fast": true}, "raw_metadata": {"fast": false}}'
);

-- 1. JSON string sorted via ::timestamp cast. Tantivy stores "released" as a
--    Str leaf inside the JSON object (JSON strings are not auto-promoted to
--    Date), but the SQL expression's expected leaf type is Date. The leaf
--    types disagree, so JsonSortGate refuses pushdown and Postgres performs
--    the sort itself. The final result is still in chronological order
--    because Postgres compares as timestamp.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'released')::timestamp AS released
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY released ASC
LIMIT 5;

SELECT description, (metadata->>'released')::timestamp AS released
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY released ASC
LIMIT 5;

-- 2. F64 leaf type. The "weight" key stores true decimals, so the column is an
--    F64 fast field and a ::float8 cast pushes down to Top K.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'weight')::float8 AS weight
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY weight DESC
LIMIT 5;

SELECT description, (metadata->>'weight')::float8 AS weight
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY weight DESC
LIMIT 5;

-- 3. Probe miss: no document defines metadata->>'nonexistent', so probe_json_leaf_type
--    returns None across every segment and the JsonSortGate rejects the pushdown.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'nonexistent' AS missing
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY missing ASC, id ASC
LIMIT 5;

-- 4. raw_metadata is declared with fast:false, so SearchField::is_raw_sortable
--    must return false and the JSON sort cannot be pushed down regardless of the
--    expression type.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, raw_metadata->>'hidden' AS h
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY h ASC, id ASC
LIMIT 5;

-- 5. DESC sort direction on a Str leaf -- confirms ORDER BY direction flows
--    through the new JSON pushdown branch.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'color' AS color
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY color DESC, id ASC
LIMIT 5;

SELECT description, metadata->>'color' AS color
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY color DESC, id ASC
LIMIT 5;

-- 6. JSON sub-path as a secondary sort key together with a primary fast field
--    (id). The full pathkey list must remain Top K compatible.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, description, (metadata->>'tier')::int AS tier
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY (metadata->>'tier')::int ASC, id ASC
LIMIT 5;

SELECT id, description, (metadata->>'tier')::int AS tier
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY (metadata->>'tier')::int ASC, id ASC
LIMIT 5;

-- 7. Aggregate scan path: GROUP BY ... ORDER BY (metadata->>'k')::cast LIMIT.
--    Confirms that aggregatescan/orderby.rs threads the JsonSortGate through
--    when the sort key is a cast over a JSON sub-path.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT (metadata->>'tier')::int AS tier, COUNT(*)
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
GROUP BY (metadata->>'tier')::int
ORDER BY (metadata->>'tier')::int ASC
LIMIT 5;

SELECT (metadata->>'tier')::int AS tier, COUNT(*)
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
GROUP BY (metadata->>'tier')::int
ORDER BY (metadata->>'tier')::int ASC
LIMIT 5;

DROP TABLE mock_items_jsonsort_edges;

\i common/common_cleanup.sql
