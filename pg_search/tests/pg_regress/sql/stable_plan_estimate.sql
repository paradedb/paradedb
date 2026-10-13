-- Runtime (STABLE) search predicates keep an index-based planner row estimate.
CREATE EXTENSION IF NOT EXISTS pg_search;
SET paradedb.enable_aggregate_custom_scan = off;

CREATE TABLE estimate_docs (id int PRIMARY KEY, tenant_id int, body text);
-- tenant 1 owns 1% of the rows, tenant 2 owns 99%
INSERT INTO estimate_docs
SELECT g, CASE WHEN g % 100 = 0 THEN 1 ELSE 2 END, 'canal' FROM generate_series(1, 10000) g;
CREATE INDEX estimate_docs_idx ON estimate_docs USING bm25 (id, tenant_id, body);
ANALYZE estimate_docs;

CREATE FUNCTION estimate_plan_rows(q text) RETURNS text LANGUAGE plpgsql AS $$
DECLARE plan json;
BEGIN
    EXECUTE 'EXPLAIN (FORMAT JSON) ' || q INTO plan;
    RETURN plan -> 0 -> 'Plan' ->> 'Plan Rows';
END
$$;

-- A STABLE value folds for estimation only, so it estimates like the literal.
SET app.tenant_id = '1';
SELECT estimate_plan_rows($$SELECT id FROM estimate_docs WHERE body @@@ 'canal' AND tenant_id = 1$$) AS literal_rows,
       estimate_plan_rows($$SELECT id FROM estimate_docs WHERE body @@@ 'canal' AND tenant_id = current_setting('app.tenant_id')::int$$) AS stable_rows;
SET app.tenant_id = '2';
SELECT estimate_plan_rows($$SELECT id FROM estimate_docs WHERE body @@@ 'canal' AND tenant_id = 2$$) AS literal_rows,
       estimate_plan_rows($$SELECT id FROM estimate_docs WHERE body @@@ 'canal' AND tenant_id = current_setting('app.tenant_id')::int$$) AS stable_rows;

-- Execution still uses the current value, not the one folded at plan time.
PREPARE estimate_rows AS SELECT count(*) FROM estimate_docs
WHERE body @@@ 'canal' AND tenant_id = current_setting('app.tenant_id')::int;
SET plan_cache_mode = force_generic_plan;
SET app.tenant_id = '1';
EXECUTE estimate_rows;
SET app.tenant_id = '2';
EXECUTE estimate_rows;

-- An unbound parameter cannot fold, so the parameterized guess remains.
PREPARE estimate_param(int) AS SELECT id FROM estimate_docs WHERE body @@@ 'canal' AND tenant_id = $1;
SELECT estimate_plan_rows('EXECUTE estimate_param(1)') AS param_rows;

DEALLOCATE ALL;
DROP FUNCTION estimate_plan_rows(text);
DROP TABLE estimate_docs;
RESET plan_cache_mode;
RESET app.tenant_id;
RESET paradedb.enable_aggregate_custom_scan;
