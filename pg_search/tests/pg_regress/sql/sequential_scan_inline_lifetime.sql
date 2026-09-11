\i common/common_setup.sql

CREATE TABLE inline_matcher_lifetime (id int PRIMARY KEY, body text, active boolean);
INSERT INTO inline_matcher_lifetime
SELECT id, CASE WHEN id % 2 = 0 THEN 'ALLOWED' ELSE 'DENIED' END, false
FROM generate_series(1, 128) AS id;
INSERT INTO inline_matcher_lifetime VALUES (129, NULL, false);
CREATE INDEX inline_matcher_lifetime_idx ON inline_matcher_lifetime
USING paradedb (
    id,
    ((lower(body) || '_suffix')::pdb.literal('alias=normalized_body')),
    (body::pdb.simple('alias=stemmed_body', 'stemmer=english'))
)
WHERE active;

SET paradedb.enable_custom_scan = off;
SET enable_indexonlyscan = off;
SET enable_bitmapscan = off;
SET plan_cache_mode = force_generic_plan;

-- Reuse inline expression state with a by-reference constant across rows and executions.
PREPARE inline_lifetime_lookup(text) AS
SELECT count(*) FILTER (
    WHERE ((lower(body) || '_suffix') === $1)
          IS DISTINCT FROM (lower(body) || '_suffix' = $1)
) AS mismatches
FROM inline_matcher_lifetime;

BEGIN;
EXECUTE inline_lifetime_lookup('allowed_suffix');
EXECUTE inline_lifetime_lookup('denied_suffix');
COMMIT;

BEGIN;
UPDATE inline_matcher_lifetime SET body = 'ALLOWED' WHERE id = 1;
EXECUTE inline_lifetime_lookup('allowed_suffix');
ROLLBACK;

EXECUTE inline_lifetime_lookup('allowed_suffix');
DISCARD PLANS;
EXECUTE inline_lifetime_lookup('denied_suffix');
DEALLOCATE inline_lifetime_lookup;

-- Keep both cached tokenizers alive across fetches and a subtransaction abort.
BEGIN;
DECLARE inline_lifetime_cursor CURSOR FOR
SELECT id,
       ((lower(body) || '_suffix') === 'allowed_suffix')
           IS DISTINCT FROM (body = 'ALLOWED') AS literal_mismatch,
       ((body::pdb.simple('alias=stemmed_body', 'stemmer=english')) ||| 'allow')
           IS DISTINCT FROM coalesce(body = 'ALLOWED', false) AS stemmer_mismatch
FROM inline_matcher_lifetime;
FETCH FORWARD 2 FROM inline_lifetime_cursor;
SAVEPOINT inline_lifetime_error;
SELECT 1 / 0;
ROLLBACK TO SAVEPOINT inline_lifetime_error;
FETCH FORWARD 3 FROM inline_lifetime_cursor;
MOVE FORWARD 123 FROM inline_lifetime_cursor;
FETCH FORWARD 1 FROM inline_lifetime_cursor;
CLOSE inline_lifetime_cursor;
COMMIT;

RESET plan_cache_mode;
RESET enable_bitmapscan;
RESET enable_indexonlyscan;
RESET paradedb.enable_custom_scan;
DROP TABLE inline_matcher_lifetime;
