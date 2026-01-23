\i common/common_setup.sql

-- =============================================================================
-- Test Suite: Operator Parameter Support for const_rewrite and exec_rewrite
-- =============================================================================
-- This test verifies that all operators (&&&, |||, ###, ===, @@@) work correctly
-- with various parameter types in both:
--   1. const_rewrite path: Literal values known at plan time
--   2. exec_rewrite path: Parameters in prepared statements (generic plans)
--
-- Type Support Matrix Legend:
--   OK     = Works correctly
--   PANIC  = Accepted but fails at runtime
--   REJECT = Rejected by type check with clear error
--   IGNORED = Accepted but modifier has no effect
-- =============================================================================

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, description)
WITH (key_field='id');

-- =============================================================================
-- SECTION 1: const_rewrite PATH (Literal Values)
-- =============================================================================
-- These tests use literal values directly in SQL, triggering the const_rewrite
-- code path where the value is known at plan time.

-- -----------------------------------------------------------------------------
-- 1.1 &&& Operator (Match Conjunction) - const_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
SELECT '&&& text (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& 'keyboard';

-- OK: varchar
SELECT '&&& varchar (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& 'keyboard'::varchar;

-- OK: text[]
SELECT '&&& text[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& ARRAY['keyboard', 'plastic'];

-- OK: varchar[]
SELECT '&&& varchar[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& ARRAY['keyboard']::varchar[];

-- OK: pdb.query (unclassified string)
SELECT '&&& pdb.query (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& 'keyboard'::pdb.query;

-- OK: pdb.boost
SELECT '&&& pdb.boost (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& 'keyboard'::pdb.boost(2.0);

-- OK: pdb.fuzzy
SELECT '&&& pdb.fuzzy (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& 'keyboard'::pdb.fuzzy(1);

-- OK: pdb.fuzzy with array
SELECT '&&& pdb.fuzzy[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& ARRAY['keyboard']::pdb.fuzzy;

-- REJECT: pdb.slop (operator is not unique - no direct stub for pdb.slop)
SELECT '&&& pdb.slop (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& 'keyboard'::pdb.slop(2);

-- -----------------------------------------------------------------------------
-- 1.2 ||| Operator (Match Disjunction) - const_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
SELECT '||| text (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| 'keyboard';

-- OK: varchar
SELECT '||| varchar (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| 'keyboard'::varchar;

-- OK: text[]
SELECT '||| text[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| ARRAY['keyboard', 'shoes'];

-- OK: varchar[]
SELECT '||| varchar[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| ARRAY['keyboard']::varchar[];

-- OK: pdb.query (unclassified string)
SELECT '||| pdb.query (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| 'keyboard'::pdb.query;

-- OK: pdb.boost
SELECT '||| pdb.boost (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| 'keyboard'::pdb.boost(2.0);

-- OK: pdb.fuzzy
SELECT '||| pdb.fuzzy (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| 'keyboard'::pdb.fuzzy(1);

-- REJECT: pdb.slop (operator is not unique - no direct stub for pdb.slop)
SELECT '||| pdb.slop (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| 'keyboard'::pdb.slop(2);

-- -----------------------------------------------------------------------------
-- 1.3 ### Operator (Phrase) - const_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
SELECT '### text (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### 'running shoes';

-- OK: varchar
SELECT '### varchar (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### 'running shoes'::varchar;

-- OK: text[]
SELECT '### text[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### ARRAY['running', 'shoes'];

-- OK: varchar[]
SELECT '### varchar[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### ARRAY['running', 'shoes']::varchar[];

-- OK: pdb.query (unclassified string)
SELECT '### pdb.query (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### 'running shoes'::pdb.query;

-- OK: pdb.boost
SELECT '### pdb.boost (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### 'running shoes'::pdb.boost(2.0);

-- OK: pdb.slop (phrase with word distance)
SELECT '### pdb.slop (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### 'running shoes'::pdb.slop(2);

-- IGNORED: pdb.fuzzy (accepted but fuzzy_data is ignored by phrase_query)
SELECT '### pdb.fuzzy (const) - fuzzy ignored' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### 'running shoes'::pdb.fuzzy(1);

-- -----------------------------------------------------------------------------
-- 1.4 === Operator (Term) - const_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
SELECT '=== text (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === 'keyboard';

-- OK: varchar
SELECT '=== varchar (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === 'keyboard'::varchar;

-- OK: text[]
SELECT '=== text[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === ARRAY['keyboard', 'plastic'];

-- OK: varchar[]
SELECT '=== varchar[] (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === ARRAY['keyboard']::varchar[];

-- OK: pdb.query (unclassified string)
SELECT '=== pdb.query (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === 'keyboard'::pdb.query;

-- OK: pdb.boost
SELECT '=== pdb.boost (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === 'keyboard'::pdb.boost(2.0);

-- OK: pdb.fuzzy
SELECT '=== pdb.fuzzy (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === 'keyboard'::pdb.fuzzy(1);

-- REJECT: pdb.slop (operator is not unique - no direct stub for pdb.slop)
SELECT '=== pdb.slop (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === 'keyboard'::pdb.slop(2);

-- REJECT: pdb.parse() - already classified query rejected by ===
SELECT '=== pdb.parse (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === pdb.parse('keyboard');

-- -----------------------------------------------------------------------------
-- 1.5 @@@ Operator (Parse/Proximity) - const_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
SELECT '@@@ text (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ 'running shoes';

-- OK: varchar
SELECT '@@@ varchar (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ 'running shoes'::varchar;

-- OK: pdb.query (unclassified string -> ParseWithField)
SELECT '@@@ pdb.query (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ 'running shoes'::pdb.query;

-- OK: pdb.boost (via implicit cast to pdb.query)
SELECT '@@@ pdb.boost (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ 'running shoes'::pdb.boost(2.0);

-- OK: pdb.fuzzy (via implicit cast to pdb.query)
SELECT '@@@ pdb.fuzzy (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ 'running'::pdb.fuzzy(1);

-- OK: pdb.ProximityClause (complete clause)
SELECT '@@@ pdb.ProximityClause (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ pdb.prox_clause('running', 1, 'shoes');

-- OK: pdb.parse (already classified query flows through)
SELECT '@@@ pdb.parse (const)' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ pdb.parse('running OR shoes');

-- REJECT: text[] not supported by @@@
SELECT '@@@ text[] (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ ARRAY['running', 'shoes'];

-- REJECT: varchar[] not supported by @@@
SELECT '@@@ varchar[] (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ ARRAY['running', 'shoes']::varchar[];

-- REJECT: incomplete ProximityClause
SELECT '@@@ incomplete prox (const) - expected error' AS test,
       array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ pdb.prox_term('running');

-- =============================================================================
-- SECTION 2: exec_rewrite PATH (Prepared Statements with Generic Plans)
-- =============================================================================
-- These tests use prepared statements with force_generic_plan, triggering the
-- exec_rewrite code path where only the parameter TYPE is known at plan time.

SET plan_cache_mode = force_generic_plan;

-- -----------------------------------------------------------------------------
-- 2.1 &&& Operator (Match Conjunction) - exec_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
PREPARE exec_and_text(text) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_text('keyboard');

-- OK: varchar
PREPARE exec_and_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_varchar('keyboard'::varchar);

-- OK: text[]
PREPARE exec_and_text_array(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_text_array(ARRAY['keyboard', 'plastic']::text[]);

-- OK: varchar[]
PREPARE exec_and_varchar_array(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_varchar_array(ARRAY['keyboard']::varchar[]);

-- OK: pdb.query
PREPARE exec_and_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_query('keyboard'::pdb.query);
-- Execute twice to verify repeated execution under generic plan
EXECUTE exec_and_query('keyboard'::pdb.query);

-- OK: pdb.boost (via implicit cast to pdb.query)
PREPARE exec_and_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_boost('keyboard'::pdb.boost(2));

-- OK: pdb.fuzzy (via implicit cast to pdb.query)
PREPARE exec_and_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_fuzzy('keyboard'::pdb.fuzzy(1));
EXECUTE exec_and_fuzzy(ARRAY['keyboard']::pdb.fuzzy);

-- REJECT: pdb.slop (operator is not unique - no direct stub for pdb.slop)
PREPARE exec_and_slop(pdb.slop) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description &&& $1;
EXECUTE exec_and_slop('keyboard'::pdb.slop(2));

-- -----------------------------------------------------------------------------
-- 2.2 ||| Operator (Match Disjunction) - exec_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
PREPARE exec_or_text(text) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_text('keyboard');

-- OK: varchar
PREPARE exec_or_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_varchar('keyboard'::varchar);

-- OK: text[]
PREPARE exec_or_text_array(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_text_array(ARRAY['keyboard', 'shoes']::text[]);

-- OK: varchar[]
PREPARE exec_or_varchar_array(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_varchar_array(ARRAY['keyboard']::varchar[]);

-- OK: pdb.query
PREPARE exec_or_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_query('keyboard'::pdb.query);

-- OK: pdb.boost (via implicit cast to pdb.query)
PREPARE exec_or_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_boost('keyboard'::pdb.boost(2));

-- OK: pdb.fuzzy (via implicit cast to pdb.query)
PREPARE exec_or_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_fuzzy('keyboard'::pdb.fuzzy(1));
EXECUTE exec_or_fuzzy(ARRAY['keyboard']::pdb.fuzzy);

-- REJECT: pdb.slop (operator is not unique - no direct stub for pdb.slop)
PREPARE exec_or_slop(pdb.slop) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ||| $1;
EXECUTE exec_or_slop('keyboard'::pdb.slop(2));

-- -----------------------------------------------------------------------------
-- 2.3 ### Operator (Phrase) - exec_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
PREPARE exec_phrase_text(text) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_text('running shoes');

-- OK: varchar
PREPARE exec_phrase_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_varchar('running shoes'::varchar);

-- OK: text[]
PREPARE exec_phrase_text_array(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_text_array(ARRAY['running', 'shoes']::text[]);

-- OK: varchar[]
PREPARE exec_phrase_varchar_array(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_varchar_array(ARRAY['running', 'shoes']::varchar[]);

-- OK: pdb.query
PREPARE exec_phrase_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_query('running shoes'::pdb.query);

-- OK: pdb.boost (via implicit cast to pdb.query)
PREPARE exec_phrase_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_boost('running shoes'::pdb.boost(2));

-- OK: pdb.slop
PREPARE exec_phrase_slop(pdb.slop) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_slop('running shoes'::pdb.slop(2));

-- IGNORED: pdb.fuzzy (via implicit cast; fuzzy_data ignored by phrase_query)
PREPARE exec_phrase_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description ### $1;
EXECUTE exec_phrase_fuzzy('running shoes'::pdb.fuzzy(1));

-- -----------------------------------------------------------------------------
-- 2.4 === Operator (Term) - exec_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
PREPARE exec_term_text(text) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_text('keyboard');

-- OK: varchar
PREPARE exec_term_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_varchar('keyboard'::varchar);

-- OK: text[]
PREPARE exec_term_text_array(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_text_array(ARRAY['keyboard', 'plastic']::text[]);

-- OK: varchar[]
PREPARE exec_term_varchar_array(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_varchar_array(ARRAY['keyboard']::varchar[]);

-- OK: pdb.query
PREPARE exec_term_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_query('keyboard'::pdb.query);

-- OK: pdb.boost (via implicit cast to pdb.query)
PREPARE exec_term_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_boost('keyboard'::pdb.boost(2));

-- OK: pdb.fuzzy (via implicit cast to pdb.query)
PREPARE exec_term_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_fuzzy('keyboard'::pdb.fuzzy(1));

-- REJECT: pdb.slop (operator is not unique - no direct stub for pdb.slop)
PREPARE exec_term_slop(pdb.slop) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_slop('keyboard'::pdb.slop(2));

-- REJECT: pdb.parse() - already classified query rejected by ===
PREPARE exec_term_parse_reject(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description === $1;
EXECUTE exec_term_parse_reject(pdb.parse('keyboard'));

-- -----------------------------------------------------------------------------
-- 2.5 @@@ Operator (Parse/Proximity) - exec_rewrite
-- -----------------------------------------------------------------------------

-- OK: text
PREPARE exec_parse_text(text) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_text('running shoes');

-- OK: varchar
PREPARE exec_parse_varchar(varchar) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_varchar('running shoes'::varchar);

-- OK: pdb.query
PREPARE exec_parse_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_query('running shoes'::pdb.query);

-- OK: pdb.boost (via implicit cast to pdb.query)
PREPARE exec_parse_boost(pdb.boost) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_boost('running shoes'::pdb.boost(2));

-- OK: pdb.fuzzy (via implicit cast to pdb.query)
PREPARE exec_parse_fuzzy(pdb.fuzzy) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_fuzzy('running'::pdb.fuzzy(1));

-- OK: pdb.ProximityClause
PREPARE exec_parse_prox(pdb.proximityclause) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_prox(pdb.prox_clause('running', 1, 'shoes'));

-- REJECT: text[] not supported by @@@
PREPARE exec_parse_text_array(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_text_array(ARRAY['running', 'shoes']::text[]);

-- REJECT: varchar[] not supported by @@@
PREPARE exec_parse_varchar_array(varchar[]) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_varchar_array(ARRAY['running', 'shoes']::varchar[]);

-- REJECT: incomplete ProximityClause
PREPARE exec_parse_prox_incomplete(pdb.proximityclause) AS
SELECT array_agg(id ORDER BY id) AS ids
FROM mock_items WHERE description @@@ $1;
EXECUTE exec_parse_prox_incomplete(pdb.prox_term('running'));

-- =============================================================================
-- CLEANUP
-- =============================================================================

-- Deallocate all prepared statements
DEALLOCATE exec_and_text;
DEALLOCATE exec_and_varchar;
DEALLOCATE exec_and_text_array;
DEALLOCATE exec_and_varchar_array;
DEALLOCATE exec_and_query;
DEALLOCATE exec_and_boost;
DEALLOCATE exec_and_fuzzy;
DEALLOCATE exec_and_slop;
DEALLOCATE exec_or_text;
DEALLOCATE exec_or_varchar;
DEALLOCATE exec_or_text_array;
DEALLOCATE exec_or_varchar_array;
DEALLOCATE exec_or_query;
DEALLOCATE exec_or_boost;
DEALLOCATE exec_or_fuzzy;
DEALLOCATE exec_or_slop;
DEALLOCATE exec_phrase_text;
DEALLOCATE exec_phrase_varchar;
DEALLOCATE exec_phrase_text_array;
DEALLOCATE exec_phrase_varchar_array;
DEALLOCATE exec_phrase_query;
DEALLOCATE exec_phrase_boost;
DEALLOCATE exec_phrase_slop;
DEALLOCATE exec_phrase_fuzzy;
DEALLOCATE exec_term_text;
DEALLOCATE exec_term_varchar;
DEALLOCATE exec_term_text_array;
DEALLOCATE exec_term_varchar_array;
DEALLOCATE exec_term_query;
DEALLOCATE exec_term_boost;
DEALLOCATE exec_term_fuzzy;
DEALLOCATE exec_term_slop;
DEALLOCATE exec_term_parse_reject;
DEALLOCATE exec_parse_text;
DEALLOCATE exec_parse_varchar;
DEALLOCATE exec_parse_query;
DEALLOCATE exec_parse_boost;
DEALLOCATE exec_parse_fuzzy;
DEALLOCATE exec_parse_prox;
DEALLOCATE exec_parse_text_array;
DEALLOCATE exec_parse_varchar_array;
DEALLOCATE exec_parse_prox_incomplete;

RESET plan_cache_mode;

DROP TABLE mock_items;

\i common/common_cleanup.sql
