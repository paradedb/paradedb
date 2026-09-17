\i common/common_setup.sql

-- A hashed SubPlan in a Base Scan's filter, over a parallel Top K scan. The final shutdown
-- walks the SubPlans of the outer scan, and the parallel subtree under it must be shut down
-- only once, while its shared memory still exists.

SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET paradedb.min_rows_per_worker = 0;

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

-- Each insert makes its own segment, so the inner scan has segments to split across workers.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO sp_outer (age) SELECT (i % 30) + 5 FROM generate_series(1, 10) i;
INSERT INTO sp_inner (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, CASE WHEN i % 3 = 0 THEN 20 ELSE i END, (i % 5) + 1
FROM generate_series(1, 6) i;
INSERT INTO sp_inner (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, CASE WHEN i % 3 = 0 THEN 20 ELSE i END, (i % 5) + 1
FROM generate_series(7, 12) i;
RESET paradedb.global_mutable_segment_rows;

ANALYZE sp_outer;
ANALYZE sp_inner;

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

DROP TABLE sp_inner;
DROP TABLE sp_outer;

RESET paradedb.min_rows_per_worker;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET max_parallel_workers_per_gather;
