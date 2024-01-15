\echo Creating pg_analytics extension...
CREATE EXTENSION pg_analytics;

\echo Initialize ParadeDB context...
CALL paradedb.init();

\echo Creating persistent table...
\i create_cold.sql

\echo Loading data...
\timing on
TRUNCATE hits;
\copy hits FROM 'hits_100k_rows.csv' WITH (FORMAT CSV, QUOTE '"', ESCAPE '"');
VACUUM FREEZE hits;

-- In benchmark_cold.sql, we run the queries via `./run.sh`
