-- Tests the `visibility` argument of pdb.agg() and paradedb.aggregate(): the
-- 'transaction' / 'raw' / 'threshold' modes, the paradedb.visibility_threshold
-- GUC, the legacy solve_mvcc spellings, and the errors for a conflicting or
-- unrecognized setting.
\echo 'Test: aggregate visibility modes'

CREATE EXTENSION IF NOT EXISTS pg_search;

DROP TABLE IF EXISTS vis_test;
CREATE TABLE vis_test (
    id serial8,
    description text NOT NULL,
    value int NOT NULL
) WITH (autovacuum_enabled = off);

INSERT INTO vis_test (description, value)
SELECT 'shoes running test', x FROM generate_series(1, 1000) x;

CREATE INDEX idx_vis_test ON vis_test
USING bm25 (id, description, value)
WITH (key_field = 'id', numeric_fields = '{"value": {"fast": true}}');

-- Delete a tenth of the rows without vacuuming, so the index still holds
-- entries for 1000 rows while only 900 are visible. Every assertion below turns
-- on that gap: 900 means the visibility check ran, 1000 means it did not.
DELETE FROM vis_test WHERE id % 10 = 0;

--
-- pdb.agg(): the three modes
--

-- Default: transaction visibility, so 900.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb)
FROM vis_test WHERE description @@@ 'running';

-- Explicit 'transaction' matches the default.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'transaction')
FROM vis_test WHERE description @@@ 'running';

-- 'raw' counts the dead index entries too, so 1000.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'raw')
FROM vis_test WHERE description @@@ 'running';

-- An explicit ::text cast resolves to the same overload as the bare literal.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'raw'::text)
FROM vis_test WHERE description @@@ 'running';

--
-- pdb.agg(): 'threshold' against the GUC
--

-- The GUC's default is 10000, comfortably above this query's ~1000 matching
-- rows, so the checks run.
SHOW paradedb.visibility_threshold;
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'threshold')
FROM vis_test WHERE description @@@ 'running';

-- No row count is below 0, so this forces the raw side of the branch.
SET paradedb.visibility_threshold = 0;
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'threshold')
FROM vis_test WHERE description @@@ 'running';

-- Well above the match estimate, so back to checking.
SET paradedb.visibility_threshold = 1000000;
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'threshold')
FROM vis_test WHERE description @@@ 'running';

-- 'threshold' leaves the pinned modes alone.
SET paradedb.visibility_threshold = 0;
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'transaction')
FROM vis_test WHERE description @@@ 'running';
SET paradedb.visibility_threshold = 1000000;
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'raw')
FROM vis_test WHERE description @@@ 'running';
RESET paradedb.visibility_threshold;

-- 'estimated' is an accepted spelling of 'threshold'.
SET paradedb.visibility_threshold = 0;
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'estimated')
FROM vis_test WHERE description @@@ 'running';
RESET paradedb.visibility_threshold;

--
-- pdb.agg(): the legacy solve_mvcc spellings
--

-- The bool overload still resolves and still means what it did.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, true)
FROM vis_test WHERE description @@@ 'running';
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, false)
FROM vis_test WHERE description @@@ 'running';

-- The old string spellings are accepted as aliases.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'always')
FROM vis_test WHERE description @@@ 'running';
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'never')
FROM vis_test WHERE description @@@ 'running';
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'enabled')
FROM vis_test WHERE description @@@ 'running';
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'disabled')
FROM vis_test WHERE description @@@ 'running';

-- Parsing is case-insensitive and tolerates surrounding whitespace.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, '  RAW ')
FROM vis_test WHERE description @@@ 'running';

--
-- pdb.agg(): errors
--

-- An unrecognized mode is an error rather than a silent fallback.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'nonsense')
FROM vis_test WHERE description @@@ 'running';

-- Two pdb.agg() calls in one query must agree.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'transaction'),
       pdb.agg('{"sum": {"field": "value"}}'::jsonb, 'raw')
FROM vis_test WHERE description @@@ 'running';

-- Including when one of them omits the argument: omitting selects
-- 'transaction', which conflicts with an explicit 'raw'.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb),
       pdb.agg('{"sum": {"field": "value"}}'::jsonb, 'raw')
FROM vis_test WHERE description @@@ 'running';

-- 'threshold' is a distinct mode, so it conflicts with a pinned one too.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'threshold'),
       pdb.agg('{"sum": {"field": "value"}}'::jsonb, 'raw')
FROM vis_test WHERE description @@@ 'running';

-- Agreeing calls are fine, whatever they agree on.
SELECT pdb.agg('{"value_count": {"field": "id"}}'::jsonb, 'raw'),
       pdb.agg('{"sum": {"field": "value"}}'::jsonb, 'raw')
FROM vis_test WHERE description @@@ 'running';

--
-- paradedb.aggregate(): the UDF
--

SELECT * FROM paradedb.aggregate(
    index => 'idx_vis_test',
    query => paradedb.match('description', 'running'),
    agg => '{"count": {"value_count": {"field": "id"}}}'
);

SELECT * FROM paradedb.aggregate(
    index => 'idx_vis_test',
    query => paradedb.match('description', 'running'),
    agg => '{"count": {"value_count": {"field": "id"}}}',
    visibility => 'raw'
);

SET paradedb.visibility_threshold = 0;
SELECT * FROM paradedb.aggregate(
    index => 'idx_vis_test',
    query => paradedb.match('description', 'running'),
    agg => '{"count": {"value_count": {"field": "id"}}}',
    visibility => 'threshold'
);
RESET paradedb.visibility_threshold;

-- solve_mvcc still works, and now warns.
SELECT * FROM paradedb.aggregate(
    index => 'idx_vis_test',
    query => paradedb.match('description', 'running'),
    agg => '{"count": {"value_count": {"field": "id"}}}',
    solve_mvcc => false
);

-- Positional calls written against the old signature still resolve, because
-- `visibility` was appended after `bucket_limit` rather than replacing
-- `solve_mvcc` in place.
SELECT * FROM paradedb.aggregate(
    'idx_vis_test',
    paradedb.match('description', 'running'),
    '{"count": {"value_count": {"field": "id"}}}',
    false
);

-- Supplying both spellings is an error, not a precedence rule.
SELECT * FROM paradedb.aggregate(
    index => 'idx_vis_test',
    query => paradedb.match('description', 'running'),
    agg => '{"count": {"value_count": {"field": "id"}}}',
    solve_mvcc => false,
    visibility => 'raw'
);

-- An unrecognized mode errors here too.
SELECT * FROM paradedb.aggregate(
    index => 'idx_vis_test',
    query => paradedb.match('description', 'running'),
    agg => '{"count": {"value_count": {"field": "id"}}}',
    visibility => 'nonsense'
);

DROP TABLE vis_test;
