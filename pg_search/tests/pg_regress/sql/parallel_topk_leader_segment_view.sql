\i common/common_setup.sql

-- A parallel Top K scan under a `Gather Merge` inside a hashed `SubPlan`. `Gather Merge` re-scans
-- its child on the first `ExecProcNode`, so the leader opens a second reader after it published
-- the shared segment view, while its claims still come out of that view.

SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET paradedb.min_rows_per_worker = 0;

DROP TABLE IF EXISTS psv_outer CASCADE;
DROP TABLE IF EXISTS psv_inner CASCADE;

CREATE TABLE psv_outer (id SERIAL8 PRIMARY KEY, age INTEGER);
CREATE TABLE psv_inner (id SERIAL8 PRIMARY KEY, uuid UUID, age INTEGER, rating INTEGER);

CREATE INDEX psv_outer_idx ON psv_outer USING paradedb (id, age)
WITH (numeric_fields = '{"age": {"fast": true}}');

-- No layers at all: a frozen or emptied mutable segment is a merge candidate on its own, and a
-- merge that collapsed the two segments would take the plan serial and the coverage with it.
CREATE INDEX psv_inner_idx ON psv_inner USING paradedb (id, uuid, age)
WITH (
    text_fields = '{"uuid": {"tokenizer": {"type": "keyword"}, "fast": true}}',
    numeric_fields = '{"age": {"fast": true}}',
    layer_sizes = '0',
    background_layer_sizes = '0'
);

-- Half the outer rows carry the age the inner Top K returns, so both the `IN` and the
-- `NOT IN` form below have rows to show.
INSERT INTO psv_outer (age)
SELECT CASE WHEN i % 2 = 0 THEN 20 ELSE (i % 30) + 5 END FROM generate_series(1, 10) i;

-- One sealed segment plus one mutable segment, so the leader's second reader has to replay a
-- mutable segment's materialization bound as well as the segment set.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO psv_inner (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, CASE WHEN i % 3 = 0 THEN 20 ELSE i END, CASE WHEN i % 3 = 0 THEN 4 ELSE 1 END
FROM generate_series(1, 6) i;
RESET paradedb.global_mutable_segment_rows;
INSERT INTO psv_inner (uuid, age, rating)
SELECT rpad(lpad((i * 7919)::text, 10, '0'), 32, '0')::uuid, CASE WHEN i % 3 = 0 THEN 20 ELSE i END, CASE WHEN i % 3 = 0 THEN 4 ELSE 1 END
FROM generate_series(7, 12) i;

ANALYZE psv_outer;
ANALYZE psv_inner;

-- The fixture is only useful while it holds both segment kinds, so record the composition.
SELECT count(*) AS segments, count(*) FILTER (WHERE mutable) AS mutable_segments
FROM paradedb.index_info('psv_inner_idx');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT COUNT(*) FROM psv_outer
WHERE NOT (psv_outer.age IN (
        SELECT age FROM psv_inner
        WHERE ((psv_inner.age IS NULL) OR (psv_inner.rating = 4)) AND (psv_inner.age @@@ '20')
        ORDER BY psv_inner.uuid ASC NULLS FIRST
        OFFSET 1 LIMIT 5))
  AND (psv_outer.age IS NOT NULL)
  AND (psv_outer.id @@@ pdb.all());

SELECT COUNT(*) FROM psv_outer
WHERE NOT (psv_outer.age IN (
        SELECT age FROM psv_inner
        WHERE ((psv_inner.age IS NULL) OR (psv_inner.rating = 4)) AND (psv_inner.age @@@ '20')
        ORDER BY psv_inner.uuid ASC NULLS FIRST
        OFFSET 1 LIMIT 5))
  AND (psv_outer.age IS NOT NULL)
  AND (psv_outer.id @@@ pdb.all());

-- The positive form of the same shape, so the rows the SubPlan produced are visible in the
-- output and not just their count.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, age FROM psv_outer
WHERE psv_outer.age IN (
        SELECT age FROM psv_inner
        WHERE ((psv_inner.age IS NULL) OR (psv_inner.rating = 4)) AND (psv_inner.age @@@ '20')
        ORDER BY psv_inner.uuid ASC NULLS FIRST
        LIMIT 5)
  AND (psv_outer.id @@@ pdb.all())
ORDER BY id;

SELECT id, age FROM psv_outer
WHERE psv_outer.age IN (
        SELECT age FROM psv_inner
        WHERE ((psv_inner.age IS NULL) OR (psv_inner.rating = 4)) AND (psv_inner.age @@@ '20')
        ORDER BY psv_inner.uuid ASC NULLS FIRST
        LIMIT 5)
  AND (psv_outer.id @@@ pdb.all())
ORDER BY id;

DROP TABLE psv_inner;
DROP TABLE psv_outer;

RESET paradedb.min_rows_per_worker;
RESET parallel_tuple_cost;
RESET parallel_setup_cost;
RESET max_parallel_workers_per_gather;
