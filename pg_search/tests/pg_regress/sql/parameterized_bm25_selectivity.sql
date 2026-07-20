-- =====================================================================
-- Issue #5275: parameterized BM25 predicates in GENERIC prepared plans
--             must not collapse the planner row estimate to 1
-- =====================================================================
-- Without a restriction-selectivity function bound to the text / text[]
-- overloads of @@@ and |||, a GENERIC prepared plan for a BM25 predicate
-- falls to UNKNOWN_SELECTIVITY (0.00001) and clamps to `rows=1`. That
-- misleads parallel-worker selection and join-order decisions.
--
-- The fix attaches paradedb.query_input_restrict to those overloads and
-- extends the function to return PARAMETERIZED_SELECTIVITY for any
-- non-constant (or wrapped) RHS. This test exercises each overload with
-- both a constant and a parameterized RHS under force_generic_plan, and
-- asserts the queries execute and return the correct 2000-row match set.

CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE sel_repro (id serial PRIMARY KEY, content text);
CREATE INDEX sel_repro_idx ON sel_repro USING bm25 (id, content)
WITH (key_field='id');

INSERT INTO sel_repro (content)
SELECT (ARRAY['technology','science','cooking','sports','music','art'])[1 + (i % 6)]
FROM generate_series(1, 12000) i;

ANALYZE sel_repro;

-- Ground truth: 2000 rows match 'technology' (1/6 of 12000).
SELECT count(*) AS truth FROM sel_repro WHERE content @@@ 'technology';

-- Constant RHS on the text overload of @@@. Same result path as before.
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT count(*) FROM sel_repro WHERE content @@@ 'technology';

SELECT count(*) FROM sel_repro WHERE content @@@ 'technology';

-- Parameterized RHS on the text overload of @@@ under a GENERIC plan.
-- Without the fix the planner uses `rows=1` here and can collapse to a
-- nested loop or serial scan in join / worker planning.
SET plan_cache_mode = force_generic_plan;

PREPARE qa_text_atatat(text) AS
    SELECT count(*) FROM sel_repro WHERE content @@@ $1;
EXECUTE qa_text_atatat('technology');
EXECUTE qa_text_atatat('technology');
EXECUTE qa_text_atatat('technology');
EXECUTE qa_text_atatat('technology');
EXECUTE qa_text_atatat('technology');
DEALLOCATE qa_text_atatat;

-- Parameterized RHS on the text overload of |||.
PREPARE qb_text_ororor(text) AS
    SELECT count(*) FROM sel_repro WHERE content ||| $1;
EXECUTE qb_text_ororor('technology');
EXECUTE qb_text_ororor('technology');
EXECUTE qb_text_ororor('technology');
EXECUTE qb_text_ororor('technology');
EXECUTE qb_text_ororor('technology');
DEALLOCATE qb_text_ororor;

-- Parameterized RHS on the text[] overload of |||.
PREPARE qc_textarr_ororor(text[]) AS
    SELECT count(*) FROM sel_repro WHERE content ||| $1;
EXECUTE qc_textarr_ororor(ARRAY['technology']);
EXECUTE qc_textarr_ororor(ARRAY['technology']);
EXECUTE qc_textarr_ororor(ARRAY['technology']);
EXECUTE qc_textarr_ororor(ARRAY['technology']);
EXECUTE qc_textarr_ororor(ARRAY['technology']);
DEALLOCATE qc_textarr_ororor;

-- Parameterized RHS on the paradedb.searchqueryinput overload of @@@.
-- Historically this path already had a restriction function bound but
-- fell back to the collapsed estimate for wrapped-Param RHS shapes.
PREPARE qd_query_input(paradedb.searchqueryinput) AS
    SELECT count(*) FROM sel_repro WHERE id @@@ $1;
EXECUTE qd_query_input(paradedb.term('content','technology'));
EXECUTE qd_query_input(paradedb.term('content','technology'));
EXECUTE qd_query_input(paradedb.term('content','technology'));
EXECUTE qd_query_input(paradedb.term('content','technology'));
EXECUTE qd_query_input(paradedb.term('content','technology'));
DEALLOCATE qd_query_input;

RESET plan_cache_mode;

DROP TABLE sel_repro CASCADE;
