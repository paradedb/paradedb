-- Test NULLS FIRST vs NULLS LAST ordering in aggregate scans
-- This tests that NULL sentinels work correctly for different data types

-- =========================================
-- Setup: Create table with various types
-- =========================================
CREATE TABLE nulls_test (
    id SERIAL PRIMARY KEY,
    text_col TEXT,
    int_col BIGINT,
    float_col DOUBLE PRECISION,
    json_col JSONB
);

-- Insert test data with NULLs for each column type
INSERT INTO nulls_test (text_col, int_col, float_col, json_col) VALUES
    ('apple', 10, 1.5, '{"category": "fruit"}'),
    ('banana', 20, 2.5, '{"category": "fruit"}'),
    (NULL, 30, 3.5, '{"category": "veggie"}'),
    ('cherry', NULL, 4.5, '{"category": "fruit"}'),
    ('date', 50, NULL, NULL),
    ('elderberry', 60, 6.5, '{"category": "berry"}'),
    (NULL, NULL, NULL, NULL);

-- Create BM25 index with fast fields for all columns
CREATE INDEX idx_nulls_test ON nulls_test
USING bm25 (id, text_col, int_col, float_col, json_col)
WITH (
    key_field = 'id',
    text_fields = '{"text_col": {"indexed": true, "fast": true}}',
    numeric_fields = '{"int_col": {"indexed": true, "fast": true}, "float_col": {"indexed": true, "fast": true}}',
    json_fields = '{"json_col": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Enable aggregate custom scan
SET paradedb.enable_aggregate_custom_scan TO on;

-- =========================================
-- Test 1: Text column - NULLS LAST (default for ASC)
-- =========================================
SELECT text_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY text_col
ORDER BY text_col ASC;

-- =========================================
-- Test 2: Text column - NULLS FIRST (explicit)
-- =========================================
SELECT text_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY text_col
ORDER BY text_col ASC NULLS FIRST;

-- =========================================
-- Test 3: Text column - DESC NULLS FIRST (default for DESC)
-- =========================================
SELECT text_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY text_col
ORDER BY text_col DESC;

-- =========================================
-- Test 4: Text column - DESC NULLS LAST (explicit)
-- =========================================
SELECT text_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY text_col
ORDER BY text_col DESC NULLS LAST;

-- =========================================
-- Test 5: Integer column - NULLS LAST (default for ASC)
-- =========================================
SELECT int_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY int_col
ORDER BY int_col ASC;

-- =========================================
-- Test 6: Integer column - NULLS FIRST (explicit)
-- =========================================
SELECT int_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY int_col
ORDER BY int_col ASC NULLS FIRST;

-- =========================================
-- Test 7: Integer column - DESC NULLS FIRST (default for DESC)
-- =========================================
SELECT int_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY int_col
ORDER BY int_col DESC;

-- =========================================
-- Test 8: Integer column - DESC NULLS LAST (explicit)
-- =========================================
SELECT int_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY int_col
ORDER BY int_col DESC NULLS LAST;

-- =========================================
-- Test 9: Float column - NULLS LAST (default for ASC)
-- =========================================
SELECT float_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY float_col
ORDER BY float_col ASC;

-- =========================================
-- Test 10: Float column - NULLS FIRST (explicit)
-- =========================================
SELECT float_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY float_col
ORDER BY float_col ASC NULLS FIRST;

-- =========================================
-- Test 11: Float column - DESC NULLS FIRST (default for DESC)
-- =========================================
SELECT float_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY float_col
ORDER BY float_col DESC;

-- =========================================
-- Test 12: Float column - DESC NULLS LAST (explicit)
-- =========================================
SELECT float_col, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY float_col
ORDER BY float_col DESC NULLS LAST;

-- =========================================
-- Test 13: JSON field - NULLS LAST (default for ASC)
-- =========================================
SELECT json_col->>'category' AS category, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY json_col->>'category'
ORDER BY category ASC;

-- =========================================
-- Test 14: JSON field - NULLS FIRST (explicit)
-- =========================================
SELECT json_col->>'category' AS category, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY json_col->>'category'
ORDER BY category ASC NULLS FIRST;

-- =========================================
-- Test 15: JSON field - DESC NULLS FIRST (default for DESC)
-- =========================================
SELECT json_col->>'category' AS category, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY json_col->>'category'
ORDER BY category DESC;

-- =========================================
-- Test 16: JSON field - DESC NULLS LAST (explicit)
-- =========================================
SELECT json_col->>'category' AS category, COUNT(*) AS count
FROM nulls_test
WHERE id @@@ paradedb.all()
GROUP BY json_col->>'category'
ORDER BY category DESC NULLS LAST;

-- =========================================
-- Cleanup
-- =========================================
DROP TABLE nulls_test CASCADE;
