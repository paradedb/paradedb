DELETE FROM paradedb._typmod_cache WHERE typmod = ARRAY['66', '77'];
DELETE FROM paradedb._typmod_cache WHERE typmod = ARRAY['66', '78'];
BEGIN;
SELECT 'hello, world'::pdb.ngram(66, 77)::text[];
ABORT;
SELECT 'hello, world'::pdb.ngram(66, 77)::text[];

-- An ordinary role may read the shared typmod cache, and may still add entries
-- through the SECURITY DEFINER path, but must not mutate it directly: the cache
-- backs every user's ParadeDB index tokenizer configuration, so direct writes
-- stay with the table owner.
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'typmod_cache_lowpriv') THEN
        CREATE ROLE typmod_cache_lowpriv;
    END IF;
END
$$;
SET ROLE typmod_cache_lowpriv;
SELECT count(*) >= 0 AS can_select FROM paradedb._typmod_cache;
-- pg_dump reads last_value straight off the sequence, so SELECT has to survive
-- there, but nothing outside _save_typmod may advance it.
SELECT last_value >= 0 AS can_read_seq FROM paradedb._typmod_cache_id_seq;
SELECT nextval('paradedb._typmod_cache_id_seq');
INSERT INTO paradedb._typmod_cache (typmod) VALUES (ARRAY['9', '9']);
UPDATE paradedb._typmod_cache SET typmod = ARRAY['9', '9'];
DELETE FROM paradedb._typmod_cache;
TRUNCATE paradedb._typmod_cache;
-- The supported write path still works for that role. The typmod pair must not
-- appear earlier in this file: save_typmod memoises per backend, so a reused
-- pair would never reach SQL and would prove nothing.
SELECT 'hello, world'::pdb.ngram(66, 78)::text[];
RESET ROLE;
DROP ROLE IF EXISTS typmod_cache_lowpriv;
