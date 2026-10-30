CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6932_t (id int PRIMARY KEY, body text, grp int4, w int8);
CREATE TABLE issue_6932_e (id int PRIMARY KEY, tag text);
INSERT INTO issue_6932_t VALUES
    (1, 'x', 1, 9223372036854775807),
    (2, 'x', 1, 9223372036854775807),
    (3, 'x', 2, -9223372036854775808),
    (4, 'x', 2, -9223372036854775808),
    (5, 'x', 3, 5),
    (6, 'x', 3, NULL);
INSERT INTO issue_6932_e SELECT id, 'x' FROM issue_6932_t;
CREATE INDEX issue_6932_t_idx ON issue_6932_t USING paradedb (id, body, grp, w);
CREATE INDEX issue_6932_e_idx ON issue_6932_e USING paradedb (id, tag);

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

-- sum(int8) over a join runs in the DataFusion aggregate backend
EXPLAIN (COSTS OFF)
SELECT sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x';

-- The sum is exact instead of wrapping
SELECT sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' AND t.grp = 1;
SELECT t.grp, sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' GROUP BY t.grp ORDER BY t.grp;
SELECT t.grp, sum(t.w) AS s FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' GROUP BY t.grp HAVING sum(t.w) > 10 ORDER BY s DESC;
SELECT sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' AND t.grp = 4;

-- Parity
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' AND t.grp = 1;
SELECT t.grp, sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' GROUP BY t.grp ORDER BY t.grp;
SELECT t.grp, sum(t.w) AS s FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' GROUP BY t.grp HAVING sum(t.w) > 10 ORDER BY s DESC;
SELECT sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' AND t.grp = 4;

-- Separate inserts create multiple index segments for MPP workers to scan.
SET paradedb.global_mutable_segment_rows TO 0;
INSERT INTO issue_6932_t
SELECT g, 'x', CASE WHEN g % 2 = 0 THEN 1 ELSE 2 END,
       CASE WHEN g % 2 = 0 THEN 9223372036854775807::int8 ELSE '-9223372036854775808'::int8 END
FROM generate_series(7, 134) g;
INSERT INTO issue_6932_t
SELECT g, 'x', CASE WHEN g % 2 = 0 THEN 1 ELSE 2 END,
       CASE WHEN g % 2 = 0 THEN 9223372036854775807::int8 ELSE '-9223372036854775808'::int8 END
FROM generate_series(135, 262) g;
INSERT INTO issue_6932_e SELECT id, 'x' FROM issue_6932_t WHERE id > 6;
RESET paradedb.global_mutable_segment_rows;
ANALYZE issue_6932_t;
ANALYZE issue_6932_e;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers TO 8;
SET max_parallel_workers_per_gather TO 3;

-- Require real workers and both aggregate stages, not a serial fallback.
DO $$
DECLARE
    line record;
    launched boolean := false;
    partial_sum boolean := false;
    final_sum boolean := false;
BEGIN
    FOR line IN EXECUTE $query$
        EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF)
        SELECT t.grp, sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
        WHERE t.body @@@ 'x' AND t.grp IN (1, 2) GROUP BY t.grp ORDER BY t.grp
    $query$
    LOOP
        launched := launched OR line."QUERY PLAN" ~ 'MPP Launch: workers=[1-9][0-9]*';
        partial_sum := partial_sum OR (line."QUERY PLAN" LIKE '%AggregateExec: mode=Partial%'
            AND line."QUERY PLAN" LIKE '%sum(%');
        final_sum := final_sum OR (line."QUERY PLAN" LIKE '%AggregateExec: mode=Final%'
            AND line."QUERY PLAN" LIKE '%sum(%');
    END LOOP;
    IF NOT (launched AND partial_sum AND final_sum) THEN
        RAISE EXCEPTION 'Expected MPP sum with workers, partial and final aggregation: %, %, %',
            launched, partial_sum, final_sum;
    END IF;
END $$;

-- Each group overflows int8 within workers and when combining partial sums.
SELECT t.grp, sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' AND t.grp IN (1, 2) GROUP BY t.grp ORDER BY t.grp;

-- The parallel result must match PostgreSQL's exact numeric sums.
SET paradedb.enable_aggregate_custom_scan TO off;
SET max_parallel_workers_per_gather TO 0;
SELECT t.grp, sum(t.w) FROM issue_6932_t t JOIN issue_6932_e e ON e.id = t.id
WHERE t.body @@@ 'x' AND t.grp IN (1, 2) GROUP BY t.grp ORDER BY t.grp;

RESET paradedb.mpp_min_rows;
RESET max_parallel_workers;

RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;

DROP TABLE issue_6932_t;
DROP TABLE issue_6932_e;
