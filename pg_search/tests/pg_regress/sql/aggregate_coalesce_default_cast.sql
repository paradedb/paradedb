\i common/common_setup.sql

-- Tantivy casts a COALESCE default to each segment's column type, so a default is only pushed
-- down when every such column holds it exactly. Every query here must match the run with the
-- aggregate scan off.

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO true;

DROP TABLE IF EXISTS cdc_int CASCADE;
DROP TABLE IF EXISTS cdc_float CASCADE;
DROP TABLE IF EXISTS cdc_json_int CASCADE;
DROP TABLE IF EXISTS cdc_json_none CASCADE;

CREATE TABLE cdc_int (id SERIAL8 PRIMARY KEY, n INTEGER);
INSERT INTO cdc_int (n) VALUES (4), (NULL);
CREATE INDEX cdc_int_idx ON cdc_int USING paradedb (id, n);

CREATE TABLE cdc_float (id SERIAL8 PRIMARY KEY, x DOUBLE PRECISION);
INSERT INTO cdc_float (x) VALUES (4), (NULL);
CREATE INDEX cdc_float_idx ON cdc_float USING paradedb (id, x);

-- Every score is an integer, so the segment's column is an integer column.
CREATE TABLE cdc_json_int (id SERIAL8 PRIMARY KEY, metadata JSONB);
INSERT INTO cdc_json_int (metadata) VALUES ('{"score": 4}'), ('{}');
CREATE INDEX cdc_json_int_idx ON cdc_json_int USING paradedb (id, (metadata::pdb.simple('columnar=true')));

-- No row has a score, so the segment has no column for it.
CREATE TABLE cdc_json_none (id SERIAL8 PRIMARY KEY, metadata JSONB);
INSERT INTO cdc_json_none (metadata) VALUES ('{"brand": "apple"}');
CREATE INDEX cdc_json_none_idx ON cdc_json_none USING paradedb (id, (metadata::pdb.simple('columnar=true')));

-- A fractional default on an integer column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(n, 1.5)), SUM(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, 1.5)), SUM(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();

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

-- Still pushed down: an integer default on an integer column.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT AVG(COALESCE(n, 2)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, 2)) FROM cdc_int WHERE id @@@ pdb.all();

-- Still pushed down: COUNT does not use the default.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT COUNT(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();

-- The same results without the aggregate scan, as the reference.
SET paradedb.enable_aggregate_custom_scan TO false;
SELECT AVG(COALESCE(n, 1.5)), SUM(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE((metadata->>'score')::double precision, 1.5)) FROM cdc_json_none WHERE id @@@ pdb.all();
SELECT SUM(COALESCE((metadata->>'score')::bigint, -1)) FROM cdc_json_none WHERE id @@@ pdb.all();
SELECT AVG(COALESCE((metadata->>'score')::double precision, 2)) FROM cdc_json_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, -1)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(x, 1.5)) FROM cdc_float WHERE id @@@ pdb.all();
SELECT AVG(COALESCE(n, 2)) FROM cdc_int WHERE id @@@ pdb.all();
SELECT COUNT(COALESCE(n, 1.5)) FROM cdc_int WHERE id @@@ pdb.all();

DROP TABLE cdc_json_none;
DROP TABLE cdc_json_int;
DROP TABLE cdc_float;
DROP TABLE cdc_int;

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
