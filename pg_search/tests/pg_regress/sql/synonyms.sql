\i common/common_setup.sql

-- Query-time synonyms. A set is the rows of pdb.synonym_rules that share a set_name, one
-- Solr-format rule each. pdb.synonym_expansion shows what a set does to a string, with the
-- analyzer the caller passes.

-- A set appears with its first rule.
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES
    ('shop', '# comments and blank lines are skipped'),
    ('shop', ''),
    ('shop', 'couch, sofa, divan'),
    ('shop', 'sneakers => running shoes'),
    ('shop', 'tv, television'),
    ('shop', 'red, crimson'),
    ('shop', 'living room, lounge'),
    ('colors', 'red, crimson, scarlet');

SELECT name, version FROM pdb.synonym_sets ORDER BY name;
SELECT set_name, rule FROM pdb.synonym_rules ORDER BY set_name, rule;

-- A bad rule fails at INSERT, with the rule and the reason, and leaves no set behind.
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES ('bad', 'a => b => c');
SELECT count(*) FROM pdb.synonym_sets WHERE name = 'bad';

-- 'none' is the value that turns expansion off, so it cannot be the name of a set.
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES ('none', 'a, b');

-------------------------------------------------------------
-- pdb.synonym_expansion: each run of tokens with its alternatives, the typed one included
-------------------------------------------------------------

SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'red sofa and sneakers');
-- A directed rule replaces its left side. The other direction does not expand.
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'running shoes');
-- The longest key wins, so a two-word key is one position.
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'living room');
-- A missing set fails with its name.
SELECT * FROM pdb.synonym_expansion('nope', paradedb.tokenizer('default'), 'sofa');

-- The rules go through the analyzer, so a stemmer applies to both sides. "sneakers" stems to
-- "sneaker", and "running shoes" to the phrase "run shoe".
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default', stemmer => 'English'), 'sneaker');

-- A right side that the analyzer removes, here a stopword, keeps the typed term.
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES ('stop', 'sofa => the'), ('stop', 'couch => the settee');
SELECT * FROM pdb.synonym_expansion('stop', paradedb.tokenizer('default', stopwords_language => 'English'), 'sofa couch');

-------------------------------------------------------------
-- Reload: a change is visible to its own transaction before commit, and to every query after.
-------------------------------------------------------------

BEGIN;
DELETE FROM pdb.synonym_rules WHERE set_name = 'shop' AND rule = 'couch, sofa, divan';
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES ('shop', 'sofa, settee');
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'sofa');
ROLLBACK;

-- Rolled back: the old rules apply again.
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'sofa');

-- The version moves once per statement, also for a COPY.
SELECT version FROM pdb.synonym_sets WHERE name = 'shop';
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES ('shop', 'sofa, loveseat'), ('shop', 'lamp, light');
SELECT version FROM pdb.synonym_sets WHERE name = 'shop';
COPY pdb.synonym_rules (set_name, rule) FROM stdin;
shop	chair, seat
shop	desk, table
\.
SELECT version FROM pdb.synonym_sets WHERE name = 'shop';
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'sofa');

-------------------------------------------------------------
-- UPDATE: a changed rule, and a rule moved to a set that appears with it.
-------------------------------------------------------------

UPDATE pdb.synonym_rules SET rule = 'tv, television, telly' WHERE set_name = 'shop' AND rule = 'tv, television';
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'telly');
UPDATE pdb.synonym_rules SET set_name = 'screens' WHERE rule = 'tv, television, telly';
SELECT name FROM pdb.synonym_sets WHERE name IN ('screens', 'shop') ORDER BY name;
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'telly');
SELECT * FROM pdb.synonym_expansion('screens', paradedb.tokenizer('default'), 'telly');

-- Deleting all rules leaves an empty set, which expands nothing. DELETE FROM pdb.synonym_sets
-- removes the set with its rules.
DELETE FROM pdb.synonym_rules WHERE set_name = 'screens';
SELECT * FROM pdb.synonym_expansion('screens', paradedb.tokenizer('default'), 'telly');
DELETE FROM pdb.synonym_sets WHERE name = 'screens';
SELECT count(*) FROM pdb.synonym_rules WHERE set_name = 'screens';
SELECT * FROM pdb.synonym_expansion('screens', paradedb.tokenizer('default'), 'telly');

-------------------------------------------------------------
-- A writer needs a grant on pdb.synonym_rules only.
-------------------------------------------------------------

CREATE ROLE synonyms_test_writer;
GRANT INSERT ON pdb.synonym_rules TO synonyms_test_writer;
SET ROLE synonyms_test_writer;
INSERT INTO pdb.synonym_rules (set_name, rule) VALUES ('writer', 'a, b');
SELECT name FROM pdb.synonym_sets WHERE name = 'writer';
RESET ROLE;

-- The trigger functions run as the extension owner, so no other role can hang them on a table.
GRANT CREATE ON SCHEMA public TO synonyms_test_writer;
SET ROLE synonyms_test_writer;
CREATE TABLE mine (set_name text);
CREATE TRIGGER mine_ensure BEFORE INSERT ON mine FOR EACH ROW EXECUTE FUNCTION pdb.ensure_synonym_set();
CREATE TRIGGER mine_bump AFTER INSERT ON mine FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
DROP TABLE mine;
RESET ROLE;
DROP OWNED BY synonyms_test_writer;
DROP ROLE synonyms_test_writer;

-------------------------------------------------------------
-- TRUNCATE moves every set's version, and every backend's copy follows.
-------------------------------------------------------------

SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'sofa');
TRUNCATE pdb.synonym_rules;
SELECT name, version FROM pdb.synonym_sets ORDER BY name;
SELECT * FROM pdb.synonym_expansion('shop', paradedb.tokenizer('default'), 'sofa');

DELETE FROM pdb.synonym_sets;
SELECT count(*) FROM pdb.synonym_rules;
