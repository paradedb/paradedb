\i common/common_setup.sql

CALL paradedb.create_paradedb_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

ALTER TABLE mock_items
    ADD COLUMN created_at_tz TIMESTAMPTZ,
    ADD COLUMN latest_available_time_tz TIMETZ,
    ADD COLUMN arbitrary_precision NUMERIC,
    ADD COLUMN fixed_precision NUMERIC(10, 2);
UPDATE mock_items SET
    created_at_tz = created_at AT TIME ZONE 'UTC',
    latest_available_time_tz = (latest_available_time || '+00')::timetz,
    arbitrary_precision = CASE id
        WHEN 1 THEN 1234
        WHEN 2 THEN 5678
        WHEN 3 THEN -100
        WHEN 4 THEN 100000000000000000000.123
    END,
    fixed_precision = CASE id
        WHEN 1 THEN 1.00
        WHEN 2 THEN 0.01
        WHEN 3 THEN 1.23
        WHEN 4 THEN 2.50
    END;

CREATE INDEX on mock_items
USING paradedb (id, description, rating, category, metadata, created_at, last_updated_date, latest_available_time, created_at_tz, latest_available_time_tz, arbitrary_precision, fixed_precision);

SELECT id, description, category FROM mock_items
WHERE id @@@ pdb.parse('description:(running shoes) AND category:footwear');

SELECT id, description, category FROM mock_items
WHERE id @@@ pdb.parse('description:(running shoes) AND category:footwear', conjunction_mode => true);

SELECT id, description, category FROM mock_items
WHERE description @@@ pdb.parse_with_field('(running shoes)', lenient => true);

SELECT id, description, created_at FROM mock_items
WHERE id @@@ pdb.parse('created_at:"2023-05-01 09:12:34"') ORDER BY id;

SELECT id, description, last_updated_date FROM mock_items
WHERE id @@@ pdb.parse('last_updated_date:"2023-05-03"') ORDER BY id;

SELECT id, description, latest_available_time FROM mock_items
WHERE id @@@ pdb.parse('latest_available_time:"09:12:34"') ORDER BY id;

SELECT id, description, created_at_tz FROM mock_items
WHERE id @@@ pdb.parse('created_at_tz:"2023-05-01 09:12:34+00"') ORDER BY id;

SELECT id, description, latest_available_time_tz FROM mock_items
WHERE id @@@ pdb.parse('latest_available_time_tz:"09:12:34+00"') ORDER BY id;

-- Global parsing must apply the same logical-to-physical conversion as field-specific parsing.
-- INTEGER is the control: its logical and physical values are identical.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('id:1') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse_with_field('1') ORDER BY id;

-- Unlimited NUMERIC uses the NumericBytes representation.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:1234') ORDER BY id;

SELECT id FROM mock_items
WHERE arbitrary_precision @@@ pdb.parse_with_field('1234') ORDER BY id;

-- NUMERIC(10,2) uses a scaled Numeric64 representation.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:1') ORDER BY id;

SELECT id FROM mock_items
WHERE fixed_precision @@@ pdb.parse_with_field('1') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:1.23') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:100000000000000000000.123') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:-100') ORDER BY id;

-- Conversion also applies to range bounds and each field in a compound AST.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:[1 TO 2]') ORDER BY id;

SELECT id FROM mock_items
WHERE fixed_precision @@@ pdb.parse_with_field('[1 TO 2]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:[-100 TO 1234]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision: IN [0.01 2.5]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision: IN [1234 -100]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('(arbitrary_precision:1234 AND fixed_precision:1) OR id:2') ORDER BY id;

-- Invalid NUMERIC literals, range bounds, and set elements report a numeric conversion error.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:not-a-number') ORDER BY id;

SELECT id FROM mock_items
WHERE fixed_precision @@@ pdb.parse_with_field('not-a-number') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:[1 TO not-a-number]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision: IN [1234 not-a-number]') ORDER BY id;

-- PostgreSQL rejects these, so they must not be rounded or read as zero.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:1.2x') ORDER BY id;

SELECT id FROM mock_items
WHERE fixed_precision @@@ pdb.parse_with_field('1.2x') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:1..2') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:""') ORDER BY id;

-- Lenient parsing still ignores invalid phrases and converts the remaining NUMERIC values.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:not-a-number OR fixed_precision:1.23', lenient => true) ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision: IN [not-a-number 1.23]', lenient => true) ORDER BY id;

-- Tantivy's lenient parser makes an invalid bound unbounded, so this runs as [1 TO *].
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:[1 TO not-a-number]', lenient => true) ORDER BY id;

DROP TABLE mock_items;

-- NUMERIC literals compare exactly, as in PostgreSQL. A literal with more fractional digits
-- than NUMERIC(10,2) holds lies between two stored values: no row equals it, and a range
-- bound moves to the stored value on its side. Unlimited NUMERIC stores values exactly, so
-- each query must return the same rows for both columns.
CREATE TABLE numeric_literals (
    id INT,
    arbitrary_precision NUMERIC,
    fixed_precision NUMERIC(10, 2)
);
INSERT INTO numeric_literals VALUES
    (1, 1.00, 1.00),
    (2, 0.01, 0.01),
    (3, 1.23, 1.23),
    (4, 2.50, 2.50);
CREATE INDEX ON numeric_literals USING paradedb (id, arbitrary_precision, fixed_precision);

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:[1.001 TO 2]') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:[1.001 TO 2]') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:[1 TO 1.229]') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:[1 TO 1.229]') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:{0.999 TO 2}') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:{0.999 TO 2}') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:<1.234') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:<1.234') ORDER BY id;

-- These match no rows.
SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:1.001') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:1.001') ORDER BY id;

SELECT id FROM numeric_literals
WHERE arbitrary_precision @@@ pdb.parse_with_field('1.001') ORDER BY id;

SELECT id FROM numeric_literals
WHERE fixed_precision @@@ pdb.parse_with_field('1.001') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision: IN [1.001 2.504]') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision: IN [1.001 2.504]') ORDER BY id;

-- Invalid NUMERIC input can be valid Base64. Lenient parsing must not use it as bytes.
INSERT INTO numeric_literals VALUES (5, 0, 0), (6, -1, -1);

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:[null TO 1.23]', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:[null TO 1.23]', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:[0 TO abcd]', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:[0 TO abcd]', lenient => true) ORDER BY id;

-- Invalid literals must not become byte terms inside compound queries.
SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:null AND id:1', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:null AND id:1', lenient => true) ORDER BY id;

-- Tantivy's trim_ast skips Boost nodes, so a boosted empty clause still matches nothing.
SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('(arbitrary_precision:"gA==")^2 AND id:5', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('(fixed_precision:"gA==")^2 AND id:5', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('id:5 AND NOT arbitrary_precision:"gA=="', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE arbitrary_precision @@@ pdb.parse_with_field('"gA=="', lenient => true) ORDER BY id;

-- gA== encodes zero, but it is not a NUMERIC literal. Only valid set elements remain.
SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision: IN ["gA==" 1.23]', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision: IN ["gA==" 1.23]', lenient => true) ORDER BY id;

-- An empty set and a valid off-grid literal must still match nothing in an AND.
SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision: IN ["gA=="] AND id:5', lenient => true) ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('fixed_precision:1.001 AND id:1', lenient => true) ORDER BY id;

-- Strict parsing still rejects the same Base64-compatible inputs.
SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision:[null TO 1.23]') ORDER BY id;

SELECT id FROM numeric_literals
WHERE id @@@ pdb.parse('arbitrary_precision: IN ["gA=="]') ORDER BY id;

DROP TABLE numeric_literals;

-- Negative scales and large literals must not match physical integers or the NaN sentinel.
CREATE TABLE numeric_literal_limits (id INT, price NUMERIC(18, -2));
INSERT INTO numeric_literal_limits VALUES (1, 123400), (2, 1000), (3, 'NaN');
CREATE INDEX ON numeric_literal_limits USING paradedb (id, price);

SELECT id FROM numeric_literal_limits
WHERE id @@@ pdb.parse('price:1234') ORDER BY id;

SELECT id FROM numeric_literal_limits
WHERE price @@@ pdb.parse_with_field('1234') ORDER BY id;

SELECT id FROM numeric_literal_limits
WHERE id @@@ pdb.parse('price: IN [1234 1000 1e100]') ORDER BY id;

SELECT id FROM numeric_literal_limits
WHERE id @@@ pdb.parse('price:{999 TO 1234}') ORDER BY id;

SELECT id FROM numeric_literal_limits
WHERE id @@@ pdb.parse('price:1e100') ORDER BY id;

-- PostgreSQL sorts NaN above every number, so this range includes the NaN row.
SELECT id FROM numeric_literal_limits
WHERE id @@@ pdb.parse('price:[1e100 TO *]') ORDER BY id;

DROP TABLE numeric_literal_limits;
