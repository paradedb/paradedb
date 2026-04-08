\i common/common_setup.sql

-- Test: HeapFilterReason variants produce the expected WARNING.

CREATE TABLE hfw_test (
    id SERIAL PRIMARY KEY,
    description TEXT NOT NULL,
    extra_int INT NOT NULL DEFAULT 0,
    flag BOOLEAN NOT NULL DEFAULT false
);

INSERT INTO hfw_test (description, extra_int, flag)
VALUES
    ('hello world', 1, true),
    ('hello parade', 2, false),
    ('other text', 3, true);

CREATE INDEX hfw_idx ON hfw_test USING bm25 (id, description) WITH (key_field = 'id');

SET enable_seqscan = off;
SET enable_bitmapscan = off;
SET paradedb.enable_filter_pushdown = on;

----------------------------------------------------------------------
-- 1. ColumnNotIndexed: non-indexed column in a comparison
--    WARNING: heap filter on `(extra_int = 1)`: the column is not
--    indexed in the bm25 index
----------------------------------------------------------------------
SELECT id, description FROM hfw_test
WHERE description @@@ 'hello' AND extra_int = 1
ORDER BY id;

----------------------------------------------------------------------
-- 2. BoolTestRequiresHeap: IS TRUE on non-indexed boolean
--    WARNING: heap filter on `(flag IS TRUE)`: boolean tests on
--    non-indexed fields require heap access
----------------------------------------------------------------------
SELECT id, description FROM hfw_test
WHERE description @@@ 'hello' AND flag IS TRUE
ORDER BY id;

----------------------------------------------------------------------
-- 3. FunctionNotIndexable: standalone boolean function referencing
--    our relation. Use a C-language or non-inlinable function so
--    PostgreSQL doesn't inline it into an OpExpr.
--    WARNING: heap filter on `is_positive_strict(extra_int)`:
--    function expressions are not indexable
----------------------------------------------------------------------
CREATE FUNCTION is_positive_strict(val INT) RETURNS BOOLEAN
    LANGUAGE plpgsql IMMUTABLE AS $$ BEGIN RETURN val > 0; END $$;

SELECT id, description FROM hfw_test
WHERE description @@@ 'hello' AND is_positive_strict(extra_int)
ORDER BY id;

DROP FUNCTION is_positive_strict;

----------------------------------------------------------------------
-- 4. No warning: all predicates are indexed
----------------------------------------------------------------------
SELECT id, description FROM hfw_test
WHERE description @@@ 'hello'
ORDER BY id;

-- Cleanup
DROP TABLE hfw_test;

\i common/common_cleanup.sql
