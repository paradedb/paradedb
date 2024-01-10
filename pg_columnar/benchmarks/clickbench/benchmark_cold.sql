\echo Creating pg_columnar extension...
CREATE EXTENSION pg_columnar;

\echo Initialize ParadeDB context...
SELECT paradedb.init();

\echo Creating persistent table...
\i create_cold.sql

\echo Loading data...
\timing on
TRUNCATE hits;
\copy hits FROM 'hits_100k_rows.csv' WITH (FORMAT CSV, QUOTE '"', ESCAPE '"');
VACUUM FREEZE hits;

\echo Running ClickBench queries...
\timing on
\i queries.sql
