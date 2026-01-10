-- 1. Setup: Create a temporary table for the demo
CREATE TEMP TABLE sorting_demo (
    id SERIAL PRIMARY KEY,
    description TEXT,
    regular_int INTEGER,
    data JSONB
);

-- 2. Insert Data: Covering standard ints, nulls, and JSONB variations
INSERT INTO sorting_demo (description, regular_int, data) VALUES
    ('Standard Positive', 10, '{"val": 10}'),
    ('Standard Negative', -5, '{"val": -5}'),
    ('Zero', 0, '{"val": 0}'),
    ('Large Number', 1000, '{"val": 1000}'),
    ('Null Column / Missing Key', NULL, '{}'),
    ('Explicit Null in JSON', 5, '{"val": null}'),
    ('String acting as Int', 20, '{"val": "20"}'); -- JSON string "20", not number 20

-- 3. Demo 1: Standard Integer Column Sorting
-- Default behavior: NULLs appear LAST in ASC order
SELECT
    'Standard Integer Column (ASC)' as test_case,
    description,
    regular_int
FROM sorting_demo
ORDER BY regular_int ASC;

-- 4. Demo 2: JSONB Field Sorting (Naive approach)
-- Note: This sorts purely by JSONB rules.
-- In JSONB, types have an ordering: Numbers < Strings < Nulls (conceptually)
SELECT
    'JSONB Raw Sort (ASC)' as test_case,
    description,
    data->'val' as raw_json_val
FROM sorting_demo
ORDER BY data->'val' ASC;

-- 5. Demo 3: JSONB Cast to Integer Sorting
-- This is how you typically WANT to sort (converting JSON to actual numbers).
-- Note: We exclude the string "20" row here to avoid casting errors,
-- or we can use NULLIF to handle safety.
SELECT
    'JSONB Cast to Integer (ASC)' as test_case,
    description,
    (data->>'val')::numeric as casted_val
FROM sorting_demo
WHERE jsonb_typeof(data->'val') = 'number' -- Only looking at actual JSON numbers
ORDER BY (data->>'val')::numeric ASC;

-- 6. Clean up (Optional, as TEMP tables drop automatically at session end)
DROP TABLE sorting_demo;