-- Regression coverage for issue #6158.
--
-- Hash-join dynamic InList pushdown must not re-scale Numeric64 execution
-- values. For numeric(10,2), 104.25 is stored / published as Int64(10425).
-- Passing that through from_scalar() would multiply by 10^2 again and push
-- Tantivy terms that reject the matching probe rows.
--
-- Asserts:
--   1. Native PostgreSQL execution returns {104,105,106}
--   2. JoinScan EXPLAIN ANALYZE reports probe-side dynamic_filter_pushdown_*=1
--   3. JoinScan returns the same IDs

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan = off;
SET enable_nestloop = off;
SET enable_mergejoin = off;
SET paradedb.planner_warnings = off;

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS numeric64_dynamic_probe CASCADE;
DROP TABLE IF EXISTS numeric64_dynamic_build CASCADE;

CREATE TABLE numeric64_dynamic_probe (
    id bigint PRIMARY KEY,
    amount numeric(10, 2) NOT NULL,
    body text NOT NULL
);
CREATE TABLE numeric64_dynamic_build (
    id bigint PRIMARY KEY,
    amount numeric(10, 2) NOT NULL,
    body text NOT NULL
);

INSERT INTO numeric64_dynamic_probe
SELECT g, g::numeric + 0.25, 'candidate'
FROM generate_series(101, 116) g;
INSERT INTO numeric64_dynamic_build VALUES
    (1, 104.25, 'wanted'),
    (2, 105.25, 'wanted'),
    (3, 106.25, 'wanted');

CREATE INDEX numeric64_dynamic_probe_idx ON numeric64_dynamic_probe
USING paradedb (id, amount, body) WITH (key_field = 'id');
CREATE INDEX numeric64_dynamic_build_idx ON numeric64_dynamic_build
USING paradedb (id, amount, body) WITH (key_field = 'id');
ANALYZE numeric64_dynamic_probe;
ANALYZE numeric64_dynamic_build;

-- 1) Native PostgreSQL baseline
SET paradedb.enable_join_custom_scan = off;
SELECT array_agg(id ORDER BY id) AS native_ids
FROM (
    SELECT p.id
    FROM numeric64_dynamic_build b
    JOIN numeric64_dynamic_probe p ON b.amount = p.amount
    WHERE b.body @@@ 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

-- 2) JoinScan with dynamic InList pushdown forced on
SET paradedb.enable_join_custom_scan = on;
SET paradedb.term_set_bitset_max_density_multi = 1.0;

CREATE OR REPLACE FUNCTION issue_6158_explain_analyze_lines(q text)
RETURNS SETOF text AS $$
DECLARE
    r record;
BEGIN
    FOR r IN EXECUTE 'EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF, BUFFERS OFF, SUMMARY OFF) ' || q LOOP
        RETURN NEXT r."QUERY PLAN";
    END LOOP;
END $$ LANGUAGE plpgsql;

CREATE TEMP TABLE issue_6158_explain_analyze_output AS
SELECT line
FROM issue_6158_explain_analyze_lines(
    'SELECT p.id
     FROM numeric64_dynamic_build b
     JOIN numeric64_dynamic_probe p ON b.amount = p.amount
     WHERE b.body @@@ ''wanted''
     ORDER BY p.id
     LIMIT 10'
) AS line;

-- EXPLAIN prints the relation alias (`table=p`), not the physical table name.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM issue_6158_explain_analyze_output
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
FROM issue_6158_explain_analyze_output;

DROP TABLE issue_6158_explain_analyze_output;
DROP FUNCTION issue_6158_explain_analyze_lines(text);

-- 3) JoinScan must match the native result set
SELECT array_agg(id ORDER BY id) AS joinscan_ids
FROM (
    SELECT p.id
    FROM numeric64_dynamic_build b
    JOIN numeric64_dynamic_probe p ON b.amount = p.amount
    WHERE b.body @@@ 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

DROP TABLE numeric64_dynamic_probe CASCADE;
DROP TABLE numeric64_dynamic_build CASCADE;

-- 4) Negatives, zero, and scale-0 Numeric64 must also survive pushdown.
DROP TABLE IF EXISTS numeric64_edge_probe CASCADE;
DROP TABLE IF EXISTS numeric64_edge_build CASCADE;

CREATE TABLE numeric64_edge_probe (
    id bigint PRIMARY KEY,
    amount numeric(10, 2) NOT NULL,
    whole numeric(10, 0) NOT NULL,
    body text NOT NULL
);
CREATE TABLE numeric64_edge_build (
    id bigint PRIMARY KEY,
    amount numeric(10, 2) NOT NULL,
    whole numeric(10, 0) NOT NULL,
    body text NOT NULL
);

INSERT INTO numeric64_edge_probe VALUES
    (1, -104.25, -104, 'candidate'),
    (2, 0.00, 0, 'candidate'),
    (3, 104.25, 104, 'candidate'),
    (4, 999999.99, 999999, 'candidate'),
    (5, -0.01, -1, 'candidate');
INSERT INTO numeric64_edge_build VALUES
    (10, -104.25, -104, 'wanted'),
    (11, 0.00, 0, 'wanted'),
    (12, 104.25, 104, 'wanted'),
    (13, -0.01, -1, 'wanted');

CREATE INDEX numeric64_edge_probe_idx ON numeric64_edge_probe
USING paradedb (id, amount, whole, body) WITH (key_field = 'id');
CREATE INDEX numeric64_edge_build_idx ON numeric64_edge_build
USING paradedb (id, amount, whole, body) WITH (key_field = 'id');
ANALYZE numeric64_edge_probe;
ANALYZE numeric64_edge_build;

SET paradedb.enable_join_custom_scan = off;
SELECT array_agg(id ORDER BY id) AS native_amount_edge
FROM (
    SELECT p.id
    FROM numeric64_edge_build b
    JOIN numeric64_edge_probe p ON b.amount = p.amount
    WHERE b.body @@@ 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

SELECT array_agg(id ORDER BY id) AS native_whole_edge
FROM (
    SELECT p.id
    FROM numeric64_edge_build b
    JOIN numeric64_edge_probe p ON b.whole = p.whole
    WHERE b.body @@@ 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

SET paradedb.enable_join_custom_scan = on;
SET paradedb.term_set_bitset_max_density_multi = 1.0;

SELECT array_agg(id ORDER BY id) AS joinscan_amount_edge
FROM (
    SELECT p.id
    FROM numeric64_edge_build b
    JOIN numeric64_edge_probe p ON b.amount = p.amount
    WHERE b.body @@@ 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

SELECT array_agg(id ORDER BY id) AS joinscan_whole_edge
FROM (
    SELECT p.id
    FROM numeric64_edge_build b
    JOIN numeric64_edge_probe p ON b.whole = p.whole
    WHERE b.body @@@ 'wanted'
    ORDER BY p.id
    LIMIT 10
) matched;

DROP TABLE numeric64_edge_probe CASCADE;
DROP TABLE numeric64_edge_build CASCADE;