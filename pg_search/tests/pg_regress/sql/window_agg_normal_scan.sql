\i common/common_setup.sql

-- Test: pdb.agg() OVER() window aggregate support
--
-- This tests that pdb.agg() OVER() works correctly regardless of which
-- execution method the planner chooses (TopK or NormalScan).
--
-- Customer-reported issue: pdb.agg() OVER() worked on small tables where
-- the planner picked TopKScanExecState, but failed on larger tables where
-- the planner picked NormalScanExecState and placed a WindowAgg node above
-- the Custom Scan.

-- Create table mimicking customer schema
CREATE TABLE nonprofits_test (
    id SERIAL PRIMARY KEY,
    legal_name TEXT NOT NULL,
    mission TEXT,
    keywords TEXT,
    city TEXT,
    state TEXT,
    is_irs_active BOOLEAN DEFAULT true,
    deleted_at TIMESTAMPTZ
);

-- Insert enough rows for meaningful aggregation results
INSERT INTO nonprofits_test (legal_name, mission, keywords, city, state, is_irs_active, deleted_at)
SELECT
    'Org ' || i || CASE WHEN i % 5 = 0 THEN ' Education Fund' ELSE '' END,
    CASE
        WHEN i % 7 = 0 THEN 'education and literacy programs for communities'
        WHEN i % 7 = 1 THEN 'support education programs in underserved areas'
        WHEN i % 7 = 2 THEN 'animal welfare and shelter services'
        WHEN i % 7 = 3 THEN 'community health services and wellness'
        WHEN i % 7 = 4 THEN 'education outreach and mentoring'
        WHEN i % 7 = 5 THEN 'environmental conservation efforts'
        ELSE 'arts and cultural preservation'
    END,
    CASE
        WHEN i % 3 = 0 THEN 'education learning school'
        WHEN i % 3 = 1 THEN 'health wellness community'
        ELSE 'environment conservation nature'
    END,
    CASE WHEN i % 4 = 0 THEN 'Boston' WHEN i % 4 = 1 THEN 'New York' WHEN i % 4 = 2 THEN 'Portland' ELSE 'San Francisco' END,
    CASE WHEN i % 4 = 0 THEN 'MA' WHEN i % 4 = 1 THEN 'NY' WHEN i % 4 = 2 THEN 'OR' ELSE 'CA' END,
    (i % 10 != 0),
    CASE WHEN i % 20 = 0 THEN NOW() ELSE NULL END
FROM generate_series(1, 10000) AS i;

ANALYZE nonprofits_test;

-- Create BM25 index with partial index predicate
CREATE INDEX nonprofits_test_idx ON nonprofits_test
USING bm25 (
    id,
    legal_name, mission, keywords,
    (city::pdb.literal), (state::pdb.literal),
    is_irs_active
)
WITH (key_field = 'id')
WHERE deleted_at IS NULL;

SET paradedb.enable_aggregate_custom_scan TO on;

-- Test 1: pdb.agg() OVER() with ORDER BY + LIMIT (TopK path - always works)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT
    n.id, n.legal_name, pdb.score(n.id) as score,
    pdb.agg('{"terms": {"field": "state", "size": 10}}', false) OVER () AS state_facets
FROM nonprofits_test n
WHERE n.id @@@ pdb.parse('education', lenient => true)
  AND n.deleted_at IS NULL
  AND n.is_irs_active = true
ORDER BY pdb.score(n.id) DESC
LIMIT 5;

SELECT
    n.id, n.legal_name, pdb.score(n.id) as score,
    pdb.agg('{"terms": {"field": "state", "size": 10}}', false) OVER () AS state_facets
FROM nonprofits_test n
WHERE n.id @@@ pdb.parse('education', lenient => true)
  AND n.deleted_at IS NULL
  AND n.is_irs_active = true
ORDER BY pdb.score(n.id) DESC
LIMIT 5;

-- Test 2: pdb.agg() OVER() WITHOUT ORDER BY (NormalScan path)
-- This is the customer-reported failing scenario. Without ORDER BY,
-- the planner cannot use TopK and must fall back to NormalScan.
-- Previously this errored; now it should compute aggregates via NormalScan.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT
    n.id, n.legal_name,
    pdb.agg('{"terms": {"field": "state", "size": 10}}', false) OVER () AS state_facets
FROM nonprofits_test n
WHERE n.id @@@ pdb.parse('education', lenient => true)
  AND n.deleted_at IS NULL
  AND n.is_irs_active = true
LIMIT 5;

SELECT
    n.id, n.legal_name,
    pdb.agg('{"terms": {"field": "state", "size": 10}}', false) OVER () AS state_facets
FROM nonprofits_test n
WHERE n.id @@@ pdb.parse('education', lenient => true)
  AND n.deleted_at IS NULL
  AND n.is_irs_active = true
LIMIT 5;

-- Test 3: Standalone pdb.agg() aggregate (non-window, should always work)
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT pdb.agg('{"terms":{"field":"state","size":10}}')
FROM nonprofits_test n
WHERE n.id @@@ pdb.parse('education', lenient => true)
  AND n.deleted_at IS NULL;

SELECT pdb.agg('{"terms":{"field":"state","size":10}}')
FROM nonprofits_test n
WHERE n.id @@@ pdb.parse('education', lenient => true)
  AND n.deleted_at IS NULL;

-- Cleanup
DROP TABLE nonprofits_test CASCADE;
