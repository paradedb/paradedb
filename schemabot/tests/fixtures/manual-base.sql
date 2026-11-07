CREATE TYPE pdb.source_code (INPUT = pdb.decode, OUTPUT = pdb.encode, STORAGE = plain);
CREATE TABLE unsupported(x int);
CREATE FUNCTION f() RETURNS int LANGUAGE sql AS $$SELECT 1$$;
