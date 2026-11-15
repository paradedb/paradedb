-- Show actionable diagnostics only when field resolution or collection fails.
\set VERBOSITY default
SET max_parallel_workers_per_gather = 0;
CREATE TABLE field_requirements (
    id int PRIMARY KEY,
    description text,
    omitted text,
    metadata jsonb
);
INSERT INTO field_requirements VALUES (1, 'Running shoes', 'shoes', '{"color":"Blue"}');
CREATE INDEX field_requirements_idx ON field_requirements
USING paradedb (id, description, metadata);

-- Typed queries can execute without inferring a field from the left-hand side.
SET paradedb.planner_warnings = off;
SELECT count(*) AS empty_matches FROM field_requirements
WHERE omitted @@@ paradedb.empty();
SELECT count(*) AS all_matches FROM field_requirements
WHERE omitted @@@ paradedb.with_index('field_requirements_idx', paradedb.all());
-- Field-based searches still fail when performance warnings are disabled.
SELECT id FROM field_requirements WHERE omitted ||| 'shoes';
RESET paradedb.planner_warnings;

SELECT id FROM field_requirements WHERE omitted ||| 'shoes';
SELECT id FROM field_requirements WHERE omitted &&& ARRAY['shoes'];
SELECT id FROM field_requirements WHERE omitted === 'shoes';
SELECT id FROM field_requirements WHERE omitted ### 'shoes';
SELECT id FROM field_requirements WHERE omitted @@@ 'shoes';

SET plan_cache_mode = force_generic_plan;
PREPARE omitted_search(text) AS SELECT id FROM field_requirements WHERE omitted ||| $1;
EXECUTE omitted_search('shoes');
DEALLOCATE omitted_search;
RESET plan_cache_mode;

-- These fields exist, but default text and JSON fields lack columnar storage.
SELECT pdb.agg('{"terms":{"field":"description"}}')
FROM field_requirements WHERE id @@@ pdb.all();
SELECT pdb.agg('{"terms":{"field":"metadata.color"}}')
FROM field_requirements WHERE id @@@ pdb.all();
SELECT pdb.agg('{"terms":{"field":"id"},"aggs":{"colors":{"terms":{"field":"metadata.color"}}}}')
FROM field_requirements WHERE id @@@ pdb.all();
SELECT pdb.agg('{"terms":{"field":"description"}}', 'raw'::text)
FROM field_requirements WHERE id @@@ pdb.all();

-- A Top K window aggregate uses the other segment collection path.
SELECT id, pdb.agg('{"terms":{"field":"metadata.color"}}') OVER ()
FROM field_requirements WHERE description ||| 'shoes'
ORDER BY paradedb.score(id) DESC LIMIT 1;

-- An unrelated collection failure must retain the underlying error.
\set VERBOSITY terse
SELECT pdb.agg('{"histogram":{"field":"id","interval":0}}')
FROM field_requirements WHERE id @@@ pdb.all();
\set VERBOSITY default

-- Both explicit columnar storage and literal's default support aggregation.
DROP INDEX field_requirements_idx;
CREATE INDEX field_requirements_idx ON field_requirements
USING paradedb (id, (description::pdb.unicode_words('columnar=true')), (metadata::pdb.literal));
SELECT id FROM field_requirements WHERE description ||| 'shoes';
SELECT pdb.agg('{"terms":{"field":"description"}}')->'buckets'
FROM field_requirements WHERE id @@@ pdb.all();
SELECT pdb.agg('{"terms":{"field":"metadata.color"}}')->'buckets'
FROM field_requirements WHERE id @@@ pdb.all();
SELECT id, pdb.agg('{"terms":{"field":"metadata.color"}}') OVER ()
FROM field_requirements WHERE description ||| 'shoes'
ORDER BY paradedb.score(id) DESC LIMIT 1;

DROP TABLE field_requirements;
RESET max_parallel_workers_per_gather;
