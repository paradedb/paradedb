-- Regression test for issue #7236.
-- The Aggregate Scan placed the empty string group before NULL under
-- ASC NULLS FIRST and after NULL under DESC NULLS LAST. Each query runs with the
-- custom scan on and off, and both must return the same order.

DROP TABLE IF EXISTS issue_7236;

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7236 (id int, s text, t text, n int);
CREATE INDEX issue_7236_idx ON issue_7236 USING paradedb (
    id,
    (s::pdb.literal),
    (t::pdb.literal),
    n
) WITH (key_field = 'id');

INSERT INTO issue_7236 VALUES
    (1, '', 'x', 1),
    (2, '', '', 2),
    (3, '', NULL, 3),
    (4, NULL, 'x', 4),
    (5, NULL, '', 5),
    (6, NULL, NULL, 6),
    (7, 'a', 'x', 7),
    (8, 'a', '', 8),
    (9, 'a', NULL, 9),
    (10, NULL, NULL, 10);

\pset null '<null>'

EXPLAIN (COSTS OFF)
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST;

-- Runs each query with the custom scan on, then off.
SET paradedb.enable_aggregate_custom_scan = on;

SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS LAST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS LAST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS FIRST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST LIMIT 1;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS LAST LIMIT 2;
SELECT s, t, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s, t ORDER BY s ASC NULLS FIRST, t DESC NULLS LAST;
SELECT s, t, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s, t ORDER BY s DESC NULLS LAST, t ASC NULLS FIRST;
SELECT s, COUNT(*) FILTER (WHERE n <= 5), COUNT(*) FILTER (WHERE n > 5)
FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST;
SELECT s, COUNT(*) FILTER (WHERE n <= 5), COUNT(*) FILTER (WHERE n > 5)
FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS LAST;

SET paradedb.enable_aggregate_custom_scan = off;

SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS LAST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS LAST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS FIRST;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST LIMIT 1;
SELECT s, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS LAST LIMIT 2;
SELECT s, t, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s, t ORDER BY s ASC NULLS FIRST, t DESC NULLS LAST;
SELECT s, t, COUNT(*) FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s, t ORDER BY s DESC NULLS LAST, t ASC NULLS FIRST;
SELECT s, COUNT(*) FILTER (WHERE n <= 5), COUNT(*) FILTER (WHERE n > 5)
FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s ASC NULLS FIRST;
SELECT s, COUNT(*) FILTER (WHERE n <= 5), COUNT(*) FILTER (WHERE n > 5)
FROM issue_7236 WHERE issue_7236 @@@ pdb.all()
GROUP BY s ORDER BY s DESC NULLS LAST;

RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE issue_7236;
