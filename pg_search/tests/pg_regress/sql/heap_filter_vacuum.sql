\i common/common_setup.sql

-- Test: BM25 queries with heap_filter predicates survive VACUUM truncation.
-- Issue #4333: stale ctids pointing to truncated blocks caused ReadBuffer errors.

-- Create table with an extra non-indexed column to force heap_filter evaluation.
CREATE TABLE hfv_test (
    id SERIAL PRIMARY KEY,
    body TEXT NOT NULL,
    extra INT NOT NULL DEFAULT 0
);

-- Insert enough rows to span many heap pages.
INSERT INTO hfv_test (body, extra)
SELECT 'the quick brown fox jumps over the lazy dog', g % 10
FROM generate_series(1, 5000) g;

-- Create BM25 index only on id and body (extra is NOT indexed).
CREATE INDEX hfv_idx ON hfv_test USING bm25 (id, body) WITH (key_field = 'id');

-- Baseline: query with heap_filter predicate works before any deletes.
SELECT count(*) FROM hfv_test WHERE body @@@ 'fox' AND extra = 5;

-- Delete trailing rows so VACUUM can truncate pages.
DELETE FROM hfv_test WHERE id > 1000;

-- Force VACUUM to reclaim and truncate pages.
VACUUM hfv_test;

-- This query uses the heap_filter path (extra is not in the BM25 index).
-- Before the fix, this would ERROR with "could not read blocks ... read only 0 of 8192 bytes".
SELECT count(*) FROM hfv_test WHERE body @@@ 'fox' AND extra = 5;

-- Also verify a simple BM25 query still works.
SELECT count(*) FROM hfv_test WHERE body @@@ 'fox';

-- Cleanup
DROP TABLE hfv_test;
