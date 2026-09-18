\i common/common_setup.sql

-- A SubPlan over a parallel Base Scan. The final shutdown can reach the parallel subtree
-- twice: on PostgreSQL 15 through the outer Base Scan's filter, and on every version through
-- its target list. The second shutdown must not read the shared memory that the
-- `Gather Merge` freed in the first one.

SET max_parallel_workers_per_gather = 2;
-- Keep the count on a plain Aggregate over the Base Scan.
SET paradedb.enable_aggregate_custom_scan = off;

DROP TABLE IF EXISTS sp_outer CASCADE;
DROP TABLE IF EXISTS sp_inner CASCADE;

CREATE TABLE sp_outer (id SERIAL8 PRIMARY KEY, age INTEGER);
CREATE TABLE sp_inner (id SERIAL8 PRIMARY KEY, uuid UUID, age INTEGER, rating INTEGER);

CREATE INDEX sp_outer_idx ON sp_outer USING paradedb (id, age)
WITH (numeric_fields = '{"age": {"fast": true}}');

CREATE INDEX sp_inner_idx ON sp_inner USING paradedb (id, uuid, age)
WITH (
    text_fields = '{"uuid": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"age": {"fast": true}}'
);

-- Each insert makes its own segment. The `rating` heap filter makes the inner Top K scan
-- uncostable, so two segments always give it one worker. Both segments hold inner matches,
-- so the SubPlan result is not empty.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO sp_outer (age) SELECT (i % 30) + 5 FROM generate_series(1, 20) i;
INSERT INTO sp_inner (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid,
       CASE WHEN i % 3 = 0 THEN 20 ELSE i END,
       CASE WHEN i % 3 = 0 THEN 4 ELSE (i % 5) + 1 END
FROM generate_series(1, 6) i;
INSERT INTO sp_inner (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid,
       CASE WHEN i % 3 = 0 THEN 20 ELSE i END,
       CASE WHEN i % 3 = 0 THEN 4 ELSE (i % 5) + 1 END
FROM generate_series(7, 12) i;
RESET paradedb.global_mutable_segment_rows;

ANALYZE sp_outer;
ANALYZE sp_inner;

SELECT COUNT(*) AS segments FROM paradedb.index_info('sp_inner_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*) FROM sp_outer
WHERE NOT (sp_outer.age IN (
        SELECT age FROM sp_inner
        WHERE ((sp_inner.age IS NULL) OR (sp_inner.rating = 4)) AND (sp_inner.age @@@ '20')
        ORDER BY sp_inner.uuid ASC NULLS FIRST
        OFFSET 1 LIMIT 5))
  AND (sp_outer.age IS NOT NULL)
  AND (sp_outer.id @@@ pdb.all());

SELECT COUNT(*) FROM sp_outer
WHERE NOT (sp_outer.age IN (
        SELECT age FROM sp_inner
        WHERE ((sp_inner.age IS NULL) OR (sp_inner.rating = 4)) AND (sp_inner.age @@@ '20')
        ORDER BY sp_inner.uuid ASC NULLS FIRST
        OFFSET 1 LIMIT 5))
  AND (sp_outer.age IS NOT NULL)
  AND (sp_outer.id @@@ pdb.all());

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT sp_outer.id,
       sp_outer.age IN (
           SELECT age FROM sp_inner
           WHERE ((sp_inner.age IS NULL) OR (sp_inner.rating = 4)) AND (sp_inner.age @@@ '20')
           ORDER BY sp_inner.uuid ASC NULLS FIRST
           OFFSET 1 LIMIT 5) AS in_inner
FROM sp_outer
WHERE sp_outer.id @@@ pdb.all()
ORDER BY sp_outer.id;

SELECT sp_outer.id,
       sp_outer.age IN (
           SELECT age FROM sp_inner
           WHERE ((sp_inner.age IS NULL) OR (sp_inner.rating = 4)) AND (sp_inner.age @@@ '20')
           ORDER BY sp_inner.uuid ASC NULLS FIRST
           OFFSET 1 LIMIT 5) AS in_inner
FROM sp_outer
WHERE sp_outer.id @@@ pdb.all()
ORDER BY sp_outer.id;

DROP TABLE sp_inner;
DROP TABLE sp_outer;

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
