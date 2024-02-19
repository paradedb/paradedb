
-- Configure search path to include paradedb schema
-- SET search_path TO paradedb, public;


-- Set search_path for the current database
ALTER DATABASE "$POSTGRES_DB" SET search_path TO public,paradedb;

-- Set search_path for the template1 database
-- TODO: Check if I need to set template1 as well
ALTER DATABASE template1 SET search_path TO public,paradedb;


GRANT USAGE ON LANGUAGE c TO postgres;
GRANT USAGE ON LANGUAGE c TO CURRENT_USER;
GRANT EXECUTE ON LANGUAGE c TO CURRENT_USER;



-- Install ParadeDB extensions
-- TODO: Add pg_cron
CREATE EXTENSION IF NOT EXISTS pg_bm25 CASCADE;
CREATE EXTENSION IF NOT EXISTS pg_analytics CASCADE;
CREATE EXTENSION IF NOT EXISTS vector CASCADE;
CREATE EXTENSION IF NOT EXISTS svector CASCADE;


-- TODO: Add PostHog

