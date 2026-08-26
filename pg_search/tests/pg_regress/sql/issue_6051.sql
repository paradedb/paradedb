\i common/common_setup.sql

-- NUMERIC with precision > 18 is stored as sortable bytes. Negative values whose digit
-- strings share a prefix (e.g. -49990 and -49999) are the case where the byte order used
-- to disagree with the numeric order.

SET enable_seqscan = off;
SET max_parallel_workers_per_gather = 0;

CREATE TABLE amounts (
    id serial PRIMARY KEY,
    direction varchar NOT NULL,
    amt_18 numeric(18,0) NOT NULL,
    amt_78 numeric(78,0) NOT NULL,
    amt_any numeric NOT NULL,
    amt_scaled numeric(30,10) NOT NULL,
    discarded_at timestamp
);

INSERT INTO amounts (direction, amt_18, amt_78, amt_any, amt_scaled)
SELECT 'debit', -g, -g, -g, -g / 1000000.0
FROM generate_series(1, 50000) g;

INSERT INTO amounts (direction, amt_18, amt_78, amt_any, amt_scaled)
VALUES
    ('credit', 5, 5, 5, 0.5),
    ('credit', 51, 51, 51, 0.51),
    ('debit', -5, -5, -5, -0.5),
    ('debit', -51, -51, -51, -0.51),
    ('debit', -50001, -50001, -50001, -0.50001),
    ('debit', -4999, -4999, -4999, -0.4999),
    ('debit', -100001, -100001, -100001, -100.001),
    ('debit', -999999999999999999, -100000000000000000000, -100000000000000000000, -1000000000000000000.0000000001),
    ('zero', 0, 0, 0, 0);

CREATE INDEX amounts_idx ON amounts USING bm25 (
    id, (direction::pdb.literal), amt_18, amt_78, amt_any, amt_scaled, discarded_at
) WITH (key_field = 'id') WHERE (discarded_at IS NULL);

-- TopN ascending: the numeric(18,0) column is the reference ordering.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_78 ASC LIMIT 8;

SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_78 ASC LIMIT 8;

SELECT amt_18 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_18 ASC LIMIT 8;

-- TopN descending over negatives only.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_78 DESC LIMIT 8;

SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_78 DESC LIMIT 8;

-- Unlimited-precision NUMERIC takes the same storage path.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amt_any FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_any ASC LIMIT 8;

SELECT amt_any FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_any ASC LIMIT 8;

-- Fractional negatives with a 20-digit integer part in the mix.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amt_scaled FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_scaled ASC LIMIT 4;

SELECT amt_scaled FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_scaled ASC LIMIT 4;

-- Shared-prefix fractions: -0.51 < -0.50001 < -0.5 < -0.4999.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT amt_scaled FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.range('amt_scaled', numrange(-1, -0.4))
ORDER BY amt_scaled ASC;

SELECT amt_scaled FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.range('amt_scaled', numrange(-1, -0.4))
ORDER BY amt_scaled ASC;

-- Whole-table ordering across the sign boundary.
SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all()
ORDER BY amt_78 ASC LIMIT 4;

SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all()
ORDER BY amt_78 DESC LIMIT 4;

-- Range pushdown must not return rows outside the requested range.
SELECT count(*) FROM (
    SELECT amt_78 FROM amounts
    WHERE discarded_at IS NULL AND id @@@ paradedb.range('amt_78', numrange(-20500, -20300))
) s
WHERE NOT (amt_78 >= -20500 AND amt_78 < -20300);

SELECT count(*) FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.range('amt_78', numrange(-20500, -20300));

SELECT count(*) FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.range('amt_18', numrange(-20500, -20300));

-- Range with a shared-prefix boundary: (-50000, -49990] must exclude -50000 and include -49990.
SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.range('amt_78', numrange(-50000, -49990, '(]'))
ORDER BY amt_78;

-- Heap-filter pushdown on the fast field.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all() AND amt_78 > -20500 AND amt_78 < -20300;

SELECT count(*) FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all() AND amt_78 > -20500 AND amt_78 < -20300;

SELECT count(*) FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all() AND amt_18 > -20500 AND amt_18 < -20300;

-- Equality on values with shared digit prefixes.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('amt_78', -49990::numeric)
ORDER BY id;

SELECT id, amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('amt_78', -49990::numeric)
ORDER BY id;

SELECT id, amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('amt_78', -100001::numeric)
ORDER BY id;

SELECT id, amt_scaled FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('amt_scaled', -0.5::numeric)
ORDER BY id;

-- IN (...) is pushed down as a term set and has to use the same term conversion.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all() AND amt_78 IN (-49999, -49990, 5)
ORDER BY id;

SELECT id, amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all() AND amt_78 IN (-49999, -49990, 5)
ORDER BY id;

SELECT id, amt_scaled FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.all() AND amt_scaled IN (-0.5, -0.50001)
ORDER BY id;

-- A term that can't be parsed as a number is an error, not an empty match.
SELECT id FROM amounts
WHERE discarded_at IS NULL AND amt_78 @@@ pdb.term_set(ARRAY['not-a-number']::text[]);

-- Rows written after the index build take the same byte layout as the build.
INSERT INTO amounts (direction, amt_18, amt_78, amt_any, amt_scaled)
VALUES ('debit', -49995, -49995, -49995, -0.49995);

SELECT amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('direction', 'debit')
ORDER BY amt_78 ASC LIMIT 8;

SELECT id, amt_78 FROM amounts
WHERE discarded_at IS NULL AND id @@@ paradedb.term('amt_78', -49995::numeric)
ORDER BY id;

DROP TABLE amounts;

-- NUMERIC with precision <= 18 is stored as a scaled integer, and IN (...) has to scale too.
CREATE TABLE prices (
    id serial PRIMARY KEY,
    price numeric(10,2)
);

INSERT INTO prices (price) VALUES (1.23), (4.56), (1.00), (4.00), (-1.23);

CREATE INDEX prices_idx ON prices USING bm25 (id, price) WITH (key_field = 'id');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, price FROM prices
WHERE id @@@ paradedb.all() AND price IN (1.23, 4.56, -1.23)
ORDER BY id;

SELECT id, price FROM prices
WHERE id @@@ paradedb.all() AND price IN (1.23, 4.56, -1.23)
ORDER BY id;

DROP TABLE prices;

-- numrange bounds are stored with the same encoding.
CREATE TABLE spans (
    id serial PRIMARY KEY,
    span numrange
);

INSERT INTO spans (span) VALUES
    ('[-50000, -49990)'),
    ('[-49999, -49991]'),
    ('(-49990, -49000]'),
    ('[-0.51, -0.5)'),
    ('[-0.5, 0.5]');

CREATE INDEX spans_idx ON spans USING bm25 (id, span) WITH (key_field = 'id');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, span FROM spans
WHERE id @@@ paradedb.range_term('span', -49995::numeric)
ORDER BY id;

SELECT id, span FROM spans
WHERE id @@@ paradedb.range_term('span', -49995::numeric)
ORDER BY id;

SELECT id, span FROM spans
WHERE span @> -49995::numeric
ORDER BY id;

SELECT id, span FROM spans
WHERE id @@@ paradedb.range_term('span', -49990::numeric)
ORDER BY id;

SELECT id, span FROM spans
WHERE span @> -49990::numeric
ORDER BY id;

SELECT id, span FROM spans
WHERE id @@@ paradedb.range_term('span', -0.5::numeric)
ORDER BY id;

SELECT id, span FROM spans
WHERE span @> -0.5::numeric
ORDER BY id;

SELECT id, span FROM spans
WHERE id @@@ paradedb.range_term('span', numrange(-49992, -49991), 'Intersects')
ORDER BY id;

SELECT id, span FROM spans
WHERE span && numrange(-49992, -49991)
ORDER BY id;

DROP TABLE spans;
