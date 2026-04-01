\i common/common_setup.sql

CALL paradedb.create_bm25_test_table(
  schema_name => 'public',
  table_name => 'mock_items'
);

CREATE INDEX search_idx ON mock_items
USING bm25 (id, description)
WITH (key_field='id');

SET plan_cache_mode = force_generic_plan;

-- BUG: parameterized LIMIT causes NormalScanExecState instead of TopKScanExecState
PREPARE param_limit(text, int) AS
SELECT
  id,
  pdb.score(id)::float8
FROM mock_items
WHERE id @@@ pdb.parse($1)
ORDER BY pdb.score(id) DESC
LIMIT $2;

EXPLAIN (COSTS OFF)
EXECUTE param_limit('description:keyboard', 5);

EXECUTE param_limit('description:keyboard', 5);

DEALLOCATE param_limit;

-- Control: constant LIMIT correctly uses TopKScanExecState
PREPARE const_limit(text) AS
SELECT
  id,
  pdb.score(id)::float8
FROM mock_items
WHERE id @@@ pdb.parse($1)
ORDER BY pdb.score(id) DESC
LIMIT 5;

EXPLAIN (COSTS OFF)
EXECUTE const_limit('description:keyboard');

EXECUTE const_limit('description:keyboard');

DEALLOCATE const_limit;

DROP TABLE mock_items;

\i common/common_cleanup.sql
