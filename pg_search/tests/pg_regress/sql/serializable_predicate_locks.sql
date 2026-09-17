-- A plain index scan gets its SIREAD lock from `index_beginscan`. Our scans read the bm25
-- index without it, so they take the lock themselves. The lock sits on the heap relation,
-- which is the target every write to the table conflict-checks.
\echo 'Test: SERIALIZABLE predicate locks'

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS ssi_doctors CASCADE;
CREATE TABLE ssi_doctors (id int PRIMARY KEY, name text, status text);
INSERT INTO ssi_doctors
SELECT g, 'doc' || g, CASE WHEN g <= 2 THEN 'oncall' ELSE 'offcall' END
FROM generate_series(1, 10) g;
CREATE INDEX ssi_doctors_idx ON ssi_doctors USING bm25 (id, name, status)
WITH (text_fields = '{"status": {"tokenizer": {"type": "keyword"}, "fast": true}, "name": {"fast": true}}');
-- Until the table has row stats, the base scan reports its worker selection as the row
-- heuristic, and whether the build left stats behind varies by cluster.
ANALYZE ssi_doctors;

DROP TABLE IF EXISTS ssi_shifts CASCADE;
CREATE TABLE ssi_shifts (id int PRIMARY KEY, doctor_id int, ward text);
INSERT INTO ssi_shifts SELECT g, g, 'ward' || g FROM generate_series(1, 10) g;
CREATE INDEX ssi_shifts_idx ON ssi_shifts USING bm25 (id, doctor_id, ward)
WITH (text_fields = '{"ward": {"tokenizer": {"type": "keyword"}, "fast": true}}');
ANALYZE ssi_shifts;

-- Names instead of oids, and deduplicated: the Postgres plan in the last section leaves a
-- lock per heap tuple it reads.
CREATE VIEW ssi_locks AS
SELECT DISTINCT locktype, relation::regclass::text AS relation, mode
FROM pg_locks
WHERE mode = 'SIReadLock'
  AND relation IN ('ssi_doctors'::regclass, 'ssi_doctors_idx'::regclass,
                   'ssi_shifts'::regclass, 'ssi_shifts_idx'::regclass);

-- An EXPLAIN without ANALYZE reads nothing, so it must take no lock.
BEGIN ISOLATION LEVEL SERIALIZABLE;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- base scan
BEGIN ISOLATION LEVEL SERIALIZABLE;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- base scan answered from the columnar store, which never reaches the heap
BEGIN ISOLATION LEVEL SERIALIZABLE;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY name;
SELECT name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY name;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- aggregate scan
BEGIN ISOLATION LEVEL SERIALIZABLE;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall';
SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall';
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- the `paradedb.aggregate` function, which the planner never sees
BEGIN ISOLATION LEVEL SERIALIZABLE;
SELECT * FROM paradedb.aggregate(index=>'ssi_doctors_idx',
                                 query=>paradedb.term('status', 'oncall'),
                                 agg=>'{"matches": {"value_count": {"field": "id"}}}');
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- join scan, which locks every source it reads. It needs the LIMIT to be chosen at all.
BEGIN ISOLATION LEVEL SERIALIZABLE;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT d.id, s.ward FROM ssi_doctors d JOIN ssi_shifts s ON d.id = s.doctor_id
WHERE d.status @@@ 'oncall' AND s.ward @@@ 'ward1' ORDER BY d.id LIMIT 10;
SELECT d.id, s.ward FROM ssi_doctors d JOIN ssi_shifts s ON d.id = s.doctor_id
WHERE d.status @@@ 'oncall' AND s.ward @@@ 'ward1' ORDER BY d.id LIMIT 10;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- Postgres' own plan for the same read locks the index, because the bm25 access method
-- does not set `ampredlocks`.
BEGIN ISOLATION LEVEL SERIALIZABLE;
SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL paradedb.enable_aggregate_custom_scan = off;
SET LOCAL paradedb.planner_warnings = 'off';
SET LOCAL enable_seqscan = off;
-- No EXPLAIN here: the pushed-down `Index Cond` prints the index oid, which moves from run
-- to run. The locked relation below says which plan ran.
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

DROP VIEW ssi_locks;
DROP TABLE ssi_shifts;
DROP TABLE ssi_doctors;
