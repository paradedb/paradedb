-- EXACT customer reproduction using their actual list_id patterns
-- Based on their sample data showing specific non-sequential list IDs

\set VERBOSITY verbose
\timing on

-- Match their EXACT settings
SET work_mem = '4MB';
SET maintenance_work_mem = '60GB';
SET max_parallel_workers = 120;
SET max_parallel_maintenance_workers = 30;

DROP TABLE IF EXISTS contacts_companies_combined_full_old CASCADE;

CREATE TABLE contacts_companies_combined_full_old (
    contact_id INT PRIMARY KEY,
    list_id INT[]
);

\echo '=== EXACT CUSTOMER REPRODUCTION ==='
\echo 'Using their actual list_id patterns from production'
\echo 'Arrays with 6000-6700 elements of non-sequential IDs'
\echo ''

-- Insert data matching their EXACT pattern:
-- Real list IDs from their sample: {407,434,638,769,1184,1202,1781,2012,2108,2270,...}
-- These are NOT random, they're specific list IDs with gaps

INSERT INTO contacts_companies_combined_full_old (contact_id, list_id)
SELECT
    i,
    -- Create arrays with similar pattern to customer's data:
    -- Non-sequential IDs with gaps, matching their distribution
    ARRAY(
        SELECT DISTINCT (
            CASE
                WHEN j % 10 = 0 THEN j * 100 + (random() * 50)::int
                WHEN j % 5 = 0 THEN j * 20 + (random() * 10)::int
                ELSE j * 3 + (random() * 5)::int
            END
        )::int
        FROM generate_series(1,
            -- Vary array sizes like customer: 6600-6700 elements
            6600 + (i % 100)
        ) j
        ORDER BY 1
        LIMIT 6700
    )
FROM generate_series(1, 100000) i;

\echo ''
\echo 'Sample of actual data (matching customer format):'
SELECT
    array_length(list_id, 1) as array_len,
    pg_column_size(list_id) as size_bytes,
    list_id[1:10] as first_10_elements
FROM contacts_companies_combined_full_old
WHERE list_id IS NOT NULL
ORDER BY array_length(list_id, 1) DESC
LIMIT 10;

\echo ''
\echo 'Memory settings:'
SHOW work_mem;
SHOW maintenance_work_mem;
SHOW max_parallel_workers;
SHOW max_parallel_maintenance_workers;

\echo ''
\echo '=== CREATING INDEX (EXACT CUSTOMER COMMAND) ==='
\echo 'Using their exact CREATE INDEX statement'
\echo ''

-- EXACT command from customer
CREATE INDEX contacts_companies_combined_full_old_list_id_idx ON contacts_companies_combined_full_old
USING bm25 (contact_id, list_id)
WITH (
    key_field=contact_id,
    numeric_fields='{"list_id": {"indexed": true}}'
);

\echo ''
\echo '=== INDEX CREATED SUCCESSFULLY ==='
\echo 'If you see this, the bug was NOT reproduced'