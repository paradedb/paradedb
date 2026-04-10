-- Regression test for https://github.com/paradedb/paradedb/issues/4719
--
-- JoinScan used to fail with:
--   ERROR:  Failed to build DataFusion logical plan: Internal("Missing right join-key column")
-- when a query combined `NOT IN (SELECT ...)` with a
-- `(col IS NULL OR col IN (SELECT ...))` pattern on the same outer relation.
--
-- Each inner subplan is planned with its own PlannerInfo and numbers its
-- range table starting from 1, so the inner relations collide with the outer
-- relation's RTI. collect_required_fields used a flat `(rti, attno)` lookup
-- that could match the wrong source, leaving the right-side join column
-- unprojected — and execution later failed when resolving the join key.

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS issue_4719_people CASCADE;
DROP TABLE IF EXISTS issue_4719_experiences CASCADE;
DROP TABLE IF EXISTS issue_4719_companies CASCADE;

CREATE TABLE issue_4719_people (
    id integer PRIMARY KEY,
    company_id integer,
    body text
);

CREATE TABLE issue_4719_experiences (
    id integer PRIMARY KEY,
    person_id integer,
    company_id integer,
    body text
);

CREATE TABLE issue_4719_companies (
    id integer PRIMARY KEY,
    body text
);

INSERT INTO issue_4719_people (id, company_id, body) VALUES
    (1, 10,   'hit'),
    (2, 20,   'hit'),
    (3, 30,   'hit'),
    (4, NULL, 'hit'),
    (5, 99,   'hit');

INSERT INTO issue_4719_experiences (id, person_id, company_id, body) VALUES
    (1, 2, 10, 'exp'),
    (2, 5, 20, 'exp'),
    (3, 3, 50, 'exp');

INSERT INTO issue_4719_companies (id, body) VALUES
    (10, 'co'), (20, 'co'), (30, 'co');

CREATE INDEX issue_4719_people_idx
    ON issue_4719_people
    USING bm25 (id, company_id, body)
    WITH (
        key_field = 'id',
        numeric_fields = '{"company_id": {"fast": true}}',
        text_fields = '{"body": {"fast": true}}'
    );

CREATE INDEX issue_4719_experiences_idx
    ON issue_4719_experiences
    USING bm25 (id, person_id, company_id, body)
    WITH (
        key_field = 'id',
        numeric_fields = '{"person_id": {"fast": true}, "company_id": {"fast": true}}',
        text_fields = '{"body": {"fast": true}}'
    );

CREATE INDEX issue_4719_companies_idx
    ON issue_4719_companies
    USING bm25 (id, body)
    WITH (
        key_field = 'id',
        text_fields = '{"body": {"fast": true}}'
    );

ANALYZE issue_4719_people;
ANALYZE issue_4719_experiences;
ANALYZE issue_4719_companies;

SET paradedb.enable_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

-- The failing query: NOT IN (SubPlan) + (IS NULL OR IN (SubPlan)).
-- Before the fix, this errored during DataFusion logical plan building.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id
FROM issue_4719_people p
WHERE p.id NOT IN (
    SELECT x.person_id
    FROM issue_4719_experiences x
    WHERE x.company_id IN (10, 20, 50)
)
  AND (p.company_id IS NULL
       OR p.company_id IN (SELECT c.id FROM issue_4719_companies c))
ORDER BY p.id DESC
LIMIT 26;

SELECT p.id
FROM issue_4719_people p
WHERE p.id NOT IN (
    SELECT x.person_id
    FROM issue_4719_experiences x
    WHERE x.company_id IN (10, 20, 50)
)
  AND (p.company_id IS NULL
       OR p.company_id IN (SELECT c.id FROM issue_4719_companies c))
ORDER BY p.id DESC
LIMIT 26;

-- Same query with JoinScan disabled, as a correctness cross-check.
SET paradedb.enable_join_custom_scan TO off;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id
FROM issue_4719_people p
WHERE p.id NOT IN (
    SELECT x.person_id
    FROM issue_4719_experiences x
    WHERE x.company_id IN (10, 20, 50)
)
  AND (p.company_id IS NULL
       OR p.company_id IN (SELECT c.id FROM issue_4719_companies c))
ORDER BY p.id DESC
LIMIT 26;

SELECT p.id
FROM issue_4719_people p
WHERE p.id NOT IN (
    SELECT x.person_id
    FROM issue_4719_experiences x
    WHERE x.company_id IN (10, 20, 50)
)
  AND (p.company_id IS NULL
       OR p.company_id IN (SELECT c.id FROM issue_4719_companies c))
ORDER BY p.id DESC
LIMIT 26;

RESET paradedb.enable_join_custom_scan;

DROP TABLE issue_4719_people CASCADE;
DROP TABLE issue_4719_experiences CASCADE;
DROP TABLE issue_4719_companies CASCADE;
