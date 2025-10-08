DELETE FROM paradedb._typmod_cache WHERE typmod = ARRAY['66', '77'];
BEGIN;
SELECT 'hello, world'::pdb.ngram(66, 77)::text[];
ABORT;
SELECT 'hello, world'::pdb.ngram(66, 77)::text[];
