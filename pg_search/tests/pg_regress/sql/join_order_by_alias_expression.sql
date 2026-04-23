-- =====================================================================
-- Issue #4850: JoinScan — ORDER BY unaliased expression when a sibling
-- aliased entry claims the same attno.
-- =====================================================================
-- When a BM25 index has two entries for the same column — one aliased,
-- one as an unaliased expression — JoinScan's ORDER BY projection must
-- not be short-circuited by the aliased sibling's attno. Verify that
-- the query succeeds and that it goes through the ParadeDB Join Scan.

SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE contacts (
    contact_id bigint PRIMARY KEY,
    company_id bigint,
    company_name character varying
);

CREATE TABLE tech_installs (
    unique_id bigint PRIMARY KEY,
    company_id bigint,
    technology_name character varying
);

INSERT INTO contacts (contact_id, company_id, company_name) VALUES (1, 1, 'amazon');
INSERT INTO tech_installs (unique_id, company_id, technology_name) VALUES (1, 1, 'java');

-- Index with BOTH an unaliased lower() expression AND an aliased
-- expression on the same column. This triggers the attno dedup
-- false-positive in ORDER BY field collection.
CREATE INDEX contacts_idx ON contacts
USING bm25 (
    contact_id,
    company_id,
    (lower(company_name::text)::pdb.literal_normalized('ascii_folding=true')),
    (company_name::pdb.simple('alias=company_name_words', 'ascii_folding=true', 'columnar=true'))
) WITH (key_field=contact_id, numeric_fields='{"company_id": {"fast": true}}');

CREATE INDEX tech_installs_idx ON tech_installs
USING bm25 (
    unique_id,
    company_id,
    technology_name
) WITH (key_field=unique_id, numeric_fields='{"company_id": {"fast": true}}');

SET paradedb.enable_join_custom_scan = on;

-- =====================================================================
-- Test 1: ORDER BY lower(company_name) — previously errored with
-- "FieldNotFound: company_name".
-- =====================================================================
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT cccf.*
FROM contacts cccf
JOIN tech_installs ti ON cccf.company_id = ti.company_id
WHERE technology_name @@@ 'java'
ORDER BY lower(company_name) ASC, cccf.contact_id
LIMIT 10;

SELECT cccf.*
FROM contacts cccf
JOIN tech_installs ti ON cccf.company_id = ti.company_id
WHERE technology_name @@@ 'java'
ORDER BY lower(company_name) ASC, cccf.contact_id
LIMIT 10;

-- Parity: same query with custom scan off (native Postgres).
SET paradedb.enable_join_custom_scan = off;
SELECT cccf.*
FROM contacts cccf
JOIN tech_installs ti ON cccf.company_id = ti.company_id
WHERE technology_name @@@ 'java'
ORDER BY lower(company_name) ASC, cccf.contact_id
LIMIT 10;
SET paradedb.enable_join_custom_scan = on;

-- Cleanup
DROP TABLE contacts CASCADE;
DROP TABLE tech_installs CASCADE;
