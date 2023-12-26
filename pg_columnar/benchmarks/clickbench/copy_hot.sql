\echo Creating pg_columnar extension...
\i create_hot.sql

\echo Loading data...
\timing on
BEGIN;
TRUNCATE hits;
\copy hits FROM 'hits005.tsv' WITH FREEZE;
VACUUM ANALYZE hits;
COMMIT;

\echo Running queries...
\timing on
\i queries.sql
