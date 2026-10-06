-- Issue #6399: COUNT(*) with `ctid IN (subquery)` failed with
-- "Pre-filter failed: Column 0 not fetched". The aggregate scan plans it as a
-- RightSemi hash join on ctid, and the join's dynamic filter was applied as a
-- pre-filter on a ctid column the scanner only fills in after visibility checks.
CREATE EXTENSION IF NOT EXISTS pg_search;

SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_aggregate_custom_scan TO on;

CREATE TABLE issue_6399 (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO issue_6399 VALUES (1, 'alice'), (2, 'bob'), (3, 'bob');
CREATE INDEX issue_6399_idx ON issue_6399 USING paradedb (id, name)
WITH (text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');

-- The plan must be the aggregate scan with a RightSemi hash join on ctid,
-- pushing a dynamic filter down to the inner scan.
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'bob');

SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'bob');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'alice');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'carol');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE id > 1);

-- Same answers with the aggregate scan turned off.
SET paradedb.enable_aggregate_custom_scan = off;
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'bob');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'alice');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'carol');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE id > 1);
SET paradedb.enable_aggregate_custom_scan TO on;

-- HOT update on an unindexed column: the index is not updated, so the ctid it
-- stored for the row goes stale and visibility checking must follow the HOT
-- chain to the visible version. A pre-filter on ctid would compare that stale
-- ctid and could drop the row; skipping it keeps the count correct.
ALTER TABLE issue_6399 ADD COLUMN note TEXT;
UPDATE issue_6399 SET note = 'seen' WHERE id = 2;
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'bob');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'alice');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE name @@@ 'carol');
SELECT count(*) FROM issue_6399 WHERE ctid IN (SELECT ctid FROM issue_6399 WHERE id > 1);

DROP TABLE issue_6399;
RESET paradedb.enable_aggregate_custom_scan;
RESET max_parallel_workers_per_gather;
