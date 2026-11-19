-- A phrase keeps the positions that the tokenizer gave its tokens.

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS phrase_notes, phrase_names, phrase_names_st, phrase_synonyms;

-- A stop word leaves a gap, so the phrase finds one word between `sofa` and `leather`.
CREATE TABLE phrase_notes (id int PRIMARY KEY, note text);
INSERT INTO phrase_notes VALUES
    (1, 'sofa of leather'),
    (2, 'sofa leather'),
    (3, 'sofa red leather');
CREATE INDEX phrase_notes_idx ON phrase_notes
    USING bm25 (id, (note::pdb.simple('stopwords_language=English')));
SELECT id FROM phrase_notes WHERE note ### 'sofa of leather' ORDER BY id;
SELECT id FROM phrase_notes
WHERE note @@@ pdb.phrase(ARRAY['sofa', 'of', 'leather']) ORDER BY id;
DROP TABLE phrase_notes;

-- The grams of one word have one position.
CREATE TABLE phrase_names (id int PRIMARY KEY, name text);
INSERT INTO phrase_names VALUES (1, 'sofa'), (2, 'couch'), (3, 'soda');
CREATE INDEX phrase_names_idx ON phrase_names USING bm25 (id, (name::pdb.edge_ngram(2, 5)));
SELECT id FROM phrase_names WHERE name ### 'sofa' ORDER BY id;

-- A synonym of several tokens is a phrase with one token at each position, so the grams of a
-- word can't be one.
CREATE TABLE phrase_synonyms (rule pdb.synonym_rule NOT NULL);
INSERT INTO phrase_synonyms VALUES ('sofa, couch');
SET paradedb.planner_warnings = off;
SELECT id FROM phrase_names
WHERE name ||| (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms);
SELECT id FROM phrase_names
WHERE name ### (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms);
SELECT id FROM phrase_names
WHERE name @@@ (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms);

-- A search tokenizer that keeps the word whole expands on the same field.
CREATE TABLE phrase_names_st (id int PRIMARY KEY, name text);
INSERT INTO phrase_names_st SELECT * FROM phrase_names;
CREATE INDEX phrase_names_st_idx ON phrase_names_st
    USING bm25 (id, (name::pdb.edge_ngram(2, 5))) WITH (search_tokenizer = 'simple');
SELECT id FROM phrase_names_st
WHERE name ||| (SELECT pdb.expand('sofa', rule) FROM phrase_synonyms) ORDER BY id;
RESET paradedb.planner_warnings;

DROP TABLE phrase_names, phrase_names_st, phrase_synonyms;
