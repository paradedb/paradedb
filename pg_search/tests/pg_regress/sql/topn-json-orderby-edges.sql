-- Additional coverage for JSON sub-path Top K pushdown (see topn-json-orderby.sql).
--
-- These cases exercise the JsonSortGate code paths that are not covered by the
-- primary regression file:
--   * JSON string value with a ::timestamp cast (probed leaf is Str, expected
--     is Date) -> pushdown rejected
--   * Float leaf type (Type::F64) with true decimal JSON numbers -> pushdown
--   * Probe miss when the requested JSON key is absent from every document
--   * Non-fast JSON field (fast: false) -> is_raw_sortable returns false
--   * Strict type map: ::int / ::float4 on I64 / F64 leaves -> pushdown rejected
--   * Mixed JSON leaf types within a single segment -> pushdown rejected
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

-- 4a. Explicit ::int cast on a JSON I64 leaf. PG int4 has a narrower domain than
--     Tantivy's I64; with LIMIT, pushdown would silently swallow cast errors for
--     values outside int4's range that fell outside the top K. The strict type
--     map rejects this and PG performs the sort itself.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'tier')::int AS tier
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY tier ASC, id ASC
LIMIT 5;

SELECT description, (metadata->>'tier')::int AS tier
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY tier ASC, id ASC
LIMIT 5;

-- 4b. Explicit ::real (float4) cast on a JSON F64 leaf. FLOAT4 has fewer mantissa
--     bits than F64, so ordering of close-magnitude values can diverge from
--     Tantivy's order. The strict type map rejects this and PG performs the sort.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'weight')::real AS weight
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY weight DESC, id ASC
LIMIT 5;

SELECT description, (metadata->>'weight')::real AS weight
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY weight DESC, id ASC
LIMIT 5;

-- 4c. Mixed leaf types within a single segment. When a JSON key has been
--     observed with both numeric and string values in the same segment,
--     Tantivy stores both an I64 column and a Str column for that path.
--     probe_segment_leaf_type detects this and returns None so the planner
--     falls back to a PG sort even when the cast type *would* otherwise
--     match the numeric leaf -- which is the case this gate exists to catch.
DROP TABLE IF EXISTS mock_items_jsonsort_mixed;
CREATE TABLE mock_items_jsonsort_mixed (
    id SERIAL PRIMARY KEY,
    description TEXT,
    metadata JSONB
);
INSERT INTO mock_items_jsonsort_mixed (description, metadata) VALUES
('Numeric ten',    '{"value": 10}'),
('Numeric twenty', '{"value": 20}'),
('Text alpha',     '{"value": "alpha"}');
CREATE INDEX jsonsort_mixed_idx ON mock_items_jsonsort_mixed
USING bm25 (id, description, metadata)
WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');

-- ::bigint cast: expected type (I64) would otherwise match the numeric leaf
-- this segment has for `value`. Only the mixed-segment detection prevents the
-- (unsafe) pushdown. EXPLAIN only because the PG-side sort would error
-- casting "alpha" to bigint.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'value')::bigint AS v
FROM mock_items_jsonsort_mixed
WHERE id @@@ paradedb.all()
ORDER BY v ASC, id ASC
LIMIT 5;

-- Raw ->> against the same mixed-type path. Expected type (Str) also fails
-- to match the numeric leaf, but this shape gives a deterministic PG-sorted
-- result row for end-to-end coverage.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'value' AS v
FROM mock_items_jsonsort_mixed
WHERE id @@@ paradedb.all()
ORDER BY v ASC, id ASC
LIMIT 5;

SELECT description, metadata->>'value' AS v
FROM mock_items_jsonsort_mixed
WHERE id @@@ paradedb.all()
ORDER BY v ASC, id ASC
LIMIT 5;

DROP TABLE mock_items_jsonsort_mixed;

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
--    (id). The full pathkey list must remain Top K compatible. Use ::bigint
--    since ::int is now rejected (see test 4a).
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id, description, (metadata->>'tier')::bigint AS tier
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY id ASC, (metadata->>'tier')::bigint ASC
LIMIT 5;

SELECT id, description, (metadata->>'tier')::bigint AS tier
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
ORDER BY id ASC, (metadata->>'tier')::bigint ASC
LIMIT 5;

-- 7. Aggregate scan path: GROUP BY ... ORDER BY (metadata->>'k')::cast LIMIT.
--    Confirms that aggregatescan/orderby.rs threads the JsonSortGate through
--    when the sort key is a cast over a JSON sub-path. Uses ::bigint for the
--    same reason as test 6.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT (metadata->>'tier')::bigint AS tier, COUNT(*)
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
GROUP BY (metadata->>'tier')::bigint
ORDER BY (metadata->>'tier')::bigint ASC
LIMIT 5;

SELECT (metadata->>'tier')::bigint AS tier, COUNT(*)
FROM mock_items_jsonsort_edges
WHERE id @@@ paradedb.all()
GROUP BY (metadata->>'tier')::bigint
ORDER BY (metadata->>'tier')::bigint ASC
LIMIT 5;

-- 8. JSON Array sub-path. A JSON array at the sub-path causes Tantivy to expose
--    a Multivalued fast field (e.g. Type::I64), but Postgres ->>/cast would
--    sort differently or error before LIMIT. The pushdown must be rejected.
DROP TABLE IF EXISTS mock_items_jsonsort_arrays;
CREATE TABLE mock_items_jsonsort_arrays (
    id SERIAL PRIMARY KEY,
    description TEXT,
    metadata JSONB
);
INSERT INTO mock_items_jsonsort_arrays (description, metadata) VALUES
('Item with array', '{"price_list": [10, 20]}'),
('Item with primitive', '{"price_list": 30}');
CREATE INDEX jsonsort_arrays_idx ON mock_items_jsonsort_arrays
USING bm25 (id, description, metadata)
WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, (metadata->>'price_list')::bigint AS price_list
FROM mock_items_jsonsort_arrays
WHERE id @@@ paradedb.all()
ORDER BY price_list ASC
LIMIT 5;

DROP TABLE mock_items_jsonsort_arrays;

-- 9. JSON Object sub-path. A JSON object at the sub-path causes Tantivy to expose
--    a primitive fast field (e.g. Type::Str) if another document has a string,
--    but Postgres ->>/cast would sort differently or error before LIMIT.
--    The pushdown must be rejected because it contains subpaths.
DROP TABLE IF EXISTS mock_items_jsonsort_objects;
CREATE TABLE mock_items_jsonsort_objects (
    id SERIAL PRIMARY KEY,
    description TEXT,
    metadata JSONB
);
INSERT INTO mock_items_jsonsort_objects (description, metadata) VALUES
('Item with object', '{"info": {"age": 30}}'),
('Item with primitive', '{"info": "some_string"}');
CREATE INDEX jsonsort_objects_idx ON mock_items_jsonsort_objects
USING bm25 (id, description, metadata)
WITH (key_field='id', json_fields='{"metadata": {"fast": true}}');

EXPLAIN (COSTS OFF, TIMING OFF)
SELECT description, metadata->>'info' AS info
FROM mock_items_jsonsort_objects
WHERE id @@@ paradedb.all()
ORDER BY info ASC
LIMIT 5;

DROP TABLE mock_items_jsonsort_objects;

DROP TABLE mock_items_jsonsort_edges;

\i common/common_cleanup.sql
