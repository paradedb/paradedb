DELETE from pg_amop where amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops') and amoprighttype = 'jsonb'::regtype::oid;

UPDATE pg_amop SET amopstrategy = 1 WHERE amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops') and amoprighttype = 'text'::regtype::oid;

UPDATE pg_amop SET amopstrategy = 2 WHERE amopfamily = (select oid from pg_opfamily where opfname = 'anyelement_bm25_ops') and amoprighttype = 'paradedb.searchqueryinput'::regtype::oid;
