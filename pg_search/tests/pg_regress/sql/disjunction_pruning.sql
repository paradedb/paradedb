\i common/common_setup.sql

RESET paradedb.disjunction_pruning;
SHOW paradedb.disjunction_pruning;
\set VERBOSITY terse
SET paradedb.disjunction_pruning = invalid;
\set VERBOSITY default

CREATE TABLE disjunction_pruning_docs (id integer PRIMARY KEY, body text);
INSERT INTO disjunction_pruning_docs
SELECT id, 'alpha beta gamma ' || repeat('filler ', 97)
FROM generate_series(1, 300) id;
INSERT INTO disjunction_pruning_docs
SELECT 1000 + n, repeat('alpha beta gamma ', n + 1) || repeat('filler ', 97 - 3 * n)
FROM generate_series(1, 10) n;
CREATE INDEX disjunction_pruning_idx ON disjunction_pruning_docs
USING paradedb (id, body);

SET plan_cache_mode = force_generic_plan;
PREPARE disjunction_pruning_topk(text) AS
SELECT id, pdb.score(id) AS score
FROM disjunction_pruning_docs
WHERE body ||| $1
ORDER BY pdb.score(id) DESC
LIMIT 10;

CREATE TEMP TABLE pruning_auto AS EXECUTE disjunction_pruning_topk('alpha beta gamma');
CREATE TEMP TABLE pruning_two_auto AS EXECUTE disjunction_pruning_topk('alpha beta');
SELECT count(*) = 10 AND min(id) = 1001 AND max(id) = 1010 AS expected_winners
FROM pruning_auto;

-- Reuse the prepared plan while changing the execution setting.
BEGIN;
SET LOCAL paradedb.disjunction_pruning = wand;
SHOW paradedb.disjunction_pruning;
CREATE TEMP TABLE pruning_wand AS EXECUTE disjunction_pruning_topk('alpha beta gamma');
CREATE TEMP TABLE pruning_single_wand AS EXECUTE disjunction_pruning_topk('alpha');
COMMIT;
SHOW paradedb.disjunction_pruning;

BEGIN;
SET LOCAL paradedb.disjunction_pruning = maxscore;
SHOW paradedb.disjunction_pruning;
CREATE TEMP TABLE pruning_maxscore AS EXECUTE disjunction_pruning_topk('alpha beta gamma');
CREATE TEMP TABLE pruning_two_maxscore AS EXECUTE disjunction_pruning_topk('alpha beta');
CREATE TEMP TABLE pruning_single_maxscore AS EXECUTE disjunction_pruning_topk('alpha');
COMMIT;
SHOW paradedb.disjunction_pruning;

SELECT count(*) = 10 AND bool_and(a.id IS NOT NULL AND w.id IS NOT NULL AND m.id IS NOT NULL)
           AS same_documents,
       bool_and(abs(a.score - w.score) < 0.00001 AND abs(a.score - m.score) < 0.00001)
           AS same_scores
FROM pruning_auto a
FULL JOIN pruning_wand w USING (id)
FULL JOIN pruning_maxscore m USING (id);

SELECT count(*) = 10 AND bool_and(w.id IS NOT NULL AND m.id IS NOT NULL) AS same_documents,
       bool_and(abs(w.score - m.score) < 0.00001) AS same_scores
FROM pruning_single_wand w
FULL JOIN pruning_single_maxscore m USING (id);

SELECT count(*) = 10 AND bool_and(a.id IS NOT NULL AND m.id IS NOT NULL) AS same_documents,
       bool_and(abs(a.score - m.score) < 0.00001) AS same_scores
FROM pruning_two_auto a
FULL JOIN pruning_two_maxscore m USING (id);

BEGIN;
SET LOCAL paradedb.disjunction_pruning = maxscore;
SHOW paradedb.disjunction_pruning;
ROLLBACK;
SHOW paradedb.disjunction_pruning;

DEALLOCATE disjunction_pruning_topk;
RESET plan_cache_mode;
DROP TABLE pruning_auto, pruning_wand, pruning_maxscore, pruning_single_wand, pruning_single_maxscore,
    pruning_two_auto, pruning_two_maxscore;
DROP TABLE disjunction_pruning_docs;

\i common/common_cleanup.sql
