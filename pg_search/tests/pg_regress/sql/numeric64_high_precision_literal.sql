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

DROP TABLE numeric64_high_precision;
