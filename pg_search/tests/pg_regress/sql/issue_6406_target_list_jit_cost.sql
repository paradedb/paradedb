-- Tests for Issue #6406:
-- A search operator in the target list must not be charged paradedb.per_tuple_cost.
-- The penalty exists to steer quals toward the index; in the target list it cannot
-- change path choice, and it pushed trivial plans past every JIT threshold.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS issue_6406 CASCADE;
CREATE TABLE issue_6406 (
    id      INT PRIMARY KEY,
    title   TEXT NOT NULL,
    content TEXT NOT NULL
);

INSERT INTO issue_6406 (id, title, content)
SELECT g, 'title-' || g, 'alpha bravo charlie ' || g
FROM generate_series(1, 200) g;

CREATE INDEX issue_6406_idx ON issue_6406
USING paradedb (id, title, content) WITH (key_field = 'id');

ANALYZE issue_6406;

CREATE OR REPLACE FUNCTION issue_6406_total_cost(query TEXT) RETURNS FLOAT8
LANGUAGE plpgsql AS $$
DECLARE
    plan JSON;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || query INTO plan;
    RETURN (plan -> 0 -> 'Plan' ->> 'Total Cost')::FLOAT8;
END;
$$;

-- Qual only: the baseline from the issue.
SELECT issue_6406_total_cost($q$
    SELECT id FROM issue_6406 WHERE content @@@ 'alpha' LIMIT 5
$q$) < current_setting('jit_above_cost')::FLOAT8 AS below_jit_threshold;

-- Same query with a match operator in the target list.
SELECT issue_6406_total_cost($q$
    SELECT id, (title) ||| 'alpha' FROM issue_6406 WHERE content @@@ 'alpha' LIMIT 5
$q$) < current_setting('jit_above_cost')::FLOAT8 AS below_jit_threshold;

-- Nested inside a larger target list expression.
SELECT issue_6406_total_cost($q$
    SELECT id, CASE WHEN (title) ||| 'alpha' THEN 1 ELSE 0 END
    FROM issue_6406 WHERE content @@@ 'alpha' LIMIT 5
$q$) < current_setting('jit_above_cost')::FLOAT8 AS below_jit_threshold;

-- A qual forced onto the per-row filter path still carries the penalty.
SET paradedb.enable_custom_scan = OFF;
SET enable_indexscan = OFF;
SET enable_bitmapscan = OFF;

SELECT issue_6406_total_cost($q$
    SELECT id FROM issue_6406 WHERE content @@@ 'alpha'
$q$) >= current_setting('paradedb.per_tuple_cost')::FLOAT8 AS qual_penalized;

RESET paradedb.enable_custom_scan;
RESET enable_indexscan;
RESET enable_bitmapscan;

DROP FUNCTION issue_6406_total_cost(TEXT);
DROP TABLE issue_6406;
