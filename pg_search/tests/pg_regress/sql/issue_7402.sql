-- `@@@` classifies a `pdb.query` value that the planner can't fold, such as a subquery or a
-- parameter of a generic plan, the same way as a constant.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS issue_7402_products;

CREATE TABLE issue_7402_products (id int PRIMARY KEY, description text);
INSERT INTO issue_7402_products VALUES
    (1, 'leather sofa'),
    (2, 'soda can'),
    (3, 'oak table');
CREATE INDEX issue_7402_idx ON issue_7402_products USING bm25 (id, description);

-- The constant gives the rows that the other forms have to give.
SELECT id FROM issue_7402_products WHERE description @@@ 'sofa'::pdb.fuzzy(1) ORDER BY id;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT id FROM issue_7402_products
WHERE description @@@ (SELECT 'sofa'::pdb.fuzzy(1)::pdb.query);
SELECT id FROM issue_7402_products
WHERE description @@@ (SELECT 'sofa'::pdb.fuzzy(1)::pdb.query) ORDER BY id;

PREPARE issue_7402(pdb.query) AS
SELECT id FROM issue_7402_products WHERE description @@@ $1 ORDER BY id;
SET plan_cache_mode = force_generic_plan;
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF) EXECUTE issue_7402('sofa'::pdb.fuzzy(1)::pdb.query);
EXECUTE issue_7402('sofa'::pdb.fuzzy(1)::pdb.query);
RESET plan_cache_mode;
DEALLOCATE issue_7402;

DROP TABLE issue_7402_products;
