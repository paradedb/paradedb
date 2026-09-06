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
USING paradedb (id, description, rating, category, metadata, created_at, last_updated_date, latest_available_time, created_at_tz, latest_available_time_tz, arbitrary_precision, fixed_precision)
WITH (key_field='id');

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

-- Conversion also applies to range bounds and each field in a compound AST.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision:[1 TO 2]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:[-100 TO 1234]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('fixed_precision: IN [0.01 2.5]') ORDER BY id;

SELECT id FROM mock_items
WHERE id @@@ pdb.parse('(arbitrary_precision:1234 AND fixed_precision:1) OR id:2') ORDER BY id;

-- Failed logical conversion leaves the phrase untouched for Tantivy's normal error path.
SELECT id FROM mock_items
WHERE id @@@ pdb.parse('arbitrary_precision:not-a-number') ORDER BY id;

DROP TABLE mock_items;
