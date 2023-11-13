-- complain if script is sourced in psql, rather than via CREATE EXTENSION
\echo Use "ALTER EXTENSION svector UPDATE TO '0.4.0'" to load this file. \quit

-- remove this single line for Postgres < 13
ALTER TYPE svector SET (STORAGE = extended);
