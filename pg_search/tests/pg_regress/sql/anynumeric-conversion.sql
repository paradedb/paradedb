\i common/common_setup.sql

DROP TABLE IF EXISTS numeric_conversion;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE numeric_conversion (
    id SERIAL PRIMARY KEY,
    description TEXT,
    amount numeric(36,0)
);

CREATE INDEX idx_numeric_conversion ON numeric_conversion USING bm25 (id, description, amount)
WITH (key_field = 'id');

--
-- Test thresholds for AnyNumeric -> TantivyValue conversion
--

-- 1. Safe F64 Range: [-9007199254740989, 9007199254740990]
-- Max Safe F64
INSERT INTO numeric_conversion (description, amount) VALUES ('Safe F64 Max', 9007199254740990);
-- Min Safe F64
INSERT INTO numeric_conversion (description, amount) VALUES ('Safe F64 Min', -9007199254740989);

-- 2. I64 Range (outside Safe F64):
-- Just above Max Safe F64
INSERT INTO numeric_conversion (description, amount) VALUES ('Unsafe I64 Upper', 9007199254740991);
-- Just below/at Min Safe F64 (Logic is > MIN_SAFE, so MIN_SAFE itself is excluded from F64)
INSERT INTO numeric_conversion (description, amount) VALUES ('Unsafe I64 Lower', -9007199254740990);

-- 3. I64 Limits
INSERT INTO numeric_conversion (description, amount) VALUES ('Max I64', 9223372036854775807);
INSERT INTO numeric_conversion (description, amount) VALUES ('Min I64', -9223372036854775808);

-- 4. U64 Range (outside I64)
-- Max I64 + 1
INSERT INTO numeric_conversion (description, amount) VALUES ('Min U64', 9223372036854775808);
-- Max U64
INSERT INTO numeric_conversion (description, amount) VALUES ('Max U64', 18446744073709551615);

-- 5. Above U64 (Fallback to F64)
-- Max U64 + 1
INSERT INTO numeric_conversion (description, amount) VALUES ('Above U64', 18446744073709551616);

-- For indexing of the mutable segment, which will trigger the type conversions.
-- TODO: See https://github.com/paradedb/paradedb/issues/3579
SELECT description, amount FROM numeric_conversion WHERE id @@@ pdb.all() ORDER BY id;

DROP TABLE numeric_conversion;
