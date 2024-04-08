\echo Creating pg_analytics extension...
DROP EXTENSION IF EXISTS pg_analytics CASCADE;
CREATE EXTENSION pg_analytics;

\echo Creating persistent table...
\i create.sql

\echo Loading data...
\timing on
-- TODO: Generate and load the data for 100GB
-- Can add truncate and vacuum freeze

\echo Running TPC-H queries...
\timing on
\i queries.sql
