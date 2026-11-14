-- A bare whole-row reference in a Join Scan target list must return the
-- source's row from its fetched heap tuple, not NULL (#6922).
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_join_custom_scan = on;

CREATE TABLE issue_6922_s (id int PRIMARY KEY, title text, path text);
CREATE TABLE issue_6922_e (id int PRIMARY KEY, s_id int, dropped text);
INSERT INTO issue_6922_s VALUES
    (1, 'dragon one', 'p/1'), (2, 'other two', 'p/2'),
    (3, 'dragon three', NULL), (4, 'dragon four', 'p/4');
INSERT INTO issue_6922_e VALUES (10, 1, 'gone'), (11, 3, 'gone'), (12, 2, 'gone');
-- The rows above keep the dropped column and lack the added one.
ALTER TABLE issue_6922_e DROP COLUMN dropped;
ALTER TABLE issue_6922_e ADD COLUMN note text DEFAULT 'added';
CREATE INDEX issue_6922_s_idx ON issue_6922_s USING paradedb (id, title);
CREATE INDEX issue_6922_e_idx ON issue_6922_e USING paradedb (id, s_id);
ANALYZE issue_6922_s;
ANALYZE issue_6922_e;

-- The searched side.
EXPLAIN (COSTS OFF)
SELECT s.id, s FROM issue_6922_s s JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;
SELECT s.id, s FROM issue_6922_s s JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;

-- The other side, referenced only by its whole row.
SELECT s.id, e FROM issue_6922_s s JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;

-- s.id 4 has no match, so its e must be NULL.
EXPLAIN (COSTS OFF)
SELECT s, e FROM issue_6922_s s LEFT JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;
SELECT s, e FROM issue_6922_s s LEFT JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;

-- Postgres returns the same rows.
SET paradedb.enable_join_custom_scan = off;
SELECT s.id, s FROM issue_6922_s s JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;
SELECT s.id, e FROM issue_6922_s s JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;
SELECT s, e FROM issue_6922_s s LEFT JOIN issue_6922_e e ON e.s_id = s.id
WHERE s.title ||| 'dragon' ORDER BY s.id LIMIT 10;

RESET paradedb.enable_join_custom_scan;
DROP TABLE issue_6922_s, issue_6922_e;
