\i common/common_setup.sql

-- A NUMERIC literal is compared with a Numeric64 column as PostgreSQL compares it, including a
-- literal far below one step of the column's scale (1e-41 against NUMERIC(10,2), where dividing
-- by 10^39 does not fit in an i128) and one with more significant digits than an i128 holds.
-- Such a literal is not on the column's grid: an equality term matches nothing, and a range
-- bound moves to the neighbouring grid point on the side the operator keeps.

CREATE TABLE numeric64_high_precision (
    id SERIAL PRIMARY KEY,
    price NUMERIC(10, 2)
);

INSERT INTO numeric64_high_precision (price) VALUES
    (-0.01), (0.00), (0.01), (1.23), (1.24);

CREATE INDEX numeric64_high_precision_idx ON numeric64_high_precision USING paradedb (
    id, price
);

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price = 1e-41
ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > 1e-41
ORDER BY id;

-- Far below one step of the scale: no row equals it, and 0.00 < 1e-41 < 0.01.
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price = 1e-41 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > 1e-41 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price >= 1e-41 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price < 1e-41 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price <= 1e-41 ORDER BY id;

-- The same on the negative side: -0.01 < -1e-41 < 0.00.
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price = -1e-41 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price >= -1e-41 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price < -1e-41 ORDER BY id;

-- More significant digits than an i128 holds: 1.23 < literal < 1.24.
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price = 1.2300000000000000000000000000000000000000000000001
ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > 1.2300000000000000000000000000000000000000000000001
ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price <= 1.2300000000000000000000000000000000000000000000001
ORDER BY id;

-- PostgreSQL's own answers, without the index, for comparison.
SELECT
    count(*) FILTER (WHERE price = 1e-41) AS eq,
    count(*) FILTER (WHERE price > 1e-41) AS gt,
    count(*) FILTER (WHERE price >= 1e-41) AS ge,
    count(*) FILTER (WHERE price < 1e-41) AS lt,
    count(*) FILTER (WHERE price <= 1e-41) AS le,
    count(*) FILTER (WHERE price = -1e-41) AS eq_neg,
    count(*) FILTER (WHERE price >= -1e-41) AS ge_neg,
    count(*) FILTER (WHERE price < -1e-41) AS lt_neg,
    count(*) FILTER (WHERE price = 1.2300000000000000000000000000000000000000000000001) AS eq_long,
    count(*) FILTER (WHERE price > 1.2300000000000000000000000000000000000000000000001) AS gt_long,
    count(*) FILTER (WHERE price <= 1.2300000000000000000000000000000000000000000000001) AS le_long
FROM numeric64_high_precision;

-- The ceiling of this scale-2 literal exceeds i64::MAX. It must not wrap and
-- match ordinary rows. NaN sorts above every finite literal in PostgreSQL.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > 92233720368547758.075
ORDER BY id;

SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > 92233720368547758.075 ORDER BY id;

INSERT INTO numeric64_high_precision (price) VALUES ('NaN'), (NULL);

SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > 92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price >= 92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price < 92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price <= 92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price = 92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price <> 92233720368547758.075 ORDER BY id;

-- An exact grid point at i64::MAX must not be mistaken for the NaN sentinel.
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price = 92233720368547758.07 ORDER BY id;

-- Check both sides near i64::MIN as well.
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price > -92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price >= -92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price < -92233720368547758.075 ORDER BY id;
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price <= -92233720368547758.075 ORDER BY id;

-- PostgreSQL's own counts without pushdown, including NaN and excluding NULL.
SELECT
    count(*) FILTER (WHERE price > 92233720368547758.075) AS gt,
    count(*) FILTER (WHERE price >= 92233720368547758.075) AS ge,
    count(*) FILTER (WHERE price < 92233720368547758.075) AS lt,
    count(*) FILTER (WHERE price <= 92233720368547758.075) AS le,
    count(*) FILTER (WHERE price = 92233720368547758.075) AS eq,
    count(*) FILTER (WHERE price <> 92233720368547758.075) AS ne,
    count(*) FILTER (WHERE price = 92233720368547758.07) AS eq_max,
    count(*) FILTER (WHERE price > -92233720368547758.075) AS gt_neg,
    count(*) FILTER (WHERE price >= -92233720368547758.075) AS ge_neg,
    count(*) FILTER (WHERE price < -92233720368547758.075) AS lt_neg,
    count(*) FILTER (WHERE price <= -92233720368547758.075) AS le_neg
FROM numeric64_high_precision;

-- A uniform segment must not be pruned for the negation of an off-grid term.
DROP INDEX numeric64_high_precision_idx;
TRUNCATE numeric64_high_precision;
INSERT INTO numeric64_high_precision (price) VALUES (0.00);
CREATE INDEX numeric64_high_precision_idx ON numeric64_high_precision USING paradedb (
    id, price
) WITH (partition_by = 'price');
SELECT id, price FROM numeric64_high_precision
WHERE id @@@ paradedb.all() AND price <> 1e-41 ORDER BY id;

DROP TABLE numeric64_high_precision;
