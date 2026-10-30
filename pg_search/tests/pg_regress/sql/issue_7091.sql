-- Regression coverage for issue #7091.
--
-- Hash-join dynamic InList pushdown must not be rejected when the build-side
-- join key contains NULL. A NULL member never matches a non-negated IN, so it
-- is skipped instead of failing the whole term conversion.
--
-- Asserts:
--   1. Native PostgreSQL execution returns {104,105}
--   2. JoinScan EXPLAIN ANALYZE reports probe-side dynamic_filter_pushdown_*=1
--   3. JoinScan returns the same IDs, with NULL probe keys never matching

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan = off;
SET enable_nestloop = off;
SET enable_mergejoin = off;
SET paradedb.planner_warnings = off;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS null_key_dynamic_probe CASCADE;
DROP TABLE IF EXISTS null_key_dynamic_build CASCADE;

CREATE TABLE null_key_dynamic_probe (
    id bigint PRIMARY KEY,
    key bigint,
    body text NOT NULL
);
CREATE TABLE null_key_dynamic_build (
    id bigint PRIMARY KEY,
    key bigint,
    body text NOT NULL
);

INSERT INTO null_key_dynamic_probe
SELECT g, g, 'candidate'
FROM generate_series(101, 116) g;
INSERT INTO null_key_dynamic_probe VALUES
    (117, NULL, 'candidate'),
    (118, NULL, 'candidate');
INSERT INTO null_key_dynamic_build VALUES
    (1, 104, 'wanted'),
    (2, 105, 'wanted'),
    (3, NULL, 'wanted'),
    (4, 106, 'other');

CREATE INDEX null_key_dynamic_probe_idx ON null_key_dynamic_probe
USING paradedb (id, key, body);
CREATE INDEX null_key_dynamic_build_idx ON null_key_dynamic_build
USING paradedb (id, key, body);
ANALYZE null_key_dynamic_probe;
ANALYZE null_key_dynamic_build;

-- 1) Native PostgreSQL baseline
SET paradedb.enable_join_custom_scan = off;
SELECT array_agg(id ORDER BY id) AS native_ids
FROM (
    SELECT p.id
    FROM null_key_dynamic_build b
    JOIN null_key_dynamic_probe p ON b.key = p.key
    WHERE b.body ||| 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

-- 2) JoinScan with dynamic InList pushdown forced on
SET paradedb.enable_join_custom_scan = on;
SET paradedb.term_set_bitset_max_density_multi = 1.0;

CREATE OR REPLACE FUNCTION issue_7091_explain_analyze_lines(q text)
RETURNS SETOF text AS $$
DECLARE
    r record;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF) ' || q LOOP
        RETURN NEXT r."QUERY PLAN";
    END LOOP;
END $$ LANGUAGE plpgsql;

CREATE TEMP TABLE issue_7091_explain_analyze_output AS
SELECT line
FROM issue_7091_explain_analyze_lines(
    'SELECT p.id
     FROM null_key_dynamic_build b
     JOIN null_key_dynamic_probe p ON b.key = p.key
     WHERE b.body ||| ''wanted''
     ORDER BY p.id
     LIMIT 10'
) AS line;

-- EXPLAIN prints the relation alias (`table=p`), not the physical table name.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM issue_7091_explain_analyze_output
        WHERE line LIKE '%PgSearchScan: table=p%'
          AND line LIKE '%dynamic_filter_pushdown_%'
    ) THEN
        RAISE EXCEPTION
            'expected probe-side PgSearchScan with dynamic_filter_pushdown_*=1';
    END IF;
END $$;

SELECT bool_or(
    line LIKE '%PgSearchScan: table=p%'
    AND line LIKE '%dynamic_filter_pushdown_%'
) AS probe_dynamic_filter_pushdown
FROM issue_7091_explain_analyze_output;

DROP TABLE issue_7091_explain_analyze_output;
DROP FUNCTION issue_7091_explain_analyze_lines(text);

-- 3) JoinScan must match the native result set
SELECT array_agg(id ORDER BY id) AS joinscan_ids
FROM (
    SELECT p.id
    FROM null_key_dynamic_build b
    JOIN null_key_dynamic_probe p ON b.key = p.key
    WHERE b.body ||| 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

DROP TABLE null_key_dynamic_probe CASCADE;
DROP TABLE null_key_dynamic_build CASCADE;
