-- Setup for composite type tests
CREATE EXTENSION IF NOT EXISTS pg_search;

-- Disable parallel workers for deterministic results
SET max_parallel_workers_per_gather = 0;

-- Create a schema for composite tests
DROP SCHEMA IF EXISTS composite_test CASCADE;
CREATE SCHEMA composite_test;
SET search_path TO composite_test, public;
