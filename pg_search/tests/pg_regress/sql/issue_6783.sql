-- Issue #6783: `pdb.agg() OVER ()` with a `LIMIT` over a partitioned table returned
-- one partition's aggregate. The planner hook replaced the window function with the
-- placeholder, each partition's Base Scan computed the aggregate over its own index,
-- and the `Limit` above the `Append` took its rows from the first partition. A plain
-- `pdb.agg()` aggregate over the same table already declines with an error, and the
-- window form now declines the same way.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET client_min_messages = warning;

DROP TABLE IF EXISTS docs CASCADE;
DROP TABLE IF EXISTS docs_plain CASCADE;
DROP TABLE IF EXISTS docs2 CASCADE;
CREATE TABLE docs (id int, body text, region int) PARTITION BY LIST (region);
CREATE TABLE docs_r0 PARTITION OF docs FOR VALUES IN (0);
CREATE TABLE docs_r1 PARTITION OF docs FOR VALUES IN (1);
INSERT INTO docs SELECT i, 'hello world', i % 2 FROM generate_series(1, 40) i;
CREATE INDEX docs_idx ON docs USING paradedb (id, body);
CREATE TABLE docs_plain AS SELECT * FROM docs;
CREATE INDEX docs_plain_idx ON docs_plain USING paradedb (id, body);
-- A two level tree: the parent's members are themselves partitioned.
CREATE TABLE docs2 (id int, body text, region int, sub int) PARTITION BY LIST (region);
CREATE TABLE docs2_r0 PARTITION OF docs2 FOR VALUES IN (0) PARTITION BY LIST (sub);
CREATE TABLE docs2_r0_s0 PARTITION OF docs2_r0 FOR VALUES IN (0);
CREATE TABLE docs2_r0_s1 PARTITION OF docs2_r0 FOR VALUES IN (1);
CREATE TABLE docs2_r1 PARTITION OF docs2 FOR VALUES IN (1);
INSERT INTO docs2 SELECT i, 'hello world', i % 2, (i / 2) % 2 FROM generate_series(1, 40) i;
CREATE INDEX docs2_idx ON docs2 USING paradedb (id, body);

-- The unpartitioned copy works, and counts every row.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs_plain WHERE body ||| 'hello' ORDER BY id LIMIT 3;

-- The issue's query: returned 20, one partition's count.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' LIMIT 3;

-- The same with an ORDER BY: failed opening the parent index's storage.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' ORDER BY id LIMIT 3;

-- A sum: returned 420, one partition's sum of 820.
SELECT id, pdb.agg('{"sum":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' LIMIT 3;

-- Pruned to one partition, the answer was that partition's, which happened to be
-- right; it declines like the plain aggregate does.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs WHERE body ||| 'hello' AND region = 0 LIMIT 3;

-- Two levels: only one leaf ran Top K, so the placeholder reached the executor.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs2 WHERE body ||| 'hello' LIMIT 3;

-- The plain aggregate's existing error, for comparison.
SELECT pdb.agg('{"value_count":{"field":"id"}}') FROM docs WHERE body ||| 'hello';

-- A SQL window function over the partitioned table is not replaced and runs natively.
SELECT id, count(*) OVER () FROM docs WHERE body ||| 'hello' ORDER BY id LIMIT 3;

-- FROM ONLY names the partitioned table itself, which holds no rows; it declines too.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM ONLY docs WHERE body ||| 'hello' LIMIT 3;

-- The partitioned table behind a derived table or a CTE, with the window function
-- outside: the planner pulls the subquery up after the hook, so the same scans ran.
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM (SELECT id, body FROM docs WHERE body ||| 'hello') s LIMIT 3;
WITH c AS (SELECT id, body FROM docs WHERE body ||| 'hello')
SELECT id, pdb.agg('{"value_count":{"field":"id"}}') OVER () FROM c LIMIT 3;

-- A join that includes the partitioned table declines the same way.
SELECT p.id, pdb.agg('{"value_count":{"field":"id"}}') OVER ()
FROM docs_plain p JOIN docs d ON p.id = d.id WHERE p.body ||| 'hello' ORDER BY p.id LIMIT 3;

DROP TABLE docs2;
DROP TABLE docs_plain;
DROP TABLE docs;
