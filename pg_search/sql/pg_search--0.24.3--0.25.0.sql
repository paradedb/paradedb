\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.25.0'" to load this file. \quit

-- 0.25.0 is an internal, never-released step of the vector development line
-- (no v0.25.0 tag exists). All of its schema DDL ships in
-- pg_search--0.25.0--0.25.1.sql instead, so the single migration ending at the
-- current version carries the complete delta (SchemaBot validates that file).
-- This script intentionally contains no DDL: it exists to connect the released
-- 0.24.x line to the 0.25.x line.
