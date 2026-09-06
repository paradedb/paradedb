CREATE EXTENSION IF NOT EXISTS pg_search;

-- A STABLE expression on the right-hand side of @@@ (the shape a row-level
-- security policy produces) is solved at execution time, but it must also be
-- estimated at plan time, so that the custom scan competes on real numbers
-- when the query has more than one qual. See issue #6151.

CREATE TABLE issue_6151 (
    id bigint PRIMARY KEY,
    tenant text,
    body text
);
INSERT INTO issue_6151
SELECT g,
       CASE WHEN g <= 100 THEN 'alpha' ELSE 'beta' END,
       'lorem ipsum ' || g
FROM generate_series(1, 10000) g;

CREATE INDEX issue_6151_idx
ON issue_6151 USING paradedb (id, ((tenant)::pdb.literal), body)
WITH (key_field = 'id');
ANALYZE issue_6151;

-- The tenant filter arrives through a STABLE function reading session state,
-- exactly as an RLS policy's USING clause would deliver it.
CREATE FUNCTION issue_6151_tenant_query() RETURNS paradedb.searchqueryinput AS $$
  SELECT paradedb.term('tenant', current_setting('issue_6151.tenant'))
$$ LANGUAGE sql STABLE;

CREATE FUNCTION issue_6151_plan_uses(q text, needle text) RETURNS boolean AS $$
DECLARE r record;
BEGIN
  FOR r IN EXECUTE 'EXPLAIN (COSTS OFF) ' || q LOOP
    IF r."QUERY PLAN" LIKE '%' || needle || '%' THEN RETURN true; END IF;
  END LOOP;
  RETURN false;
END $$ LANGUAGE plpgsql;

SET issue_6151.tenant = 'alpha';

-- Two quals, one of them the STABLE expression: the estimator folds the
-- expression, and the custom scan wins over the plain index scan.
SELECT issue_6151_plan_uses(
  $$SELECT id FROM issue_6151
    WHERE id @@@ issue_6151_tenant_query() AND body @@@ 'lorem'$$,
  'ParadeDB Base Scan') AS multi_qual_uses_custom_scan;

SELECT count(*) FROM issue_6151
WHERE id @@@ issue_6151_tenant_query() AND body @@@ 'lorem';

-- A single expression qual keeps the custom scan.
SELECT issue_6151_plan_uses(
  $$SELECT id FROM issue_6151 WHERE id @@@ issue_6151_tenant_query()$$,
  'ParadeDB Base Scan') AS single_qual_uses_custom_scan;

-- The fold is for estimation only: nothing folded may be cached in a plan, so
-- a prepared statement must follow the session state on every execution, also
-- once a generic plan becomes possible after the fifth execution.
PREPARE issue_6151_count AS
SELECT count(*) FROM issue_6151
WHERE id @@@ issue_6151_tenant_query() AND body @@@ 'lorem';
EXECUTE issue_6151_count;
EXECUTE issue_6151_count;
EXECUTE issue_6151_count;
EXECUTE issue_6151_count;
EXECUTE issue_6151_count;
EXECUTE issue_6151_count;
SET issue_6151.tenant = 'beta';
EXECUTE issue_6151_count;
DEALLOCATE issue_6151_count;

-- An expression qual combined with a heap filter keeps the parameterized
-- guess (a heap filter needs an executor expression context that does not
-- exist at plan time) and must plan and run without error.
SELECT count(*) FROM issue_6151
WHERE id @@@ issue_6151_tenant_query() AND length(body) > 15;

DROP FUNCTION issue_6151_tenant_query(), issue_6151_plan_uses(text, text);
DROP TABLE issue_6151;
