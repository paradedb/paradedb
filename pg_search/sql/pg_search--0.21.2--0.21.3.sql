DROP TYPE IF EXISTS pdb.icu CASCADE;
CREATE TYPE pdb.icu;
CREATE OR REPLACE FUNCTION pdb.icu_in(cstring) RETURNS pdb.icu AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.icu_out(pdb.icu) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.icu_send(pdb.icu) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
CREATE OR REPLACE FUNCTION pdb.icu_recv(internal) RETURNS pdb.icu AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;
CREATE TYPE pdb.icu (
                          INPUT = pdb.icu_in,
                          OUTPUT = pdb.icu_out,
                          SEND = pdb.icu_send,
                          RECEIVE = pdb.icu_recv,
                          COLLATABLE = true,
                          CATEGORY = 't', -- 't' is for tokenizer
                          PREFERRED = false,
                          LIKE = text
                      );

ALTER TYPE pdb.icu SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);
