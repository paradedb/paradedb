\i common/common_setup.sql

-- ============================================================================
-- NUMERIC TYPMOD SCALE LIMITS
-- ============================================================================
-- A Numeric64 field is backed by decimal_bytes::Decimal64NoScale, which encodes
-- scales in -18..=18. PostgreSQL accepts a far wider scale range, so the field
-- type routing has to bound the scale as well as the precision. Checking only
-- the precision let types such as numeric(3,20) select Numeric64 and then fail
-- when a value was actually indexed.
--
-- Both lifecycles are covered, because they materialize the value at different
-- points:
--   1. populated table -> CREATE INDEX
--   2. empty table -> CREATE INDEX -> INSERT -> query
--
-- See https://github.com/paradedb/paradedb/issues/6101
-- ============================================================================

-- ----------------------------------------------------------------------------
-- On the boundary: scale 18 stays on Numeric64
-- ----------------------------------------------------------------------------
CREATE TABLE numeric_scale_18 (id int, value numeric(3,18));
INSERT INTO numeric_scale_18 VALUES (1, 0.000000000000000123);
CREATE INDEX numeric_scale_18_idx ON numeric_scale_18 USING bm25 (id, value);

SELECT id, value FROM numeric_scale_18 WHERE id @@@ pdb.all() ORDER BY id;

-- ----------------------------------------------------------------------------
-- One step past it: scale 19 must fall back to NumericBytes, not fail
-- ----------------------------------------------------------------------------
CREATE TABLE numeric_scale_19 (id int, value numeric(3,19));
INSERT INTO numeric_scale_19 VALUES (1, 0.0000000000000000123);
CREATE INDEX numeric_scale_19_idx ON numeric_scale_19 USING bm25 (id, value);

SELECT id, value FROM numeric_scale_19 WHERE id @@@ pdb.all() ORDER BY id;

-- ----------------------------------------------------------------------------
-- Symmetric for negative scales
-- ----------------------------------------------------------------------------
CREATE TABLE numeric_scale_neg19 (id int, value numeric(3,-19));
INSERT INTO numeric_scale_neg19 VALUES (1, 1230000000000000000000);
CREATE INDEX numeric_scale_neg19_idx ON numeric_scale_neg19 USING bm25 (id, value);

SELECT id, value FROM numeric_scale_neg19 WHERE id @@@ pdb.all() ORDER BY id;

-- ----------------------------------------------------------------------------
-- Empty-index lifecycle: the value is materialized by the first search, so this
-- path failed even when CREATE INDEX had succeeded
-- ----------------------------------------------------------------------------
CREATE TABLE numeric_scale_delayed (id int, value numeric(3,20));
CREATE INDEX numeric_scale_delayed_idx ON numeric_scale_delayed USING bm25 (id, value);
INSERT INTO numeric_scale_delayed VALUES (1, 0.00000000000000000123);

SELECT id, value FROM numeric_scale_delayed WHERE id @@@ pdb.all() ORDER BY id;

-- ----------------------------------------------------------------------------
-- The precision bound is unchanged
-- ----------------------------------------------------------------------------
CREATE TABLE numeric_precision_19 (id int, value numeric(19,2));
INSERT INTO numeric_precision_19 VALUES (1, 1234567890123456.78);
CREATE INDEX numeric_precision_19_idx ON numeric_precision_19 USING bm25 (id, value);

SELECT id, value FROM numeric_precision_19 WHERE id @@@ pdb.all() ORDER BY id;

DROP TABLE numeric_scale_18, numeric_scale_19, numeric_scale_neg19,
           numeric_scale_delayed, numeric_precision_19;
