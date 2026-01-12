\i common/common_setup.sql

-- Test prepared statements for &&&/|||/###/===/@@@ with pdb.* parameters under generic plans

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, description)
WITH (key_field='id');

SET plan_cache_mode = force_generic_plan;

PREPARE and_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description &&& $1;

EXECUTE and_query('keyboard'::pdb.query);
-- Execute twice to exercise repeated execution under a forced generic plan.
EXECUTE and_query('keyboard'::pdb.query);

PREPARE and_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description &&& $1;

EXECUTE and_boost('keyboard'::pdb.boost(2));

PREPARE and_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description &&& $1;

EXECUTE and_fuzzy('keyboard'::pdb.fuzzy(1));

PREPARE and_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description &&& $1;

EXECUTE and_varchar('keyboard'::varchar);

PREPARE or_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ||| $1;

EXECUTE or_query('keyboard'::pdb.query);

PREPARE or_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ||| $1;

EXECUTE or_boost('keyboard'::pdb.boost(2));

PREPARE or_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ||| $1;

EXECUTE or_fuzzy('keyboard'::pdb.fuzzy(1));

PREPARE or_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ||| $1;

EXECUTE or_varchar('keyboard'::varchar);

PREPARE phrase_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ### $1;

EXECUTE phrase_query('running shoes'::pdb.query);

PREPARE phrase_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ### $1;

EXECUTE phrase_boost('running shoes'::pdb.boost(2));

PREPARE phrase_slop(pdb.slop) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ### $1;

EXECUTE phrase_slop('running shoes'::pdb.slop(2));

PREPARE phrase_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ### $1;

EXECUTE phrase_varchar('running shoes'::varchar);

PREPARE and_array_text(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description &&& $1;

EXECUTE and_array_text(ARRAY['keyboard']::text[]);

PREPARE and_array_varchar(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description &&& $1;

EXECUTE and_array_varchar(ARRAY['keyboard']::varchar[]);

PREPARE or_array_text(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ||| $1;

EXECUTE or_array_text(ARRAY['keyboard']::text[]);

PREPARE or_array_varchar(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ||| $1;

EXECUTE or_array_varchar(ARRAY['keyboard']::varchar[]);

PREPARE phrase_array_text(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ### $1;

EXECUTE phrase_array_text(ARRAY['running', 'shoes']::text[]);

PREPARE phrase_array_varchar(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description ### $1;

EXECUTE phrase_array_varchar(ARRAY['running', 'shoes']::varchar[]);

PREPARE term_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description === $1;

EXECUTE term_query('keyboard'::pdb.query);

PREPARE term_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description === $1;

EXECUTE term_boost('keyboard'::pdb.boost(2));

PREPARE term_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description === $1;

EXECUTE term_fuzzy('keyboard'::pdb.fuzzy(1));

PREPARE term_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description === $1;

EXECUTE term_varchar('keyboard'::varchar);

-- Expected error: === only accepts unclassified pdb.query values.
PREPARE term_parse_rejected(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description === $1;

EXECUTE term_parse_rejected(pdb.parse('keyboard'));

PREPARE parse_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description @@@ $1;

EXECUTE parse_varchar('running shoes'::varchar);

PREPARE prox_clause(pdb.proximityclause) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description @@@ $1;

EXECUTE prox_clause(pdb.prox_clause('running', 1, 'shoes'));

-- Expected error: proximity clauses must be complete (left/right/distance).
PREPARE prox_incomplete(pdb.proximityclause) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items
WHERE description @@@ $1;

EXECUTE prox_incomplete(pdb.prox_term('running'));

DEALLOCATE and_query;
DEALLOCATE and_boost;
DEALLOCATE and_fuzzy;
DEALLOCATE and_varchar;
DEALLOCATE or_query;
DEALLOCATE or_boost;
DEALLOCATE or_fuzzy;
DEALLOCATE or_varchar;
DEALLOCATE phrase_query;
DEALLOCATE phrase_boost;
DEALLOCATE phrase_slop;
DEALLOCATE phrase_varchar;
DEALLOCATE and_array_text;
DEALLOCATE and_array_varchar;
DEALLOCATE or_array_text;
DEALLOCATE or_array_varchar;
DEALLOCATE phrase_array_text;
DEALLOCATE phrase_array_varchar;
DEALLOCATE term_query;
DEALLOCATE term_boost;
DEALLOCATE term_fuzzy;
DEALLOCATE term_varchar;
DEALLOCATE term_parse_rejected;
DEALLOCATE parse_varchar;
DEALLOCATE prox_clause;
DEALLOCATE prox_incomplete;

RESET plan_cache_mode;

DROP TABLE mock_items;

\i common/common_cleanup.sql
