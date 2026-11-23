-- V1 migrations are diagnosed before the planner rewrites search and score expressions.
\set VERBOSITY default
\pset format unaligned
SET paradedb.planner_warnings = 'off';
SHOW paradedb.v1api_warnings;
CREATE TABLE v1_api_items (id integer PRIMARY KEY, body text);
INSERT INTO v1_api_items VALUES (1, 'running shoes'), (2, 'walking boots');
CREATE INDEX v1_api_idx ON v1_api_items USING bm25 (id, body)
WITH (key_field = 'id', text_fields = '{"body": {"tokenizer": {"type": "default"}}}');

SELECT id, pdb.score(id) = paradedb.score(id) AS same_score,
       paradedb.snippet(body) AS snippet
FROM v1_api_items WHERE id @@@ paradedb.term('body', 'shoes');

WITH matches AS (
    SELECT id FROM v1_api_items WHERE body @@@ pdb.match('shoes')
)
SELECT id FROM matches;

-- The legacy comparison matches an indexed token in the body.
SELECT id FROM v1_api_items WHERE id @@@ paradedb.term_with_operator('body', '=', 'shoes'::text);

-- Legacy JSON and argument values are distinct from V2 builders, including when folded.
SELECT '{"all":null}'::paradedb.searchqueryinput;
SELECT '{"all":null}'::jsonb::paradedb.searchqueryinput;
SELECT paradedb.searchqueryinput_in('"all"');
SELECT paradedb.disjunction_max(ARRAY[paradedb.all()]);

-- Existing indexes can still be read and maintained in error mode.
SET paradedb.v1api_warnings = 'off';
SELECT id FROM v1_api_items WHERE id @@@ paradedb.term('body', 'shoes');
SET plan_cache_mode = force_generic_plan;
PREPARE v1_api_search(text) AS
SELECT id FROM v1_api_items WHERE id @@@ paradedb.term('body', $1);
EXECUTE v1_api_search('shoes');

SET paradedb.v1api_warnings = 'error';
EXECUTE v1_api_search('shoes');
PREPARE v1_api_literal AS SELECT paradedb.all();
EXPLAIN (COSTS OFF) EXECUTE v1_api_literal;
EXECUTE v1_api_literal;
DEALLOCATE v1_api_literal;
EXPLAIN (ANALYZE, COSTS OFF) SELECT paradedb.all();
ALTER INDEX v1_api_idx SET (text_fields = '{}');
ALTER INDEX v1_api_idx SET (mutable_segment_rows = 0);
REINDEX INDEX v1_api_idx;
SELECT id FROM v1_api_items WHERE body ||| 'shoes' AND id = 1;
SELECT id FROM v1_api_items WHERE v1_api_items @@@ pdb.parse('body:shoes');
SELECT id, pdb.score(id) FROM v1_api_items
WHERE v1_api_items @@@ pdb.all()::pdb.const(0) ORDER BY id;
SELECT pdb.more_like_this(document => '{"body": "running shoes"}') IS NOT NULL AS has_query;
SELECT paradedb.index_fields('v1_api_idx') IS NOT NULL AS has_fields;

CREATE FUNCTION public.all() RETURNS integer LANGUAGE sql IMMUTABLE RETURN 1;
SELECT public.all();
DROP FUNCTION public.all();

SET paradedb.v1api_warnings = 'off';
SET paradedb.planner_warnings = 'error';
ALTER INDEX v1_api_idx SET (key_field = 'id');
SELECT paradedb.all();
DEALLOCATE v1_api_search;
DROP TABLE v1_api_items;
RESET plan_cache_mode;
RESET paradedb.planner_warnings;
RESET paradedb.v1api_warnings;
