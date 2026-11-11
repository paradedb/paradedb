CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_6931_i (id int PRIMARY KEY, body text, a int4, w int8);
CREATE TABLE issue_6931_e (id int PRIMARY KEY, tag text, b int4);
INSERT INTO issue_6931_i VALUES (1, 'alpha', 2147483647, 9223372036854775807), (2, 'beta', 5, 10);
INSERT INTO issue_6931_e VALUES (1, 'x', 1), (2, 'y', 0);
CREATE INDEX issue_6931_i_idx ON issue_6931_i USING paradedb (id, body, a, w);
CREATE INDEX issue_6931_e_idx ON issue_6931_e USING paradedb (id, tag, b);

SET paradedb.enable_join_custom_scan = on;

-- Integer arithmetic in a Join Scan runs natively with overflow checks
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

-- Each width and mixed widths: operands widen to the operator's result type,
-- and overflow is checked at that width.
CREATE TABLE issue_6931_l (id int PRIMARY KEY, body text, s int2, a int4, w int8);
CREATE TABLE issue_6931_r (id int PRIMARY KEY, tag text, t int2, b int4, c int8);
INSERT INTO issue_6931_l VALUES
    (1, 'max', 32767, 2147483647, 9223372036854775807),
    (2, 'min', -32768, -2147483648, -9223372036854775808),
    (3, 'small', 3, 5, 10),
    (4, 'nulls', NULL, NULL, NULL),
    (5, 'minneg', -32768, -2147483648, -9223372036854775808);
INSERT INTO issue_6931_r VALUES
    (1, 'one', 1, 1, 1),
    (2, 'one', 1, 1, 1),
    (3, 'two', 2, 2, 2),
    (4, 'one', 1, 1, 1),
    (5, 'neg', -1, -1, -1);
CREATE INDEX issue_6931_l_idx ON issue_6931_l USING paradedb (id, body, s, a, w);
CREATE INDEX issue_6931_r_idx ON issue_6931_r USING paradedb (id, tag, t, b, c);

EXPLAIN (COSTS OFF)
SELECT l.id, l.s + r.t AS ss, l.s * r.b AS sa, l.a - r.c AS ac
FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small' ORDER BY l.id LIMIT 10;

-- Results match PostgreSQL, NULLs propagate, and a mixed-width result does not
-- overflow at the narrower operand's width (32767 + 1 is an integer).
SELECT l.id, l.s + r.t AS ss, l.s + r.b AS sb, l.s * r.b AS sb2, l.a - r.c AS ac,
       l.w * r.t AS wt, l.a * 2 AS a2
FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small OR nulls' ORDER BY l.id LIMIT 10;
SELECT l.id, l.s + r.b AS sb, l.a + r.c AS ac
FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' ORDER BY l.id LIMIT 10;

-- The overflow cases below run in the native pdb_int_* UDFs, in projections
-- and in join filters, not in the PostgreSQL fallback.
EXPLAIN (COSTS OFF)
SELECT l.id, l.s + r.t FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' LIMIT 10;
EXPLAIN (COSTS OFF)
SELECT l.id FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' AND l.w * (r.c + 1) > 0 LIMIT 10;

-- smallint
SELECT l.id, l.s + r.t FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' LIMIT 10;
SELECT l.id, l.s - r.t FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'min' LIMIT 10;
SELECT l.id, l.s * r.t FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'minneg' LIMIT 10;
-- integer, including integer + smallint and MIN * -1
SELECT l.id, l.a + r.t FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' LIMIT 10;
SELECT l.id, l.a - r.b FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'min' LIMIT 10;
SELECT l.id, l.a * r.b FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'minneg' LIMIT 10;
SELECT l.id, l.s * r.b * 65539 FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' LIMIT 10;
-- bigint, including integer * bigint and MIN * -1
SELECT l.id, l.w + r.b FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' LIMIT 10;
SELECT l.id, l.w - r.c FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'min' LIMIT 10;
SELECT l.id, r.b * l.w FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'minneg' LIMIT 10;
SELECT l.id FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' AND l.w * (r.c + 1) > 0 LIMIT 10;

-- The error carries SQLSTATE 22003 (numeric_value_out_of_range)
\set VERBOSITY sqlstate
SELECT l.id, l.a + r.b FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'max' LIMIT 10;
\set VERBOSITY default

-- Only columns, constants and integer +, - and * feed the native UDFs. Any other
-- operand (integer /, % or abs(), CASE, other functions) leaves the whole
-- expression to the PostgreSQL fallback, so PostgreSQL's errors are kept.
EXPLAIN (COSTS OFF)
SELECT i.id, (i.a / e.b) + 1 FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'beta' LIMIT 10;
SELECT i.id, (i.a / e.b) + 1 FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'beta' LIMIT 10;
SELECT i.id, (i.a % e.b) * 2 FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'beta' LIMIT 10;
SELECT l.id, (l.a / r.b) - 1 FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'minneg' LIMIT 10;
SELECT l.id, abs(l.a) + r.c FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'min' LIMIT 10;
SELECT l.id, (l.a / r.b) + 1 AS q, abs(l.s) * 2 AS m, (l.w % r.c) - 1 AS r
FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small' LIMIT 10;
SELECT i.id, i.a + CASE WHEN sqrt(e.b - 1) > 0 THEN 1 ELSE 0 END
FROM issue_6931_i i JOIN issue_6931_e e ON e.id = i.id
WHERE i.body @@@ 'beta' LIMIT 10;
EXPLAIN (COSTS OFF)
SELECT l.id, l.a + length(r.tag) FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small' LIMIT 10;
SELECT l.id, l.a + length(r.tag) FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small' LIMIT 10;

-- Non-integer operands with an integer result (date - date) stay on the fallback
CREATE TABLE issue_6931_d (id int PRIMARY KEY, body text, d date);
CREATE TABLE issue_6931_f (id int PRIMARY KEY, tag text, d date);
INSERT INTO issue_6931_d VALUES (1, 'alpha', '2024-03-01');
INSERT INTO issue_6931_f VALUES (1, 'x', '2024-01-01');
CREATE INDEX issue_6931_d_idx ON issue_6931_d USING paradedb (id, body, d);
CREATE INDEX issue_6931_f_idx ON issue_6931_f USING paradedb (id, tag, d);
EXPLAIN (COSTS OFF)
SELECT d.id, d.d - f.d FROM issue_6931_d d JOIN issue_6931_f f ON f.id = d.id
WHERE d.body @@@ 'alpha' LIMIT 10;
SELECT d.id, d.d - f.d FROM issue_6931_d d JOIN issue_6931_f f ON f.id = d.id
WHERE d.body @@@ 'alpha' LIMIT 10;

-- The operation comes from the operator's function, not its symbol
CREATE OPERATOR public.- (LEFTARG = int4, RIGHTARG = int4, FUNCTION = int4pl);
EXPLAIN (COSTS OFF)
SELECT l.id, l.a OPERATOR(public.-) r.b FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small' LIMIT 10;
SELECT l.id, l.a OPERATOR(public.-) r.b FROM issue_6931_l l JOIN issue_6931_r r ON r.id = l.id
WHERE l.body @@@ 'small' LIMIT 10;
DROP OPERATOR public.- (int4, int4);

RESET paradedb.enable_join_custom_scan;

DROP TABLE issue_6931_i;
DROP TABLE issue_6931_e;
DROP TABLE issue_6931_l;
DROP TABLE issue_6931_r;
DROP TABLE issue_6931_d;
DROP TABLE issue_6931_f;
