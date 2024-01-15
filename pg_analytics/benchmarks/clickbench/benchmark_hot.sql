\echo Creating pg_analytics extension...
CREATE EXTENSION pg_analytics;

\echo Initialize ParadeDB context...
CALL paradedb.init();

\echo Creating temporary table...
\i create_hot.sql

\echo Loading data...
\timing on
TRUNCATE hits;
\copy hits FROM 'hits_100k_rows.csv' WITH (FORMAT CSV, QUOTE '"', ESCAPE '"');
VACUUM FREEZE hits;

\echo Running ClickBench queries...
\timing on
\i queries.sql
