\i common/common_setup.sql

-- A segment only has a column for a JSON path when one of its documents carries the key.
-- Rows inserted after the build land in segments of their own, without that column, and
-- the scans have to read them as NULL, as Postgres does for a missing key.

SET paradedb.enable_custom_scan TO true;
SET paradedb.enable_custom_scan_without_operator TO true;
SET paradedb.enable_join_custom_scan TO true;
SET paradedb.enable_aggregate_custom_scan TO true;

DROP TABLE IF EXISTS ju CASCADE;
DROP TABLE IF EXISTS jp CASCADE;

CREATE TABLE ju (id SERIAL8 PRIMARY KEY, name TEXT, metadata JSONB);
CREATE TABLE jp (id SERIAL8 PRIMARY KEY, name TEXT);

INSERT INTO ju (name, metadata) SELECT 'bob', '{"brand": "apple"}' FROM generate_series(1, 20);
INSERT INTO jp (name) SELECT 'bob' FROM generate_series(1, 23);

CREATE INDEX ju_idx ON ju USING paradedb (id, (name::pdb.literal), (metadata::pdb.simple('columnar=true')));
CREATE INDEX jp_idx ON jp USING paradedb (id, (name::pdb.literal));

-- One row in a mutable segment, one in an immutable segment, neither with a "brand" key.
INSERT INTO ju (name, metadata) VALUES ('bob', '{}');
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO ju (name, metadata) VALUES ('bob', '{"other": 1}');
RESET paradedb.global_mutable_segment_rows;

-- The join scan evaluates `IS NOT NULL` itself and groups by the JSON path.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

-- The same query with the ordinals and the strings resolved in the scan, which reads the
-- column through the named arm instead of the deferred one.
SET paradedb.defer_column_fetch TO off;
SET paradedb.defer_string_decode TO off;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;
RESET paradedb.defer_string_decode;
RESET paradedb.defer_column_fetch;

-- The same query with the ordinals resolved above the join, in `TantivyFetchExec`.
SET paradedb.defer_column_fetch TO on;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;
RESET paradedb.defer_column_fetch;

-- The same, grouped by the jsonb value.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT ju.metadata->'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT ju.metadata->'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

-- An outer join with a predicate the scan can't push down.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*), ju.metadata->'brand'
FROM ju FULL JOIN jp ON ju.id = jp.id
WHERE (ju.name @@@ 'bob') OR ((ju.name IS NULL) AND (ju.name @@@ 'bob'))
GROUP BY ju.metadata->'brand'
ORDER BY 2;

SELECT count(*), ju.metadata->'brand'
FROM ju FULL JOIN jp ON ju.id = jp.id
WHERE (ju.name @@@ 'bob') OR ((ju.name IS NULL) AND (ju.name @@@ 'bob'))
GROUP BY ju.metadata->'brand'
ORDER BY 2;

-- A right join null-extends a `jp` row with no `ju` match, so the NULL group mixes a
-- null-extended row with the two rows whose segment has no column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT ju.metadata->>'brand', count(jp.id)
FROM ju RIGHT JOIN jp ON ju.id = jp.id
WHERE jp.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT ju.metadata->>'brand', count(jp.id)
FROM ju RIGHT JOIN jp ON ju.id = jp.id
WHERE jp.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

-- The same queries without the custom scans, as the reference.
SET paradedb.enable_custom_scan TO false;
SET paradedb.enable_join_custom_scan TO false;
SET paradedb.enable_aggregate_custom_scan TO false;

SELECT ju.metadata->>'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT ju.metadata->'brand', count(*)
FROM ju JOIN jp ON ju.id = jp.id
WHERE ju.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

SELECT count(*), ju.metadata->'brand'
FROM ju FULL JOIN jp ON ju.id = jp.id
WHERE (ju.name = 'bob') OR ((ju.name IS NULL) AND (ju.name = 'bob'))
GROUP BY ju.metadata->'brand'
ORDER BY 2;

SELECT ju.metadata->>'brand', count(jp.id)
FROM ju RIGHT JOIN jp ON ju.id = jp.id
WHERE jp.name IS NOT NULL
GROUP BY 1
ORDER BY 1;

DROP TABLE jp;
DROP TABLE ju;

RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_custom_scan_without_operator;
RESET paradedb.enable_custom_scan;
