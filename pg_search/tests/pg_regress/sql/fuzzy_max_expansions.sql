-- Tests the `max_expansions` argument of `pdb.fuzzy`, which caps how many terms each fuzzy
-- token expands to, keeping the terms with the smallest edit distance.
\i common/common_setup.sql

CREATE TABLE fuzzy_max_expansions (id SERIAL PRIMARY KEY, word TEXT);
-- Edit distance from `japan`: japan 0, jaban 1, japon 1, jason 2, korea > 2
INSERT INTO fuzzy_max_expansions (word) VALUES ('japan'), ('japon'), ('jaban'), ('jason'), ('korea');
-- 72 distinct terms are within distance 1 of `qqq`: `qq` + any of 36 characters, and `qqq` + any
-- of 36 characters.
INSERT INTO fuzzy_max_expansions (word)
SELECT prefix || c
FROM unnest(ARRAY['qq', 'qqq']) AS prefix,
     regexp_split_to_table('abcdefghijklmnopqrstuvwxyz0123456789', '') AS c;

-- One segment, as the cap applies to each segment's terms.
CREATE INDEX fuzzy_max_expansions_idx ON fuzzy_max_expansions
USING paradedb (id, word) WITH (target_segment_count = 1);

--
-- the typmod round-trips the 4th argument
--
SELECT 'japan'::pdb.fuzzy(2, f, f, 3);
SELECT 'japan'::pdb.fuzzy(2, f, f);
SELECT 'japan'::pdb.fuzzy(2, f, f, -1);

--
-- -1: no limit, every term within distance 2
--
SELECT id, word FROM fuzzy_max_expansions WHERE word === 'japan'::pdb.fuzzy(2, f, f, -1) ORDER BY id;

--
-- the closest terms win; ties go to the earlier term (`jaban` before `japon`)
--
SELECT id, word FROM fuzzy_max_expansions WHERE word === 'japan'::pdb.fuzzy(2, f, f, 1) ORDER BY id;
SELECT id, word FROM fuzzy_max_expansions WHERE word === 'japan'::pdb.fuzzy(2, f, f, 2) ORDER BY id;
SELECT id, word FROM fuzzy_max_expansions WHERE word === 'japan'::pdb.fuzzy(2, f, f, 3) ORDER BY id;
SELECT id, word FROM fuzzy_max_expansions WHERE word === 'japan'::pdb.fuzzy(2, f, f, 50) ORDER BY id;

--
-- match operators cap each token
--
SELECT id, word FROM fuzzy_max_expansions WHERE word ||| 'japan korea'::pdb.fuzzy(2, f, f, 1) ORDER BY id;
SELECT id, word FROM fuzzy_max_expansions WHERE word &&& 'japan'::pdb.fuzzy(2, f, f, 1) ORDER BY id;
SELECT id, word FROM fuzzy_max_expansions WHERE word === ARRAY['japan', 'korea']::pdb.fuzzy(2, f, f, 1) ORDER BY id;

--
-- the query JSON carries the cap
--
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id FROM fuzzy_max_expansions WHERE word ||| 'japan'::pdb.fuzzy(2, f, f, 1);

--
-- a JSON query without `max_expansions` still parses
--
SELECT id, word FROM fuzzy_max_expansions
WHERE id @@@ '{"fuzzy_term": {"field": "word", "value": "japan", "distance": 2}}'::jsonb ORDER BY id;
SELECT id, word FROM fuzzy_max_expansions
WHERE id @@@ '{"fuzzy_term": {"field": "word", "value": "japan", "distance": 2, "max_expansions": 1}}'::jsonb ORDER BY id;

--
-- error conditions
--
SELECT 'japan'::pdb.fuzzy(2, f, f, 0);
SELECT 'japan'::pdb.fuzzy(2, f, f, -2);
SELECT 'japan'::pdb.fuzzy(2, f, f, 134217727);
SELECT 'japan'::pdb.fuzzy(2, f, f, lots);
SELECT 'japan'::pdb.fuzzy(2, f, f, 3, 4);
SELECT id FROM fuzzy_max_expansions WHERE word @@@ 'japan'::pdb.fuzzy(2, f, f, 1);
SELECT id FROM fuzzy_max_expansions
WHERE id @@@ '{"fuzzy_term": {"field": "word", "value": "japan", "distance": 2, "max_expansions": 0}}'::jsonb;

--
-- without max_expansions, a token expands to at most 50 terms
--
-- 72 distinct terms (ids 6 to 77) are within distance 1 of `qqq`, as inserted above.
SELECT count(*) FROM fuzzy_max_expansions WHERE word === 'qqq'::pdb.fuzzy(1);
SELECT count(*) FROM fuzzy_max_expansions WHERE word ||| 'qqq'::pdb.fuzzy(1);
SELECT count(*) FROM fuzzy_max_expansions WHERE word === ARRAY['qqq']::pdb.fuzzy(1);
SELECT count(*) FROM fuzzy_max_expansions
WHERE id @@@ '{"fuzzy_term": {"field": "word", "value": "qqq", "distance": 1}}'::jsonb;
SELECT count(*) FROM fuzzy_max_expansions WHERE word === 'qqq'::pdb.fuzzy(1, f, f, -1);
-- the exact term is always kept
SELECT count(*) FROM fuzzy_max_expansions WHERE word === 'qqq'::pdb.fuzzy(1, f, f, 1) AND word = 'qqq';
-- distance 0 is an exact match, unaffected by the limit
SELECT count(*) FROM fuzzy_max_expansions WHERE word === 'qqq'::pdb.fuzzy(0, f, f, 1);

DROP TABLE fuzzy_max_expansions;
\i common/common_cleanup.sql
