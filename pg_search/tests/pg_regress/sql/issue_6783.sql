-- pdb.agg() OVER () over a partitioned table (#6783). Each partition has its own Base Scan,
-- and each scan used to answer with its own partition's aggregate: with a LIMIT the rows
-- came from the first partition and carried its count, and with an ORDER BY the
-- placeholder was computed above the Merge Append, where no scan replaces it. Every
-- partition's scan now answers with the aggregate over all of them.

\i common/common_setup.sql

DROP TABLE IF EXISTS docs CASCADE;
DROP TABLE IF EXISTS docs_plain CASCADE;
DROP TABLE IF EXISTS docs2 CASCADE;

CREATE TABLE docs (id int, body text, region int) PARTITION BY LIST (region);
CREATE TABLE docs_r0 PARTITION OF docs FOR VALUES IN (0);
CREATE TABLE docs_r1 PARTITION OF docs FOR VALUES IN (1);
INSERT INTO docs
SELECT i, CASE WHEN i % 4 < 2 THEN 'hello world' ELSE 'hello there' END, i % 2
FROM generate_series(1, 40) i;
CREATE INDEX docs_idx ON docs USING paradedb (id, body);

-- The same rows, unpartitioned.
CREATE TABLE docs_plain AS SELECT * FROM docs;
CREATE INDEX docs_plain_idx ON docs_plain USING paradedb (id, body);

-- Two levels: one partition is itself partitioned.
CREATE TABLE docs2 (id int, body text, region int, sub int) PARTITION BY LIST (region);
CREATE TABLE docs2_r0 PARTITION OF docs2 FOR VALUES IN (0) PARTITION BY LIST (sub);
CREATE TABLE docs2_r0_s0 PARTITION OF docs2_r0 FOR VALUES IN (0);
CREATE TABLE docs2_r0_s1 PARTITION OF docs2_r0 FOR VALUES IN (1);
CREATE TABLE docs2_r1 PARTITION OF docs2 FOR VALUES IN (1);
INSERT INTO docs2 SELECT id, body, region, (id / 2) % 2 FROM docs;
CREATE INDEX docs2_idx ON docs2 USING paradedb (id, body);

-- The issue's query, which returned 20, against the unpartitioned copy.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' LIMIT 3;
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs_plain WHERE body ||| 'hello' LIMIT 3;

-- With an ORDER BY, which failed: each partition's scan computes the aggregate now,
-- with no Result above the Merge Append.
EXPLAIN (COSTS OFF)
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' ORDER BY id LIMIT 3;
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' ORDER BY id LIMIT 3;
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' ORDER BY id DESC LIMIT 3;

-- A sum and a min, which returned the first partition's 420 and 2.
SELECT id, pdb.agg('{"sum":{"field":"id"}}') OVER (), pdb.agg('{"min":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' LIMIT 3;

-- A SQL aggregate beside pdb.agg() is computed the same way, and Postgres agrees with
-- the custom scan off.
SELECT id, count(*) OVER (), pdb.agg('{"max":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'world' ORDER BY id LIMIT 3;
SET paradedb.enable_custom_scan = off;
SELECT id, count(*) OVER (), max(id) OVER ()
FROM docs WHERE body ||| 'world' ORDER BY id LIMIT 3;
RESET paradedb.enable_custom_scan;

-- Two levels of partitions.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs2 WHERE body ||| 'hello' ORDER BY id LIMIT 3;

-- A filter each partition evaluates on the heap, and one that leaves a single partition.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' AND region IN (0, 1) AND id > 10 ORDER BY id LIMIT 3;
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' AND region = 1 ORDER BY id LIMIT 3;

-- A generic plan prunes a partition when the executor starts: the aggregate covers the
-- partition the parameter keeps.
PREPARE by_region(int) AS
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' AND region = $1 ORDER BY id LIMIT 3;
SET plan_cache_mode = force_generic_plan;
EXECUTE by_region(0);
EXECUTE by_region(1);
RESET plan_cache_mode;
DEALLOCATE by_region;

DROP TABLE docs2;
DROP TABLE docs_plain;
DROP TABLE docs;
