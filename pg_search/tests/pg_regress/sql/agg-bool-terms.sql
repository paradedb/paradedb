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
WITH (
    key_field = 'id',
    text_fields    = '{"body": {}, "category": {"fast": true}}',
    boolean_fields = '{"has_attachment": {"fast": true}}'
);

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

-- Cleanup
DROP TABLE docs CASCADE;
