CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7533_docs (id int PRIMARY KEY, body text, category text);
INSERT INTO issue_7533_docs VALUES
    (1, 'hello', 'a'), (2, 'hello', 'b'), (3, 'hello', NULL),
    (4, 'other', 'a'), (5, 'hello hello', 'a');
CREATE INDEX issue_7533_idx ON issue_7533_docs
USING paradedb (id, body);
ANALYZE issue_7533_docs;

-- The same parameter controls the optional filter and its value.
PREPARE issue_7533(text, text) AS
SELECT id, pdb.score(id) AS score FROM issue_7533_docs
WHERE body ||| $1 AND ($2 IS NULL OR category = $2)
ORDER BY pdb.score(id) DESC, id LIMIT 2;

-- Generic plans must return the same ids, scores and order as these custom plans.
SET plan_cache_mode = force_custom_plan;
EXPLAIN (COSTS OFF) EXECUTE issue_7533('hello', 'a');
EXECUTE issue_7533('hello', 'a');
EXECUTE issue_7533('hello', NULL);
EXECUTE issue_7533('hello', 'b');
EXECUTE issue_7533('hello', 'missing');

SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF) EXECUTE issue_7533('hello', 'a');
EXECUTE issue_7533('hello', 'a');
EXECUTE issue_7533('hello', NULL);
EXECUTE issue_7533('hello', 'b');
EXECUTE issue_7533('hello', 'missing');
EXECUTE issue_7533('hello', 'a');
EXECUTE issue_7533('hello', NULL);

-- text -> varchar wraps the parameter in a RelabelType.
PREPARE issue_7533_relabel(text, text) AS
SELECT id, pdb.score(id) AS score FROM issue_7533_docs
WHERE body ||| $1 AND (($2::varchar) IS NULL OR category = $2)
ORDER BY pdb.score(id) DESC, id LIMIT 2;
EXECUTE issue_7533_relabel('hello', 'b');

-- A varchar length limit adds a FuncExpr around the parameter.
PREPARE issue_7533_typmod(text, text) AS
SELECT id, pdb.score(id) AS score FROM issue_7533_docs
WHERE body ||| $1 AND (($2::varchar(10)) IS NULL OR category = $2)
ORDER BY pdb.score(id) DESC, id LIMIT 2;
EXECUTE issue_7533_typmod('hello', NULL);

DEALLOCATE issue_7533;
DEALLOCATE issue_7533_relabel;
DEALLOCATE issue_7533_typmod;

RESET plan_cache_mode;
DROP TABLE issue_7533_docs;
