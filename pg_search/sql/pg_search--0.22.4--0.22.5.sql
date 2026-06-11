\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.22.5'" to load this file. \quit

ALTER TYPE pdb.literal SET (TYPMOD_IN = literal_typmod_in, TYPMOD_OUT = generic_typmod_out);
