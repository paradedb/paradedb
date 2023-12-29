\echo Creating pg_columnar extension...
\i create_hot.sql

\echo Loading data...
\timing on
TRUNCATE hits;
\copy hits FROM 'hits_100k_rows.csv' WITH (FORMAT CSV, QUOTE '"', ESCAPE '"');
VACUUM FREEZE hits;

\echo Running queries...
\timing on
\i queries.sql
