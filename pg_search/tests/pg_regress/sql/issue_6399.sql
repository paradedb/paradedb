-- Regression test for https://github.com/paradedb/paradedb/issues/6399:
-- COUNT(*) with `ctid IN (subquery)` failed with
-- "Pre-filter failed: Column 0 not fetched" under the aggregate custom scan.
-- The probe-side semi-join on `ctid` was applied as a pre-filter even though
-- `ctid` cannot be fetched before visibility checks, so it must be left to
-- the parent operator instead.

-- Setup
DROP TABLE IF EXISTS issue_6399_pf CASCADE;

CREATE TABLE issue_6399_pf (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO issue_6399_pf VALUES (1, 'alice'), (2, 'bob'), (3, 'bob');
CREATE INDEX issue_6399_pf_idx ON issue_6399_pf USING paradedb (id, name)
WITH (text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');

SET max_parallel_workers_per_gather = 0;

-- Previously errored with "Pre-filter failed: Column 0 not fetched".
SELECT count(*) FROM issue_6399_pf WHERE ctid IN (SELECT ctid FROM issue_6399_pf WHERE name @@@ 'bob');

-- Parity with the native plan.
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT count(*) FROM issue_6399_pf WHERE ctid IN (SELECT ctid FROM issue_6399_pf WHERE name @@@ 'bob');
SET paradedb.enable_aggregate_custom_scan TO on;

-- Non-aggregate shape over the same predicate still works.
SELECT id FROM issue_6399_pf WHERE ctid IN (SELECT ctid FROM issue_6399_pf WHERE name @@@ 'bob') ORDER BY id;

-- Teardown
DROP TABLE issue_6399_pf CASCADE;
