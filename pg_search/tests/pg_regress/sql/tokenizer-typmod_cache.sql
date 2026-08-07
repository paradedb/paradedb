DELETE FROM paradedb._typmod_cache WHERE typmod = ARRAY['66', '77'];
BEGIN;
SELECT 'hello, world'::pdb.ngram(66, 77)::text[];
ABORT;
SELECT 'hello, world'::pdb.ngram(66, 77)::text[];

-- An ordinary role may read the shared typmod cache but must not mutate it:
-- the cache backs every user's ParadeDB index tokenizer configuration, so writes
-- stay with the table owner (inserts go through SECURITY DEFINER _save_typmod).
CREATE ROLE typmod_cache_lowpriv;
SET ROLE typmod_cache_lowpriv;
SELECT count(*) >= 0 AS can_select FROM paradedb._typmod_cache;
UPDATE paradedb._typmod_cache SET typmod = ARRAY['9', '9'];
DELETE FROM paradedb._typmod_cache;
TRUNCATE paradedb._typmod_cache;
RESET ROLE;
DROP ROLE typmod_cache_lowpriv;
