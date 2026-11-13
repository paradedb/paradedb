-- A Join Scan or an Aggregate Scan that is read in parts, by a cursor or a PL/pgSQL
-- FOR loop, must go on from the row where the previous fetch stopped. Postgres shuts
-- the plan down at the end of each partial fetch and then runs it again (#7443).

\i common/common_setup.sql

CREATE TABLE issue_7443_s (id int PRIMARY KEY, title text);
CREATE TABLE issue_7443_e (id int PRIMARY KEY, s_id int);
INSERT INTO issue_7443_s SELECT g, 'dragon ' || g FROM generate_series(1, 9000) g;
INSERT INTO issue_7443_e SELECT g, g FROM generate_series(1, 9000) g;
CREATE INDEX issue_7443_s_idx ON issue_7443_s USING paradedb (id, title);
CREATE INDEX issue_7443_e_idx ON issue_7443_e USING paradedb (id, s_id);

SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

EXPLAIN (COSTS OFF)
SELECT s.id, e.id AS e_id
FROM issue_7443_s s JOIN issue_7443_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon'
ORDER BY s.id LIMIT 6;

-- Join Scan: three fetches of 4 rows return rows 1-4, then 5-6, then none.
BEGIN;
DECLARE issue_7443_join CURSOR FOR
SELECT s.id, e.id AS e_id
FROM issue_7443_s s JOIN issue_7443_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon'
ORDER BY s.id LIMIT 6;
FETCH 4 FROM issue_7443_join;
FETCH 4 FROM issue_7443_join;
FETCH 4 FROM issue_7443_join;
COMMIT;

-- Join Scan: a FOR loop in a procedure called at the top level fetches one row at
-- a time. It must see each of the 12 rows once and then end.
CREATE PROCEDURE issue_7443_join_loop(INOUT ids int[] DEFAULT '{}')
LANGUAGE plpgsql AS $$
DECLARE
    r record;
BEGIN
    FOR r IN
        SELECT s.id
        FROM issue_7443_s s JOIN issue_7443_e e ON e.s_id = s.id
        WHERE s.title ||| 'dragon'
        ORDER BY s.id LIMIT 12
    LOOP
        ids := ids || r.id;
        EXIT WHEN cardinality(ids) > 30;
    END LOOP;
END
$$;
CALL issue_7443_join_loop();

EXPLAIN (COSTS OFF)
SELECT s.id, count(*)
FROM issue_7443_s s JOIN issue_7443_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon'
GROUP BY s.id;

-- Aggregate Scan: a FOR loop in a function fetches 10 rows, then 50 at a time. The
-- 9000 groups are more than one DataFusion batch (8192 rows), and the loop must
-- see all of them.
CREATE FUNCTION issue_7443_agg_loop() RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE
    r record;
    n bigint := 0;
BEGIN
    FOR r IN
        SELECT s.id, count(*)
        FROM issue_7443_s s JOIN issue_7443_e e ON e.s_id = s.id
        WHERE s.title ||| 'dragon'
        GROUP BY s.id
    LOOP
        n := n + 1;
    END LOOP;
    RETURN n;
END
$$;
SELECT issue_7443_agg_loop();

RESET paradedb.enable_join_custom_scan;
RESET paradedb.enable_aggregate_custom_scan;

DROP PROCEDURE issue_7443_join_loop;
DROP FUNCTION issue_7443_agg_loop;
DROP TABLE issue_7443_s;
DROP TABLE issue_7443_e;
