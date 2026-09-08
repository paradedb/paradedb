\i common/common_setup.sql
\set VERBOSITY default

CREATE TABLE whole_row_search (sku TEXT PRIMARY KEY, description TEXT, category TEXT);
INSERT INTO whole_row_search VALUES
    ('SKU-1', 'running shoes', 'sports'),
    ('SKU-2', 'wireless keyboard', 'shoes'),
    ('SKU-3', 'cotton shirt', 'apparel');
CREATE INDEX whole_row_search_idx ON whole_row_search
USING paradedb (sku, description, category) WITH (key_field = 'sku');

-- Bare terms search across indexed fields instead of the key field.
SELECT sku FROM whole_row_search AS t WHERE t @@@ 'shoes' ORDER BY sku;
SELECT sku FROM whole_row_search AS t WHERE t @@@ 'shoes'::pdb.query ORDER BY sku;
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.parse('shoes') ORDER BY sku;
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.parse('description:shoes') ORDER BY sku;
SELECT sku FROM whole_row_search AS t
WHERE t @@@ pdb.parse('description:(running shoes)', conjunction_mode => true) ORDER BY sku;
SELECT sku FROM whole_row_search AS t
WHERE t @@@ pdb.parse('description:shoes OR missing:shoes', lenient => true) ORDER BY sku;
SELECT sku FROM whole_row_search AS t WHERE t.sku === 'SKU-1' ORDER BY sku;
SELECT sku FROM whole_row_search AS t WHERE t.description @@@ pdb.term('shoes') ORDER BY sku;

-- Whole-row match-all queries retain their scores through adjustments.
SELECT sku, pdb.score(t.sku) FROM whole_row_search AS t WHERE t @@@ pdb.all() ORDER BY sku;
SELECT sku, pdb.score(t.sku) FROM whole_row_search AS t WHERE t @@@ pdb.all()::pdb.boost(2) ORDER BY sku;
SELECT sku, pdb.score(t.sku) FROM whole_row_search AS t WHERE t @@@ pdb.all()::pdb.const(3) ORDER BY sku;
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.empty() ORDER BY sku;

-- EXPLAIN identifies both whole-row and field-bound match-all scans.
EXPLAIN (COSTS OFF)
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.all() ORDER BY sku LIMIT 1;
EXPLAIN (COSTS OFF)
SELECT sku FROM whole_row_search AS t WHERE t.sku @@@ pdb.all() ORDER BY sku LIMIT 1;

-- Field-specific queries share the same error and actionable hint.
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.term('shoes');
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.term('shoes')::pdb.boost(2);
SELECT sku FROM whole_row_search AS t WHERE t @@@ pdb.prox_clause('running', 0, 'shoes');

-- A raw array is not a complete proximity clause.
SELECT sku FROM whole_row_search AS t WHERE t @@@ ARRAY['shoes', 'keyboard'];

SET plan_cache_mode = force_generic_plan;
PREPARE whole_row_query(pdb.query) AS
SELECT sku, pdb.score(t.sku) FROM whole_row_search AS t WHERE t @@@ $1 ORDER BY sku;
EXPLAIN (COSTS OFF) EXECUTE whole_row_query(pdb.all());
EXECUTE whole_row_query(pdb.all());
EXECUTE whole_row_query(pdb.all()::pdb.boost(2));
EXECUTE whole_row_query(pdb.all()::pdb.const(3));
EXECUTE whole_row_query(pdb.empty());
EXECUTE whole_row_query(pdb.parse('description:shoes')::pdb.const(4));
EXECUTE whole_row_query('shoes'::pdb.query::pdb.const(4));
EXECUTE whole_row_query(pdb.term('shoes'));
EXECUTE whole_row_query(pdb.all());
DEALLOCATE whole_row_query;

PREPARE whole_row_text(text) AS
SELECT sku FROM whole_row_search AS t WHERE t @@@ $1 ORDER BY sku;
EXECUTE whole_row_text('shoes');
EXECUTE whole_row_text('description:keyboard');
DEALLOCATE whole_row_text;
RESET plan_cache_mode;

DROP TABLE whole_row_search;
\set VERBOSITY terse
\i common/common_cleanup.sql
