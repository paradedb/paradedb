-- STDDEV(DISTINCT ...) and STDDEV_POP(DISTINCT ...) over a join used to fail
-- in the DataFusion aggregate scan, because DataFusion's stddev has no
-- DISTINCT accumulator. They are now computed as sqrt(VAR(DISTINCT ...)).
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7314_a (id int PRIMARY KEY, k int, q int);
CREATE TABLE issue_7314_b (id int PRIMARY KEY, k int);

INSERT INTO issue_7314_a VALUES
    (1, 1, 1), (2, 1, 1), (3, 2, 2), (4, 2, 4), (5, 2, 4), (6, 3, 7), (7, 3, NULL);
INSERT INTO issue_7314_b VALUES (1, 1), (2, 2), (3, 3);

CREATE INDEX issue_7314_a_idx ON issue_7314_a USING paradedb (id, k, q);
CREATE INDEX issue_7314_b_idx ON issue_7314_b USING paradedb (id, k);

SET paradedb.enable_aggregate_custom_scan TO on;

-- The DataFusion aggregate scan is still used (this is a fix, not a fallback).
EXPLAIN (COSTS OFF)
SELECT STDDEV(DISTINCT a.q)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

-- The query from the issue, plus the other STDDEV spellings.
SELECT STDDEV(DISTINCT a.q), STDDEV_SAMP(DISTINCT a.q), STDDEV_POP(DISTINCT a.q)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

-- GROUP BY: group k=1 has a single distinct value (sample stddev is NULL),
-- group k=3 has one value and one NULL.
SELECT a.k, STDDEV(DISTINCT a.q), STDDEV_POP(DISTINCT a.q)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all()
GROUP BY a.k
ORDER BY a.k;

-- FILTER is applied before the square root.
SELECT STDDEV(DISTINCT a.q) FILTER (WHERE a.k > 1),
       STDDEV_POP(DISTINCT a.q) FILTER (WHERE a.k > 1)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

-- Mixed with non-DISTINCT aggregates in the same query.
SELECT STDDEV(DISTINCT a.q), STDDEV(a.q), VARIANCE(DISTINCT a.q), COUNT(*)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

-- Same queries with the aggregate scan off, for comparison.
SET paradedb.enable_aggregate_custom_scan TO off;

SELECT STDDEV(DISTINCT a.q), STDDEV_SAMP(DISTINCT a.q), STDDEV_POP(DISTINCT a.q)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

SELECT a.k, STDDEV(DISTINCT a.q), STDDEV_POP(DISTINCT a.q)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all()
GROUP BY a.k
ORDER BY a.k;

SELECT STDDEV(DISTINCT a.q) FILTER (WHERE a.k > 1),
       STDDEV_POP(DISTINCT a.q) FILTER (WHERE a.k > 1)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

SELECT STDDEV(DISTINCT a.q), STDDEV(a.q), VARIANCE(DISTINCT a.q), COUNT(*)
FROM issue_7314_a a JOIN issue_7314_b b ON a.k = b.k
WHERE a.id @@@ pdb.all();

RESET paradedb.enable_aggregate_custom_scan;

DROP TABLE issue_7314_a;
DROP TABLE issue_7314_b;
