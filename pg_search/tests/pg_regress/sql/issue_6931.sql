CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6931_i (id int PRIMARY KEY, body text, a int4, w int8);
CREATE TABLE issue_6931_e (id int PRIMARY KEY, tag text, b int4);
INSERT INTO issue_6931_i VALUES (1, 'alpha', 2147483647, 9223372036854775807), (2, 'beta', 5, 10);
INSERT INTO issue_6931_e VALUES (1, 'x', 1), (2, 'y', 0);
CREATE INDEX issue_6931_i_idx ON issue_6931_i USING paradedb (id, body, a, w);
CREATE INDEX issue_6931_e_idx ON issue_6931_e USING paradedb (id, tag, b);

SET paradedb.enable_join_custom_scan = on;

-- Integer arithmetic in a Join Scan is evaluated by PostgreSQL
EXPLAIN (COSTS OFF)
SELECT i.id FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' AND (i.w + e.b) > 0 LIMIT 10;

-- Overflow raises an error instead of wrapping
SELECT i.id FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' AND (i.w + e.b) > 0 LIMIT 10;
SELECT i.id, (i.w + e.b) AS s FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' LIMIT 10;
SELECT i.id, (i.a + e.b) AS s FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' LIMIT 10;
SELECT i.id, (-i.a - 2 * e.b) AS s FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' LIMIT 10;
SELECT i.id, (i.w * (e.b + 1)) AS s FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'alpha' LIMIT 10;

-- Arithmetic that does not overflow is unchanged
SELECT i.id, (i.w + e.b) AS s, (i.a - e.b) AS d, (i.a * 2) AS m
FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'beta' AND (i.w + e.b) > 0 LIMIT 10;

RESET paradedb.enable_join_custom_scan;

DROP TABLE issue_6931_i;
DROP TABLE issue_6931_e;
