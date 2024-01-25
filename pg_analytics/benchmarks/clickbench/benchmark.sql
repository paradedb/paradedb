\echo Creating pg_analytics extension...
DROP EXTENSION IF EXISTS pg_analytics CASCADE;
CREATE EXTENSION pg_analytics;

\echo Creating persistent table...
\i create.sql

\echo Loading data...
\timing on
TRUNCATE hits;
\copy hits FROM 'hits_100k_rows.csv' WITH (FORMAT CSV, QUOTE '"', ESCAPE '"');
VACUUM FREEZE hits;

\echo Running ClickBench queries...
\timing on
\i queries.sql
