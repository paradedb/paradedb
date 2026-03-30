-- Regression test for pdb.agg() on boolean fields and single-argument overload.
--
-- Bug 1: pdb.agg(jsonb) terms aggregation on bool columns failed with:
--   "Missing value U64(2) for field ... is not supported for column type Bool"
--
-- Bug 2: pdb.agg(jsonb) single-argument overload was missing after
--   ALTER EXTENSION UPDATE through the 0.21.15→0.21.16 migration path.

CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan TO on;

DROP TABLE IF EXISTS docs CASCADE;

CREATE TABLE docs (
    id SERIAL PRIMARY KEY,
    body TEXT,
    category TEXT,
    has_attachment BOOLEAN NOT NULL DEFAULT false
);

INSERT INTO docs (body, category, has_attachment) VALUES
    ('quarterly report draft',   'finance',     true),
    ('annual budget summary',    'finance',     false),
    ('project kickoff notes',    'engineering', true),
    ('sprint retrospective',     'engineering', true),
    ('company policy update',    'hr',          false),
    ('onboarding checklist',     'hr',          false),
    ('architecture design doc',  'engineering', false);

CREATE INDEX docs_idx ON docs
USING bm25 (id, body, category, has_attachment)
WITH (key_field = 'id');

-- Test 1: terms aggregation on a boolean field using single-argument pdb.agg(jsonb)
-- This exercises both fixes: the single-arg overload must exist, and bool terms must not crash.
SELECT pdb.agg('{"terms": {"field": "has_attachment", "size": 10}}'::jsonb)
FROM docs
WHERE body @@@ pdb.all();

-- Test 2: terms aggregation on a boolean field with GROUP BY
SELECT category, pdb.agg('{"terms": {"field": "has_attachment"}}'::jsonb)
FROM docs
WHERE body @@@ pdb.all()
GROUP BY category
ORDER BY category;

-- Test 3: two-argument overload on a boolean field (solve_mvcc = true)
SELECT pdb.agg('{"terms": {"field": "has_attachment"}}'::jsonb, true)
FROM docs
WHERE body @@@ pdb.all();

-- Test 4: two-argument overload on a boolean field (solve_mvcc = false)
SELECT pdb.agg('{"terms": {"field": "has_attachment"}}'::jsonb, false)
FROM docs
WHERE body @@@ pdb.all();

-- Test 5: NULL bool values should form their own group (standard SQL behavior)
DROP TABLE IF EXISTS docs_nullable CASCADE;

CREATE TABLE docs_nullable (
    id SERIAL PRIMARY KEY,
    body TEXT,
    has_flag BOOLEAN
);

INSERT INTO docs_nullable (body, has_flag) VALUES
    ('doc with true',   true),
    ('doc with false',  false),
    ('doc with null 1', NULL),
    ('doc with null 2', NULL),
    ('another true',    true);

CREATE INDEX docs_nullable_idx ON docs_nullable
USING bm25 (id, body, has_flag)
WITH (key_field = 'id');

-- 5a: EXPLAIN to confirm aggregate custom scan is used
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT has_flag, COUNT(*)
FROM docs_nullable
WHERE body @@@ pdb.all()
GROUP BY has_flag
ORDER BY has_flag;

-- 5b: GROUP BY nullable bool — expect three groups: true, false, NULL
SELECT has_flag, COUNT(*)
FROM docs_nullable
WHERE body @@@ pdb.all()
GROUP BY has_flag
ORDER BY has_flag;

-- 5c: EXPLAIN to confirm aggregate custom scan for pdb.agg on nullable bool
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "has_flag"}}'::jsonb)
FROM docs_nullable
WHERE body @@@ pdb.all();

-- 5d: Verify pdb.agg terms on nullable bool includes all docs
SELECT pdb.agg('{"terms": {"field": "has_flag"}}'::jsonb)
FROM docs_nullable
WHERE body @@@ pdb.all();

DROP TABLE docs_nullable CASCADE;

-- Cleanup
DROP TABLE docs CASCADE;
