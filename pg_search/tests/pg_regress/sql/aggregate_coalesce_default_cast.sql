\i common/common_setup.sql

-- Tantivy casts a COALESCE default to each segment's column type, so a default is only pushed
-- down when every such column holds it exactly. Every query here must return the same value as
-- its reference at the end.

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO true;

DROP TABLE IF EXISTS cdc_int CASCADE;
DROP TABLE IF EXISTS cdc_float CASCADE;
DROP TABLE IF EXISTS cdc_json_int CASCADE;
DROP TABLE IF EXISTS cdc_json_none CASCADE;

CREATE TABLE cdc_int (id SERIAL8 PRIMARY KEY, g INTEGER, n INTEGER);
INSERT INTO cdc_int (g, n) VALUES (1, 4), (1, NULL);
CREATE INDEX cdc_int_idx ON cdc_int USING paradedb (id, g, n) WITH (key_field = 'id');

CREATE TABLE cdc_float (id SERIAL8 PRIMARY KEY, x DOUBLE PRECISION);
INSERT INTO cdc_float (x) VALUES (4), (NULL);
CREATE INDEX cdc_float_idx ON cdc_float USING paradedb (id, x) WITH (key_field = 'id');

-- Every score is an integer, so the segment's column is an integer column.
CREATE TABLE cdc_json_int (id SERIAL8 PRIMARY KEY, metadata JSONB);
INSERT INTO cdc_json_int (metadata) VALUES ('{"score": 4}'), ('{}');
CREATE INDEX cdc_json_int_idx ON cdc_json_int USING paradedb (id, (metadata::pdb.simple('columnar=true'))) WITH (key_field = 'id');

-- No row has a score, so the segment has no column for it. The brand column holds strings.
CREATE TABLE cdc_json_none (id SERIAL8 PRIMARY KEY, metadata JSONB);
INSERT INTO cdc_json_none (metadata) VALUES ('{"brand": "apple"}'), ('{}');
CREATE INDEX cdc_json_none_idx ON cdc_json_none USING paradedb (id, (metadata::pdb.simple('columnar=true'))) WITH (key_field = 'id');

-- A fractional default on an integer column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(n, 1.5)), SUM(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, 1.5)), SUM(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();

-- The same under GROUP BY.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT g, AVG(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all() GROUP BY g ORDER BY g;
SELECT g, AVG(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all() GROUP BY g ORDER BY g;

-- The same as a window aggregate in a top K query.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, AVG(COALESCE(n, 1.5)) OVER () FROM cdc_int WHERE id @@@ pdb.all() ORDER BY id LIMIT 5;
SELECT id, AVG(COALESCE(n, 1.5)) OVER () FROM cdc_int WHERE id @@@ pdb.all() ORDER BY id LIMIT 5;

-- A fractional default on a JSON path with integer values.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_int WHERE id @@@ pdb.all();

-- A fractional default on a JSON path the segment has no column for.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_none WHERE id @@@ pdb.all();
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_none WHERE id @@@ pdb.all();

-- A negative default on a JSON path the segment has no column for.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT SUM(COALESCE((metadata->>'score')::bigint, -1)) FROM cdc_json_none WHERE id @@@ pdb.all();
SELECT SUM(COALESCE((metadata->>'score')::bigint, -1)) FROM cdc_json_none WHERE id @@@ pdb.all();

-- Non-finite defaults on a float column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(x, 'NaN'::float8)), SUM(COALESCE(x, 'Infinity'::float8)) FROM cdc_float WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(x, 'NaN'::float8)), SUM(COALESCE(x, 'Infinity'::float8)) FROM cdc_float WHERE id @@@ pdb.all();

-- Still pushed down: a non-negative integer default on a JSON path.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE((metadata->>'score')::double precision, 2)) FROM cdc_json_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE((metadata->>'score')::double precision, 2)) FROM cdc_json_int WHERE id @@@ pdb.all();

-- Still pushed down: a negative default on an integer column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(n, -1)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, -1)) FROM cdc_int WHERE id @@@ pdb.all();

-- Still pushed down: a fractional default on a float column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(x, 1.5)) FROM cdc_float WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(x, 1.5)) FROM cdc_float WHERE id @@@ pdb.all();

-- Still pushed down: an integer default on an integer column, also as a NUMERIC with a scale.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(n, 2)), AVG(COALESCE(n, 1.0)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, 2)), AVG(COALESCE(n, 1.0)) FROM cdc_int WHERE id @@@ pdb.all();

-- Still pushed down: an integer default on an integer column in a window aggregate.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, AVG(COALESCE(n, 2)) OVER () FROM cdc_int WHERE id @@@ pdb.all() ORDER BY id LIMIT 5;
SELECT id, AVG(COALESCE(n, 2)) OVER () FROM cdc_int WHERE id @@@ pdb.all() ORDER BY id LIMIT 5;

-- Still pushed down: COUNT with a non-null default counts every row, whatever the column holds.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT COUNT(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(COALESCE(metadata->>'brand', '0')) FROM cdc_json_none WHERE id @@@ pdb.all();
SELECT COUNT(COALESCE(metadata->>'brand', '0')) FROM cdc_json_none WHERE id @@@ pdb.all();

-- The same queries without the ParadeDB operator or the aggregate scan, as the reference.
SET paradedb.enable_aggregate_custom_scan TO false;
SELECT AVG(COALESCE(n, 1.5)), SUM(COALESCE(n, 1.5)) FROM cdc_int;
SELECT g, AVG(COALESCE(n, 1.5)) FROM cdc_int GROUP BY g ORDER BY g;
SELECT id, AVG(COALESCE(n, 1.5)) OVER () FROM cdc_int ORDER BY id LIMIT 5;
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_int;
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_none;
SELECT SUM(COALESCE((metadata->>'score')::bigint, -1)) FROM cdc_json_none;
SELECT AVG(COALESCE(x, 'NaN'::float8)), SUM(COALESCE(x, 'Infinity'::float8)) FROM cdc_float;
SELECT AVG(COALESCE((metadata->>'score')::double precision, 2)) FROM cdc_json_int;
SELECT AVG(COALESCE(n, -1)) FROM cdc_int;
SELECT AVG(COALESCE(x, 1.5)) FROM cdc_float;
SELECT AVG(COALESCE(n, 2)), AVG(COALESCE(n, 1.0)) FROM cdc_int;
SELECT id, AVG(COALESCE(n, 2)) OVER () FROM cdc_int ORDER BY id LIMIT 5;
SELECT COUNT(COALESCE(n, 1.5)) FROM cdc_int;
SELECT COUNT(COALESCE(metadata->>'brand', '0')) FROM cdc_json_none;

DROP TABLE cdc_json_none;
DROP TABLE cdc_json_int;
DROP TABLE cdc_float;
DROP TABLE cdc_int;

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
