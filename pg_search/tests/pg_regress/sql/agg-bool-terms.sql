-- Regression test for pdb.agg() on boolean fields and single-argument overload.

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
SELECT pdb.agg('{"terms": {"field": "has_attachment", "size": 10}}'::jsonb)
FROM docs
WHERE body @@@ pdb.all();

-- Test 2: two-argument overload on a boolean field (solve_mvcc = true)
SELECT pdb.agg('{"terms": {"field": "has_attachment"}}'::jsonb, true)
FROM docs
WHERE body @@@ pdb.all();

-- Test 3: two-argument overload on a boolean field (solve_mvcc = false)
SELECT pdb.agg('{"terms": {"field": "has_attachment"}}'::jsonb, false)
FROM docs
WHERE body @@@ pdb.all();

-- Test 4: NULL bool values should form their own group (standard SQL behavior)
DROP TABLE IF EXISTS docs_nullable CASCADE;

CREATE TABLE docs_nullable (
    id SERIAL PRIMARY KEY,
    body TEXT,
    category TEXT NOT NULL,
    has_flag BOOLEAN
);

INSERT INTO docs_nullable (body, category, has_flag) VALUES
    ('doc with true',   'x', true),
    ('doc with false',  'y', false),
    ('doc with null 1', 'x', NULL),
    ('doc with null 2', 'y', NULL),
    ('another true',    'x', true);

CREATE INDEX docs_nullable_idx ON docs_nullable
USING bm25 (id, body, (category::pdb.unicode_words('columnar=true')), has_flag)
WITH (key_field = 'id');

-- 4a: EXPLAIN to confirm aggregate custom scan is used
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT has_flag, COUNT(*)
FROM docs_nullable
WHERE body @@@ pdb.all()
GROUP BY has_flag
ORDER BY has_flag;

-- 4b: GROUP BY nullable bool — expect three groups: true, false, NULL
SELECT has_flag, COUNT(*)
FROM docs_nullable
WHERE body @@@ pdb.all()
GROUP BY has_flag
ORDER BY has_flag;

-- 4c: EXPLAIN to confirm aggregate custom scan for pdb.agg on nullable bool
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT pdb.agg('{"terms": {"field": "has_flag"}}'::jsonb)
FROM docs_nullable
WHERE body @@@ pdb.all();

-- 4d: Verify pdb.agg terms on nullable bool includes all docs and the null sentinel is scrubbed 
-- when not using custom scan
SET paradedb.enable_aggregate_custom_scan = off;
SELECT pdb.agg('{"terms": {"field": "has_flag"}}'::jsonb)
FROM docs_nullable
WHERE body @@@ pdb.all();

-- 4e: Verify pdb.agg terms on nullable bool includes all docs and the null sentinel is scrubbed
-- when using custom scan
SET paradedb.enable_aggregate_custom_scan = on;
SELECT pdb.agg('{"terms": {"field": "has_flag"}}'::jsonb)
FROM docs_nullable
WHERE body @@@ pdb.all();

-- Test 5: Nested terms sub-aggregation over a nullable bool — sentinel must be
-- scrubbed at every level of the bucket tree, not just the top.
SELECT pdb.agg('{"terms": {"field": "category"}, "aggs": {"by_flag": {"terms": {"field": "has_flag"}}}}'::jsonb)
FROM docs_nullable
WHERE body @@@ pdb.all();

DROP TABLE docs_nullable CASCADE;

-- Cleanup
DROP TABLE docs CASCADE;
