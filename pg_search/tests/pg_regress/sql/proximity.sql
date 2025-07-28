CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS prox;
CREATE TABLE prox (
    id serial8,
    text text
);
INSERT INTO prox (text) VALUES ('a b c d e f g h i j k l m n o p q r s t u v w x y z');
INSERT INTO prox (text) VALUES ('bbq chicken is delicious');
INSERT INTO prox (text) VALUES ('bbq ribs are delicious');
INSERT INTO prox (text) VALUES ('chicken will be served at the bbq party');
INSERT INTO prox (text) VALUES ('ribs will be served at the party bbq');
CREATE INDEX idxprox ON prox USING bm25 (id, text) WITH (key_field = 'id');

-- no match
SELECT * FROM prox WHERE text @@@ pdb.proximity('a', 23, 'z');
-- match
SELECT * FROM prox WHERE text @@@ pdb.proximity('a', 24, 'z');

-- no match
SELECT * FROM prox WHERE text @@@ pdb.proximity_in_order('delicious', 2, pdb.prox_array('bbq', 'chicken'));
-- match
SELECT * FROM prox WHERE text @@@ pdb.proximity('delicious', 2, pdb.prox_array('bbq', 'chicken'));
SELECT * FROM prox WHERE text @@@ pdb.proximity_in_order(pdb.prox_array('bbq', 'chicken'), 2, 'delicious');

-- match
SELECT * FROM prox WHERE text @@@ pdb.proximity(pdb.prox_clause(pdb.prox_array('chicken', 'ribs'), 0, 'will'), 4, pdb.prox_clause('bbq', 0, 'party'));
SELECT * FROM prox WHERE text @@@ pdb.proximity_in_order(pdb.prox_clause(pdb.prox_array('chicken', 'ribs'), 0, 'will'), 4, pdb.prox_clause('bbq', 0, 'party'));

-- match
SELECT * FROM prox WHERE text @@@ pdb.proximity(pdb.prox_regex('del...ous'), 1, pdb.prox_array('chicken', pdb.prox_regex('r..s')));
-- no match
SELECT * FROM prox WHERE text @@@ pdb.proximity_in_order(pdb.prox_regex('del...ous'), 1, pdb.prox_array('chicken', pdb.prox_regex('r..s')));


--
-- just to assert the json representation
--
select pdb.proximity('a', 42, 'b');
select pdb.proximity_in_order('a', 42, 'b');
select pdb.prox_term('the_term');
select pdb.prox_regex('the_pattern');
select pdb.prox_regex('the_pattern', 100);
select pdb.prox_array('a', 'b', 'c', pdb.prox_term('d'), pdb.prox_regex('e'));
select pdb.prox_clause('a', 42, 'b');


--
-- use the ~~~ operator
--

SELECT paradedb.snippet(text) FROM prox WHERE text @@@ ('a' ~~~ 24 ~~~ 'z');   -- match
SELECT paradedb.snippet(text) FROM prox WHERE text @@@ ('a' ~~~ 3 ~~~ 'c' ~~~ 2 ~~~ 'g');   -- no match
SELECT paradedb.snippet(text) FROM prox WHERE text @@@ ('a' ~~~ 3 ~~~ 'c' ~~~ 3 ~~~ 'g');   -- match
SELECT paradedb.snippet(text) FROM prox WHERE text @@@ (ARRAY['a', 'b', 'c'] ~~~ 1 ~~~ 'd');   -- match
SELECT paradedb.snippet(text) FROM prox WHERE text @@@ ('a' ~~~ 1 ~~~ ARRAY['b', 'c', 'd']);   -- match