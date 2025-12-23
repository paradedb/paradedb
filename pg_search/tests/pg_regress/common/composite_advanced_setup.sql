-- Setup for advanced composite type tests (MVCC, parallel build, fast fields)
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Create a schema for composite advanced tests
DROP SCHEMA IF EXISTS composite_adv CASCADE;
CREATE SCHEMA composite_adv;
SET search_path TO composite_adv, public;
