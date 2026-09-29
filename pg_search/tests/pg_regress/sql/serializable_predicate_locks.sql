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

-- Which scan ran is the point. The rest of a plan (worker selection, exec method, the
-- Tantivy query, the DataFusion physical plan, the rewritten index oid) moves with changes
-- this test has no opinion about.
CREATE FUNCTION ssi_plan_nodes(query text) RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (COSTS OFF) ' || query LOOP
        line := regexp_replace(btrim(line), '^-> *', '');
        IF line ~ '^(Custom Scan|Index Scan|Index Only Scan|Seq Scan|Bitmap)' THEN
            RETURN NEXT line;
        END IF;
    END LOOP;
END;
$$;

-- Only this backend's locks: a lock another session left behind is not something this test
-- controls. Names instead of oids, and deduplicated, because the Postgres plan in the last
-- section leaves one lock per heap tuple it reads. That last section reports `tuple` only
-- while the matches on a page stay within `max_pred_locks_per_page`; a third match there
-- promotes the report to `page`.
CREATE VIEW ssi_locks AS
SELECT DISTINCT locktype, relation::regclass::text AS relation, mode
FROM pg_locks
WHERE mode = 'SIReadLock'
  AND pid = pg_backend_pid()
  AND relation IN ('ssi_doctors'::regclass, 'ssi_doctors_idx'::regclass,
                   'ssi_shifts'::regclass, 'ssi_shifts_idx'::regclass);

-- Below SERIALIZABLE there is no lock to take.
BEGIN;
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- An EXPLAIN without ANALYZE reads nothing, so it must take no lock.
BEGIN ISOLATION LEVEL SERIALIZABLE;
EXPLAIN (COSTS OFF)
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- A node that Postgres initializes and never executes reads nothing, so it owes nothing
-- either. `index_beginscan` follows the same rule: it runs on the first `IndexNext`.
BEGIN ISOLATION LEVEL SERIALIZABLE;
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' LIMIT 0;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- base scan
BEGIN ISOLATION LEVEL SERIALIZABLE;
SELECT * FROM ssi_plan_nodes($$SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id$$);
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- base scan answered from the columnar store, which never reaches the heap
BEGIN ISOLATION LEVEL SERIALIZABLE;
SELECT * FROM ssi_plan_nodes($$SELECT name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY name$$);
SELECT name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY name;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- aggregate scan
BEGIN ISOLATION LEVEL SERIALIZABLE;
SELECT * FROM ssi_plan_nodes($$SELECT count(*) FROM ssi_doctors WHERE status @@@ 'oncall'$$);
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
SELECT * FROM ssi_plan_nodes($$SELECT d.id, s.ward FROM ssi_doctors d JOIN ssi_shifts s ON d.id = s.doctor_id
WHERE d.status @@@ 'oncall' AND s.ward @@@ 'ward1' ORDER BY d.id LIMIT 10$$);
SELECT d.id, s.ward FROM ssi_doctors d JOIN ssi_shifts s ON d.id = s.doctor_id
WHERE d.status @@@ 'oncall' AND s.ward @@@ 'ward1' ORDER BY d.id LIMIT 10;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

-- Postgres' own plan for the same read locks the index, because the bm25 access method
-- sets no `ampredlocks`, plus the heap tuples it fetches.
BEGIN ISOLATION LEVEL SERIALIZABLE;
SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL paradedb.enable_aggregate_custom_scan = off;
SET LOCAL paradedb.planner_warnings = 'off';
SET LOCAL enable_seqscan = off;
SELECT * FROM ssi_plan_nodes($$SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id$$);
SELECT id, name FROM ssi_doctors WHERE status @@@ 'oncall' ORDER BY id;
SELECT * FROM ssi_locks ORDER BY 1, 2;
COMMIT;

DROP VIEW ssi_locks;
DROP FUNCTION ssi_plan_nodes(text);
DROP TABLE ssi_shifts;
DROP TABLE ssi_doctors;
