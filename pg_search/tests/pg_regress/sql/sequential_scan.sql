\i common/common_setup.sql

DROP TABLE IF EXISTS sequential_scan CASCADE;
CREATE TABLE sequential_scan (id int PRIMARY KEY, body text NOT NULL, keep boolean NOT NULL DEFAULT true);
INSERT INTO sequential_scan SELECT g, 'keyword number ' || g, true FROM generate_series(1, 20000) g;
CREATE INDEX sequential_scan_idx ON sequential_scan USING bm25 (id, body) WITH (key_field = 'id') WHERE keep;

-- Silence the (unrelated) "Aggregate Scan not used" partial-index diagnostic so the output shows
-- only the per-row filter's own warnings.
SET paradedb.check_aggregate_scan = false;

-- The per-row filter's deparsed qual embeds the BM25 index oid, which changes on every database
-- creation; mask it so the plans below are stable.
CREATE FUNCTION explain_seqscan(query text) RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) ' || query LOOP
        RETURN NEXT regexp_replace(line, '"oid":\d+', '"oid":N');
    END LOOP;
END;
$$;

-- Tiny work_mem: the ~20k-key match set exceeds it and spills; count is still correct.
SET work_mem = '64kB';
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword'$$);
SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword';

-- Membership correctness across the spilled, on-disk sorted set (probes low/mid/high keys).
SELECT explain_seqscan($$SELECT id FROM sequential_scan WHERE body ||| 'keyword' AND id IN (1, 10000, 20000) ORDER BY id$$);
SELECT id FROM sequential_scan WHERE body ||| 'keyword' AND id IN (1, 10000, 20000) ORDER BY id;

-- Negation over the spilled set: everything matched, so NOT (...) excludes all rows.
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE NOT (body ||| 'keyword')$$);
SELECT count(*) FROM sequential_scan WHERE NOT (body ||| 'keyword');

-- A term matched by no row -> empty set -> nothing matches (no spill).
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE body ||| 'nonexistentterm'$$);
SELECT count(*) FROM sequential_scan WHERE body ||| 'nonexistentterm';

-- With ample work_mem the identical query stays in memory: no spill WARNING.
SET work_mem = '256MB';
SELECT explain_seqscan($$SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword'$$);
SELECT count(*) FROM sequential_scan WHERE body ||| 'keyword';

DROP TABLE sequential_scan CASCADE;
