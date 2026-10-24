-- Issue #6893: use PostgreSQL's FuncExpr constructor in operator rewrites.
-- IvorySQL's extra FuncExpr fields caused a compile-time failure, which cannot
-- be reproduced in SQL on upstream PostgreSQL. Exercise the affected builders
-- here with generic plans: parameters must remain runtime expressions, rather
-- than being constant-folded before the FuncExpr construction paths are reached.
CREATE TABLE issue_6893_docs (id integer, body text);
INSERT INTO issue_6893_docs VALUES
    (1, 'alpha red'), (2, 'alpha blue'), (3, 'beta blue'), (4, 'gamma red');
CREATE INDEX issue_6893_idx ON issue_6893_docs USING paradedb (id, body);
SET plan_cache_mode = force_generic_plan;
SET paradedb.planner_warnings = off;

-- Text runtime constructor and the surrounding with_index expression.
PREPARE issue_6893_text(text) AS
SELECT array_agg(id ORDER BY id) AS ids FROM issue_6893_docs WHERE body ||| $1;
EXECUTE issue_6893_text('alpha red');
EXECUTE issue_6893_text('beta blue');

-- The array overload must retain the argument type and conjunction semantics.
PREPARE issue_6893_array(text[]) AS
SELECT array_agg(id ORDER BY id) AS ids FROM issue_6893_docs WHERE body &&& $1;
EXECUTE issue_6893_array(ARRAY['alpha', 'red']);
EXECUTE issue_6893_array(ARRAY['beta', 'blue']);

-- Parse runtime constructors, both field-bound and whole-row signatures.
PREPARE issue_6893_parse(text) AS
SELECT array_agg(id ORDER BY id) AS ids FROM issue_6893_docs WHERE body @@@ $1;
EXECUTE issue_6893_parse('alpha');
EXECUTE issue_6893_parse('beta');
PREPARE issue_6893_row(text) AS
SELECT array_agg(id ORDER BY id) AS ids FROM issue_6893_docs WHERE issue_6893_docs @@@ $1;
EXECUTE issue_6893_row('body:alpha');
EXECUTE issue_6893_row('body:beta');

-- A pdb.* RHS needs both an implicit cast FuncExpr and the outer query FuncExpr.
PREPARE issue_6893_fuzzy(text) AS
SELECT array_agg(id ORDER BY id) AS ids FROM issue_6893_docs WHERE body === $1::pdb.fuzzy(1, f, f);
EXECUTE issue_6893_fuzzy('alph');
EXECUTE issue_6893_fuzzy('bet');

-- Parse also accepts an already classified pdb.query parameter.
PREPARE issue_6893_query(pdb.query) AS
SELECT array_agg(id ORDER BY id) AS ids FROM issue_6893_docs WHERE body @@@ $1;
EXECUTE issue_6893_query(pdb.term('alpha'));
EXECUTE issue_6893_query(pdb.term('beta'));

-- Verify plan reuse; results above must not come from parameter-specific plans.
SELECT name, generic_plans, custom_plans
FROM pg_prepared_statements WHERE name LIKE 'issue_6893_%' ORDER BY name;
DEALLOCATE issue_6893_text;
DEALLOCATE issue_6893_array;
DEALLOCATE issue_6893_parse;
DEALLOCATE issue_6893_row;
DEALLOCATE issue_6893_fuzzy;
DEALLOCATE issue_6893_query;
RESET plan_cache_mode;
RESET paradedb.planner_warnings;
DROP TABLE issue_6893_docs;
