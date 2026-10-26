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

-- Unfielded literals must use the NUMERIC value, not its scaled integer or byte encoding.
CREATE TABLE unfielded_numeric (
    id INT PRIMARY KEY,
    description TEXT,
    fixed_precision NUMERIC(10,2),
    arbitrary_precision NUMERIC
);
INSERT INTO unfielded_numeric VALUES
    (1, 'alpha', 1.23, 5),
    (2, 'beta', 123, 123),
    (3, 'gamma', 0.01, 7);
CREATE INDEX unfielded_numeric_idx ON unfielded_numeric
USING bm25 (id, description, fixed_precision, arbitrary_precision);

SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('1.23') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('1.23', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('5') ORDER BY id;

-- Field-qualified parsing and the other all-fields entry points agree.
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('fixed_precision:123 OR arbitrary_precision:123') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE fixed_precision @@@ pdb.parse_with_field('1.23') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE unfielded_numeric @@@ pdb.parse('1.23') ORDER BY id;

-- A text match remains valid even though numeric fields cannot parse the same input.
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('alpha') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123 AND beta') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123 OR beta') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123 beta', conjunction_mode => true) ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123 AND beta', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('1.23^2') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('beta AND -1.23') ORDER BY id;

-- Raw Base64 must not match NumericBytes zero, but can still match text.
INSERT INTO unfielded_numeric VALUES
    (4, 'delta', 0, 0),
    (5, 'gA== 123', NULL, NULL),
    (6, 'alpha beta', NULL, NULL);
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('"gA=="') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('"gA=="', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('123') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('"alpha beta"') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('"alpha beta"~1') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('"alpha be"*') ORDER BY id;

-- Explicitly invalid fields and malformed syntax must still fail in strict mode.
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('fixed_precision:alpha OR alpha') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('missing:123 OR alpha') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('alpha AND (') ORDER BY id;
SELECT id FROM unfielded_numeric
WHERE id @@@ pdb.parse('fixed_precision:alpha OR alpha', lenient => true) ORDER BY id;

DROP TABLE unfielded_numeric;

-- Each field needs its own scale. NUMERIC(30,2) also exercises typmod-constrained bytes.
CREATE TABLE unfielded_numeric_scales (
    id INT PRIMARY KEY,
    cents NUMERIC(10,2),
    mills NUMERIC(10,3),
    hundreds NUMERIC(18,-2),
    wide NUMERIC(30,2)
);
INSERT INTO unfielded_numeric_scales VALUES
    (11, 1.23, 0.123, 123400, 9),
    (12, 123, 1.23, 1000, 123),
    (13, 'NaN', 'NaN', 'NaN', 'NaN'),
    (14, 0, 0, 0, 0);
CREATE INDEX unfielded_numeric_scales_idx ON unfielded_numeric_scales
USING bm25 (id, cents, mills, hundreds, wide);

SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('1.23') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('0.123') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('1234') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('123400') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('9') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('9223372036854775807') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('NaN') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('0') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('"gA=="', lenient => true) ORDER BY id;

-- A valid but unrepresentable equality must not disappear from an AND.
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('0.0001 AND id:11') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('0.0001 AND id:11', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('id:11 AND -0.0001') ORDER BY id;

-- With no compatible field, strict parsing still reports an error. Lenient parsing drops it.
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('invalid') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('invalid AND id:11', lenient => true) ORDER BY id;

-- PostgreSQL rejects '.', so it must not become a NumericBytes zero term.
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('"."') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('"." AND id:14', lenient => true) ORDER BY id;

-- Tantivy does not trim empty clauses under Boost; an invalid boosted literal stays false.
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('("gA==")^2 AND id:14', lenient => true) ORDER BY id;

-- A valid out-of-domain equality stays false instead of being discarded in lenient mode.
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('1e100 AND id:14', lenient => true) ORDER BY id;

-- Quote unfielded negatives so '-' is part of the number, not the exclusion operator.
INSERT INTO unfielded_numeric_scales VALUES (15, -1.23, -0.123, -1000, -100);
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('"-1.23"') ORDER BY id;
SELECT id FROM unfielded_numeric_scales
WHERE id @@@ pdb.parse('"-100"') ORDER BY id;

DROP TABLE unfielded_numeric_scales;

-- Unindexed NUMERIC fields must not turn a discarded literal into an empty equality.
CREATE TABLE unfielded_numeric_unindexed (id INT PRIMARY KEY, amount NUMERIC(10,2));
INSERT INTO unfielded_numeric_unindexed VALUES (1, 1.23);
CREATE INDEX ON unfielded_numeric_unindexed USING bm25 (id, amount)
WITH (numeric_fields = '{"amount":{"indexed":false,"fast":true}}');
SELECT id FROM unfielded_numeric_unindexed
WHERE id @@@ pdb.parse('1.234 AND id:1', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric_unindexed
WHERE id @@@ pdb.parse('1.234') ORDER BY id;
DROP TABLE unfielded_numeric_unindexed;

-- Expansion preserves aliased tokenizers, stopword removal, and score boosts.
CREATE TABLE unfielded_numeric_tokenizer (id INT PRIMARY KEY, amount NUMERIC(10,2), title TEXT);
INSERT INTO unfielded_numeric_tokenizer VALUES (1, 1.23, 'the alpha'), (2, 123, 'the beta');
CREATE INDEX ON unfielded_numeric_tokenizer USING bm25 (
    id, amount, (title::pdb.simple('alias=words', 'stopwords_language=english'))
);
SELECT id FROM unfielded_numeric_tokenizer
WHERE id @@@ pdb.parse('alpha') ORDER BY id;
SELECT id FROM unfielded_numeric_tokenizer
WHERE id @@@ pdb.parse('words:alpha') ORDER BY id;
SELECT id FROM unfielded_numeric_tokenizer
WHERE id @@@ pdb.parse('the') ORDER BY id;
SELECT id FROM unfielded_numeric_tokenizer
WHERE id @@@ pdb.parse('the AND alpha', lenient => true) ORDER BY id;
SELECT id FROM unfielded_numeric_tokenizer
WHERE id @@@ pdb.parse('the AND 1.23', lenient => true) ORDER BY id;
SELECT
    (SELECT paradedb.score(id) FROM unfielded_numeric_tokenizer WHERE id @@@ pdb.parse('1.23^2')) =
    2 * (SELECT paradedb.score(id) FROM unfielded_numeric_tokenizer WHERE id @@@ pdb.parse('1.23'))
    AS doubled_score;
DROP TABLE unfielded_numeric_tokenizer;
