\i common/common_setup.sql

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_filter_pushdown = on;
-- The issue is about the Base Scan; `count(*)` would plan as the Aggregate Scan.
SET paradedb.enable_aggregate_custom_scan = off;

CREATE TABLE issue_6739_repro (
    id bigint PRIMARY KEY,
    body text,
    tags jsonb
);

INSERT INTO issue_6739_repro
SELECT i, 'search document ' || i, jsonb_build_array('tag_' || (i % 5))
FROM generate_series(1, 100) AS i;

-- tags is deliberately excluded from the ParadeDB index to trigger heap filter evaluation
CREATE INDEX issue_6739_repro_idx ON issue_6739_repro USING paradedb (id, body);

-- Test evaluating allocating functions like jsonb_build_array in heap filter predicates
EXPLAIN (COSTS OFF)
SELECT count(*) FROM issue_6739_repro
WHERE body @@@ 'document'
  AND tags @> jsonb_build_array('tag_1');

SELECT count(*) FROM issue_6739_repro
WHERE body @@@ 'document'
  AND tags @> jsonb_build_array('tag_1');

EXPLAIN (COSTS OFF)
SELECT count(*) FROM issue_6739_repro
WHERE body @@@ 'document'
  AND (
    tags @> jsonb_build_array('never1')
    OR tags @> jsonb_build_array('never2')
    OR tags @> jsonb_build_array('never3')
  );

SELECT count(*) FROM issue_6739_repro
WHERE body @@@ 'document'
  AND (
    tags @> jsonb_build_array('never1')
    OR tags @> jsonb_build_array('never2')
    OR tags @> jsonb_build_array('never3')
  );

DROP TABLE issue_6739_repro;

-- A generic plan solves `lib = $2` into the same context the DataFusion aggregate
-- evaluates the heap filter in, and each segment recompiles the filter from that
-- solved tree. So the filter's scratch memory must never be that context, or the
-- second segment reads a freed tree.
CREATE TABLE issue_6739_generic (id int PRIMARY KEY, body text, lib text);
CREATE INDEX issue_6739_generic_idx ON issue_6739_generic USING paradedb (id, body);

INSERT INTO issue_6739_generic
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       CASE WHEN g % 10 = 0 THEN 'rare' ELSE 'common' END
FROM generate_series(1, 500) g;
INSERT INTO issue_6739_generic
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       CASE WHEN g % 10 = 0 THEN 'rare' ELSE 'common' END
FROM generate_series(501, 1000) g;
INSERT INTO issue_6739_generic
SELECT g,
       repeat(CASE WHEN g % 4 = 0 THEN 'contrato de prestacao ' ELSE 'alpha beta servicos ' END
              || md5(g::text) || ' ', 1 + g % 8),
       CASE WHEN g % 10 = 0 THEN 'rare' ELSE 'common' END
FROM generate_series(1001, 1500) g;

SELECT count(*) > 1 AS several_segments FROM paradedb.index_info('issue_6739_generic_idx');

SET plan_cache_mode = force_generic_plan;
SET paradedb.enable_aggregate_custom_scan = on;

PREPARE issue_6739_p(text, text) AS
SELECT count(*) FROM (SELECT DISTINCT id FROM issue_6739_generic WHERE body @@@ $1 AND lib = $2) s;

EXPLAIN (COSTS OFF) EXECUTE issue_6739_p('servicos OR prestacao', 'rare');

-- different counts, so a stale solve of `$2` would show
EXECUTE issue_6739_p('servicos OR prestacao', 'rare');
EXECUTE issue_6739_p('servicos OR prestacao', 'common');

DEALLOCATE issue_6739_p;
RESET plan_cache_mode;
DROP TABLE issue_6739_generic;
