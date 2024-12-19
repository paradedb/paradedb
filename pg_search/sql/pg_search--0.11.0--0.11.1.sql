DELETE
FROM pg_depend
WHERE classid = 2602 /* AccessMethodOperatorRelationId */
  AND objid = (SELECT oid
               FROM pg_amop
               WHERE amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops')
                 AND amoprighttype = 'jsonb'::regtype::oid);

DELETE
from pg_amop
where amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops')
  and amoprighttype = 'jsonb'::regtype::oid;

UPDATE pg_amop
SET amopstrategy = 1
WHERE amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops')
  and amoprighttype = 'text'::regtype::oid;

UPDATE pg_amop
SET amopstrategy = 2
WHERE amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops')
  and amoprighttype = 'paradedb.searchqueryinput'::regtype::oid;

--
-- drop the search_config functions
--
DROP OPERATOR IF EXISTS @@@(anyelement, jsonb) CASCADE;
DROP FUNCTION IF EXISTS search_config_restrict(planner_info internal, operator_oid oid, args internal,
                                               _var_relid pg_catalog.int4);
DROP FUNCTION IF EXISTS search_with_search_config(element anyelement, config_json jsonb);
DROP FUNCTION IF EXISTS search_config_support(arg internal);
