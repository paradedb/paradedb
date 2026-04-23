-- =====================================================================
-- Issue #4849: Aggregate-on-JOIN with BM25 aliased fields
-- =====================================================================
-- Verifies that GROUP BY columns, aggregate arguments, and aggregate-
-- internal ORDER BY resolve to the BM25 field alias (not the heap
-- attribute name) when building the DataFusion aggregate plan over a
-- join. Each test has three parts:
--   1. EXPLAIN to confirm the ParadeDB Aggregate Scan is used (no fallback).
--   2. Run the query with the custom scan on — the path this fix covers.
--   3. Run the same query with the custom scan off — native Postgres parity.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_join_custom_scan TO on;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Test Data
CREATE TABLE repro_cccf (
    company_id bigint PRIMARY KEY,
    company_name text,
    company_domain text
);

CREATE TABLE repro_ti (
    company_id bigint PRIMARY KEY
);

INSERT INTO repro_cccf VALUES
    (1, 'Acme Corp', 'acme.com'),
    (2, 'Globex Inc', 'globex.com'),
    (3, 'Initech', 'initech.com');

INSERT INTO repro_ti VALUES (1), (2), (3);

-- BM25 index where company_name is aliased to company_name_words.
-- This causes the DataFusion schema to register the field as
-- "company_name_words", so the planner must resolve GROUP BY / aggregate
-- arg / aggregate ORDER BY names through the BM25 index too.
CREATE INDEX repro_cccf_idx ON repro_cccf
USING bm25 (
    company_id,
    (lower(company_domain)::pdb.literal_normalized('ascii_folding=true')),
    (company_name::pdb.simple('alias=company_name_words', 'columnar=true'))
) WITH (key_field=company_id);

CREATE INDEX repro_ti_idx ON repro_ti
USING bm25 (company_id) WITH (key_field=company_id);

-- =====================================================================
-- Test 1: GROUP BY on the aliased column
-- Previously crashed with "No field named cccf_0.company_name".
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT company_name, count(*)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id
GROUP BY company_name
ORDER BY company_name;

SELECT company_name, count(*)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id
GROUP BY company_name
ORDER BY company_name;

-- Parity: same query with custom scan off (native Postgres).
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT company_name, count(*)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id
GROUP BY company_name
ORDER BY company_name;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 2: Aggregate argument referencing the aliased column
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT count(DISTINCT company_name)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id;

SELECT count(DISTINCT company_name)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id;

-- Parity: same query with custom scan off.
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT count(DISTINCT company_name)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id;
SET paradedb.enable_aggregate_custom_scan TO on;

-- =====================================================================
-- Test 3: Aggregate-internal ORDER BY referencing the aliased column
-- Exercises the third fieldname_from_var call site (aggregate ORDER BY).
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT string_agg(company_name, ',' ORDER BY company_name)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id;

SELECT string_agg(company_name, ',' ORDER BY company_name)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id;

-- Parity: same query with custom scan off.
SET paradedb.enable_aggregate_custom_scan TO off;
SELECT string_agg(company_name, ',' ORDER BY company_name)
FROM repro_cccf cccf
JOIN repro_ti ti ON cccf.company_id = ti.company_id;
SET paradedb.enable_aggregate_custom_scan TO on;

-- Cleanup
DROP TABLE repro_cccf CASCADE;
DROP TABLE repro_ti CASCADE;
