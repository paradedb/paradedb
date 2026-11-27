\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.27.1'" to load this file. \quit

-- ============================================================================
-- Fragment: 7655.remove_paradedb_aggregate.sql
-- ============================================================================
DROP FUNCTION IF EXISTS aggregate(regclass, searchqueryinput, pg_catalog.json, bool, bigint, bigint, text);
