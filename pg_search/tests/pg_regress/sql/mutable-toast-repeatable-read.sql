\i common/common_setup.sql

-- Regression test for RECENTLY_DEAD tuple visibility in mutable segments.
-- Verifies that RECENTLY_DEAD tuples are indexed during materialization so
-- that the TOAST race in #5076 is prevented by the active snapshot guard,
-- while still preserving visibility for older snapshots (#3709).
-- See: https://github.com/paradedb/paradedb/issues/5076
--      https://github.com/paradedb/paradedb/pull/3709

DROP TABLE IF EXISTS toast_rr_test;
CREATE TABLE toast_rr_test (
    id SERIAL PRIMARY KEY,
    body TEXT
);

CREATE INDEX toast_rr_idx ON toast_rr_test
USING bm25 (id, body)
WITH (key_field=id, mutable_segment_rows=2);

-- Insert a row with a large TOASTed value.
INSERT INTO toast_rr_test (body) VALUES (repeat('hello ', 200000));

-- Confirm the row is searchable.
SELECT id FROM toast_rr_test WHERE body ||| 'hello';

-- Now UPDATE the row (autocommit). This makes the old tuple RECENTLY_DEAD.
UPDATE toast_rr_test SET body = repeat('world ', 200000) WHERE id = 1;

-- The updated value should now be searchable.
SELECT id FROM toast_rr_test WHERE body ||| 'world';

-- The old value should no longer be visible (the UPDATE committed).
SELECT id FROM toast_rr_test WHERE body ||| 'hello';

DROP TABLE toast_rr_test;
