-- A synonym index is a paradedb index on a pdb.synonym_rule column.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS shop_synonyms, en_synonyms, bad_synonyms, synonyms CASCADE;

-- The domain parses the Solr rule. A blank line and a comment are lines of a synonym file.
CREATE TABLE shop_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO shop_synonyms VALUES ('a => b => c');
INSERT INTO shop_synonyms VALUES ('sofa, couch, divan');
INSERT INTO shop_synonyms VALUES
    ('sneakers => running shoes'),
    ('running shoes, trainers'),
    ('# a comment'),
    ('');

CREATE INDEX shop_synonyms_idx ON shop_synonyms USING paradedb (rule);

SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'red sofa');
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'new running shoes');
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'Sneakers');

-- A rule change is live on the next query, and a deleted rule stops expanding before VACUUM.
INSERT INTO shop_synonyms VALUES ('red, crimson');
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'red sofa');
DELETE FROM shop_synonyms WHERE rule = 'sofa, couch, divan';
UPDATE shop_synonyms SET rule = 'red, scarlet' WHERE rule = 'red, crimson';
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'red sofa');
BEGIN;
INSERT INTO shop_synonyms VALUES ('sofa, settee');
ROLLBACK;
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'sofa');

-- VACUUM marks the deletes in the index, and REINDEX rebuilds it from the rows.
VACUUM shop_synonyms;
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'red sofa');
REINDEX INDEX shop_synonyms_idx;
SELECT * FROM pdb.synonym_expansion('shop_synonyms_idx', 'red sofa');

-- Rules go through the index tokenizer, and a term that loses a word to it fails the insert.
CREATE TABLE en_synonyms (rule pdb.synonym_rule NOT NULL);
CREATE INDEX en_synonyms_idx ON en_synonyms
    USING paradedb ((rule::pdb.simple('stemmer=english', 'stopwords_language=english')));
INSERT INTO en_synonyms VALUES ('Couches, sofas');
INSERT INTO en_synonyms VALUES ('the who => band');
SELECT * FROM pdb.synonym_expansion('en_synonyms_idx', 'the couch');

-- The same check fails the build.
CREATE TABLE bad_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO bad_synonyms VALUES ('rock of ages, rock');
CREATE INDEX bad_synonyms_idx ON bad_synonyms
    USING paradedb ((rule::pdb.simple('stopwords_language=english')));

-- One table holds several sets, with a partial index per set. Synonym indexes do not count
-- toward the one ParadeDB index of a table, and a scan of the table never uses one.
CREATE TABLE synonyms (set_name text NOT NULL, rule pdb.synonym_rule NOT NULL);
INSERT INTO synonyms VALUES ('shop', 'sofa, couch'), ('colors', 'red, crimson');
CREATE INDEX set_name_idx ON synonyms USING paradedb (set_name);
CREATE INDEX shop_idx ON synonyms USING paradedb (rule) WHERE set_name = 'shop';
CREATE INDEX colors_idx ON synonyms USING paradedb (rule) WHERE set_name = 'colors';
SELECT * FROM pdb.synonym_expansion('shop_idx', 'red sofa');
SELECT * FROM pdb.synonym_expansion('colors_idx', 'red sofa');
CREATE INDEX second_set_name_idx ON synonyms USING paradedb (set_name);
EXPLAIN (COSTS OFF) SELECT set_name FROM synonyms WHERE set_name @@@ 'shop';
SELECT set_name FROM synonyms WHERE set_name @@@ 'shop';

-- The function needs a synonym index.
SELECT * FROM pdb.synonym_expansion('set_name_idx', 'red');

DROP TABLE shop_synonyms, en_synonyms, bad_synonyms, synonyms;
