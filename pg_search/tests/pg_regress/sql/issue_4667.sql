CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;
SET paradedb.enable_join_custom_scan = on;

DROP TABLE IF EXISTS people CASCADE;
DROP TABLE IF EXISTS companies CASCADE;

CREATE TABLE people (
    id bigint NOT NULL,
    company_id bigint,
    full_name text,
    linkedin_followers integer,
    seniority_slug text
);

CREATE TABLE companies (
    id bigint NOT NULL
);

INSERT INTO companies (id) VALUES
    (100),
    (200);

INSERT INTO people (id, company_id, full_name, linkedin_followers, seniority_slug) VALUES
    (1, 100, 'Alice Director', 5000, 'director'),
    (2, NULL, 'Bob Manager', 1200, 'manager'),
    (3, 200, 'Carol Manager', 800, 'manager'),
    (4, 100, 'Dan Staff', 300, 'staff'),
    (5, 999, 'Eve Orphan', 100, 'manager');

CREATE INDEX people_search_idx ON people USING paradedb (id, ((full_name)::pdb.literal_normalized), linkedin_followers, ((seniority_slug)::pdb.literal_normalized), company_id) WITH (key_field=id);
CREATE INDEX companies_search_idx ON companies USING paradedb (id) WITH (key_field=id);
-- The query uses `SELECT DISTINCT` with derived expressions (`p.full_name IS NULL`,
-- `p.linkedin_followers IS NULL`, `p.seniority_slug IS NULL`).
-- JoinScan only supports pushing down `DISTINCT` over plain indexed columns, so it
-- cannot evaluate DISTINCT on derived expressions; PostgreSQL evaluates them in
-- an upper plan node instead.
-- Because DISTINCT cannot be evaluated inside JoinScan, pushing down the `LIMIT 26`
-- into JoinScan is unsound: truncating rows before upper deduplication could return
-- fewer distinct rows than requested.
-- Without LIMIT pushdown, a top-level JoinScan cannot perform Top-K and would force
-- an unbounded scan followed by a full sort in PostgreSQL, losing its primary
-- performance advantage. JoinScan therefore declines and falls back to PostgreSQL.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT
    p.id AS id,
    p.full_name AS "nameSort",
    p.full_name IS NULL AS "nameIsNull",
    p.linkedin_followers AS "followersSort",
    p.linkedin_followers IS NULL AS "followersIsNull",
    p.seniority_slug AS "senioritySort",
    p.seniority_slug IS NULL AS "seniorityIsNull"
FROM people AS p
WHERE p.id @@@ pdb.all()
AND p.seniority_slug IN ('manager', 'director')
AND (p.company_id IS NULL OR p.company_id IN (SELECT c.id FROM companies AS c))
ORDER BY id DESC
LIMIT 26;

SELECT DISTINCT
    p.id AS id,
    p.full_name AS "nameSort",
    p.full_name IS NULL AS "nameIsNull",
    p.linkedin_followers AS "followersSort",
    p.linkedin_followers IS NULL AS "followersIsNull",
    p.seniority_slug AS "senioritySort",
    p.seniority_slug IS NULL AS "seniorityIsNull"
FROM people AS p
WHERE p.id @@@ pdb.all()
AND p.seniority_slug IN ('manager', 'director')
AND (p.company_id IS NULL OR p.company_id IN (SELECT c.id FROM companies AS c))
ORDER BY id DESC
LIMIT 26;

-- DISTINCT without ORDER BY + SubPlan: exercises sortClause.is_null() path
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT DISTINCT
    p.id AS id,
    p.full_name IS NULL AS "nameIsNull"
FROM people AS p
WHERE p.id @@@ pdb.all()
AND p.seniority_slug IN ('manager', 'director')
AND (p.company_id IS NULL OR p.company_id IN (SELECT c.id FROM companies AS c));

SELECT DISTINCT
    p.id AS id,
    p.full_name IS NULL AS "nameIsNull"
FROM people AS p
WHERE p.id @@@ pdb.all()
AND p.seniority_slug IN ('manager', 'director')
AND (p.company_id IS NULL OR p.company_id IN (SELECT c.id FROM companies AS c));

DROP TABLE people CASCADE;
DROP TABLE companies CASCADE;
RESET paradedb.enable_join_custom_scan;
