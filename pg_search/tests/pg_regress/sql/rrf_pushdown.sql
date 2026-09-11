-- The natural RRF hybrid-search shape puts a ranking window function in the same query
-- level as `ORDER BY ... LIMIT`. PG zeroes `limit_tuples` whenever a WindowAgg sits
-- between the LIMIT and the scan, so Top K was never considered and the whole result set
-- was read and sorted.
--
-- When the window is a bare ranking over the same order as the LIMIT, the top N rows are
-- the first N in window order, so feeding the WindowAgg only those N is result-preserving
-- and the scan can be a Top K. See #5742.
--
-- Every "pushes down" case below also asserts the rows and rank values are identical to
-- computing the window over the full result set.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE wlp (
    id    int PRIMARY KEY,
    label text,
    n     int,
    grp   int
);

INSERT INTO wlp
SELECT g, 'shoes item ' || g, g, g % 4
FROM generate_series(1, 500) g;

CREATE INDEX wlp_idx ON wlp USING paradedb (id, label, n, grp) WITH (key_field = 'id');

-- ============================================================
-- Pushes down: bare ranking window over the LIMIT's own ordering
-- ============================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, ROW_NUMBER() OVER (ORDER BY n DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY n DESC LIMIT 10;

-- row_number, rank, dense_rank all match the full-corpus computation.
-- `mismatches` must be 0 for each.
SELECT count(*) AS mismatches FROM (
    (SELECT id, ROW_NUMBER() OVER (ORDER BY n DESC) AS rank
     FROM wlp WHERE label @@@ 'shoes' ORDER BY n DESC LIMIT 10)
    EXCEPT
    (SELECT id, rank FROM
        (SELECT id, ROW_NUMBER() OVER (ORDER BY n DESC) AS rank
         FROM wlp WHERE label @@@ 'shoes') x
     ORDER BY rank LIMIT 10)
) d;

SELECT count(*) AS mismatches FROM (
    (SELECT id, RANK() OVER (ORDER BY grp DESC, id) AS rank
     FROM wlp WHERE label @@@ 'shoes' ORDER BY grp DESC, id LIMIT 25)
    EXCEPT
    (SELECT id, rank FROM
        (SELECT id, RANK() OVER (ORDER BY grp DESC, id) AS rank
         FROM wlp WHERE label @@@ 'shoes') x
     ORDER BY rank LIMIT 25)
) d;

SELECT count(*) AS mismatches FROM (
    (SELECT id, DENSE_RANK() OVER (ORDER BY grp DESC, id) AS rank
     FROM wlp WHERE label @@@ 'shoes' ORDER BY grp DESC, id LIMIT 25)
    EXCEPT
    (SELECT id, rank FROM
        (SELECT id, DENSE_RANK() OVER (ORDER BY grp DESC, id) AS rank
         FROM wlp WHERE label @@@ 'shoes') x
     ORDER BY rank LIMIT 25)
) d;

-- score ordering, which is what the RRF text branch uses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, ROW_NUMBER() OVER (ORDER BY paradedb.score(id) DESC, id) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY paradedb.score(id) DESC, id LIMIT 10;

-- ============================================================
-- OFFSET: the scan must fetch LIMIT + OFFSET rows, else the ranks
-- past the offset are computed over too few rows
-- ============================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, ROW_NUMBER() OVER (ORDER BY n DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY n DESC LIMIT 20 OFFSET 30;

SELECT count(*) AS mismatches FROM (
    (SELECT id, ROW_NUMBER() OVER (ORDER BY n DESC) AS rank
     FROM wlp WHERE label @@@ 'shoes' ORDER BY n DESC LIMIT 20 OFFSET 30)
    EXCEPT
    (SELECT id, rank FROM
        (SELECT id, ROW_NUMBER() OVER (ORDER BY n DESC) AS rank
         FROM wlp WHERE label @@@ 'shoes') x
     ORDER BY rank LIMIT 20 OFFSET 30)
) d;

-- ============================================================
-- Must NOT push down: truncating the input would change the values
-- ============================================================

-- PARTITION BY draws rows from outside the top N
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, ROW_NUMBER() OVER (PARTITION BY grp ORDER BY n DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY n DESC LIMIT 10;

-- window ordering differs from the query's ORDER BY
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, ROW_NUMBER() OVER (ORDER BY grp DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY n DESC LIMIT 10;

-- percent_rank needs the total row count
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, PERCENT_RANK() OVER (ORDER BY n DESC) AS pr
FROM wlp WHERE label @@@ 'shoes'
ORDER BY n DESC LIMIT 10;

-- and their values stay correct
SELECT count(*) AS mismatches FROM (
    (SELECT id, ROW_NUMBER() OVER (PARTITION BY grp ORDER BY n DESC) AS rank
     FROM wlp WHERE label @@@ 'shoes' ORDER BY n DESC LIMIT 10)
    EXCEPT
    (SELECT id, rank FROM
        (SELECT id, n, ROW_NUMBER() OVER (PARTITION BY grp ORDER BY n DESC) AS rank
         FROM wlp WHERE label @@@ 'shoes') x
     ORDER BY n DESC LIMIT 10)
) d;

-- `limit_tuples == -1` also stands in for DISTINCT / GROUP BY / HAVING / aggregates, all of
-- which collapse rows above the WindowAgg. Overriding it on the window check alone would feed
-- the collapse only LIMIT rows and lose whole output rows.
-- DISTINCT: 4 distinct grp values, so LIMIT 3 must return 3 rows
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT DISTINCT grp, DENSE_RANK() OVER (ORDER BY grp DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY grp DESC LIMIT 3;

SELECT DISTINCT grp, DENSE_RANK() OVER (ORDER BY grp DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
ORDER BY grp DESC LIMIT 3;

-- GROUP BY on an expression the Aggregate Scan can't resolve, so it falls through to Base Scan
SET paradedb.planner_warnings = 'off';

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT grp + 0 AS g, RANK() OVER (ORDER BY grp + 0 DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
GROUP BY grp + 0 ORDER BY grp + 0 DESC LIMIT 3;

SELECT grp + 0 AS g, RANK() OVER (ORDER BY grp + 0 DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
GROUP BY grp + 0 ORDER BY grp + 0 DESC LIMIT 3;

SELECT grp + 0 AS g, RANK() OVER (ORDER BY grp + 0 DESC) AS rank
FROM wlp WHERE label @@@ 'shoes'
GROUP BY grp + 0 HAVING count(*) > 0 ORDER BY grp + 0 DESC LIMIT 3;

RESET paradedb.planner_warnings;

-- ============================================================
-- The RRF shape itself: both branches must be Top K
-- ============================================================

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
WITH a AS (
    SELECT id, RANK() OVER (ORDER BY paradedb.score(id) DESC, id) AS rank
    FROM wlp WHERE label @@@ 'shoes'
    ORDER BY paradedb.score(id) DESC, id LIMIT 20
),
b AS (
    SELECT id, RANK() OVER (ORDER BY n DESC) AS rank
    FROM wlp WHERE label @@@ 'item'
    ORDER BY n DESC LIMIT 20
)
SELECT COALESCE(a.id, b.id) AS id,
       COALESCE(1.0/(60 + a.rank), 0.0) + COALESCE(1.0/(60 + b.rank), 0.0) AS rrf
FROM a FULL OUTER JOIN b ON a.id = b.id
ORDER BY rrf DESC, id LIMIT 5;

DROP TABLE wlp;
