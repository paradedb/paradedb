\i common/common_setup.sql

-- PG18 RTE_GROUP regression (issue #3827)
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS issue_3827_t CASCADE;
CREATE TABLE issue_3827_t (
    id SERIAL PRIMARY KEY,
    txt TEXT,
    n INT,
    not_indexed INT  -- column NOT in the bm25 index
);

INSERT INTO issue_3827_t (txt, n, not_indexed) VALUES
    ('foo', 1, 10),
    ('foo', 2, 20),
    ('foo', 3, 30);

CREATE INDEX issue_3827_t_idx ON issue_3827_t
USING bm25 (id, txt, n)
WITH (
    key_field = 'id',
    text_fields = '{"txt": {}}',
    numeric_fields = '{"n": {"fast": true}}'
);

-- Test 1: Window agg pushdown with GROUP BY/ORDER BY on grouping column (planner hook)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT n, SUM(n) OVER ()
FROM issue_3827_t
WHERE id @@@ pdb.all()
GROUP BY n
ORDER BY n
LIMIT 1;

-- Test 2: ORDER BY on grouped column is pushed into aggregate definition (planner-time)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT n, COUNT(*)
FROM issue_3827_t
WHERE id @@@ pdb.all()
GROUP BY n
ORDER BY n
LIMIT 1;

-- Test 3: HAVING on grouped column uses support function before flattening
-- (aggregate term prevents HAVING from being pushed into WHERE)
SELECT txt
FROM issue_3827_t
GROUP BY txt
HAVING (txt @@@ pdb.parse('foo')) OR SUM(n) < 0
ORDER BY txt;

-- Test 4: GROUP BY on non-indexed column should gracefully fall back
-- (RTE_GROUP resolution returns the var, but field lookup returns None)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT not_indexed, COUNT(*)
FROM issue_3827_t
WHERE id @@@ pdb.all()
GROUP BY not_indexed
ORDER BY not_indexed;

SELECT not_indexed, COUNT(*)
FROM issue_3827_t
WHERE id @@@ pdb.all()
GROUP BY not_indexed
ORDER BY not_indexed;

-- Test 5: Multi-column GROUP BY on indexed columns
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT txt, n, COUNT(*)
FROM issue_3827_t
WHERE id @@@ pdb.all()
GROUP BY txt, n
ORDER BY txt, n;

SELECT txt, n, COUNT(*)
FROM issue_3827_t
WHERE id @@@ pdb.all()
GROUP BY txt, n
ORDER BY txt, n;

DROP TABLE issue_3827_t;

\i common/common_cleanup.sql
