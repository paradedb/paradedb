\i common/common_setup.sql

DROP TABLE IF EXISTS require_positions;
CREATE TABLE require_positions (
    id serial8 not null primary key,
    t text
);

INSERT INTO require_positions (t) VALUES ('AAABBB');
INSERT INTO require_positions (t) VALUES ('BBBAAA');

CREATE INDEX idxrequire_positions ON require_positions USING bm25 (id, (t::pdb.ngram(3, 3))) WITH (key_field = 'id');

SELECT * FROM require_positions WHERE t ### 'aaa';
SELECT * FROM require_positions WHERE t @@@ ('aaa' ##> 1 ## 'bbb');
SELECT * FROM require_positions WHERE t @@@ pdb.phrase('aaa');
SELECT * FROM require_positions WHERE t @@@ pdb.phrase_prefix(ARRAY['aaa', 'b']);
SELECT * FROM require_positions WHERE t @@@ pdb.regex_phrase(ARRAY['a.*', 'bbb']);

DROP TABLE require_positions;
CREATE TABLE require_positions (
    id serial8 not null primary key,
    t jsonb
);

INSERT INTO require_positions (t) VALUES ('{"key1": "value1", "key2": 3}');
INSERT INTO require_positions (t) VALUES ('{"key1": "value1", "key2": 3}');

CREATE INDEX idxrequire_positions ON require_positions USING bm25 (id, (t::pdb.ngram(3, 3))) WITH (key_field = 'id');

SELECT * FROM require_positions WHERE t->>'key1' ### 'aaa';
SELECT * FROM require_positions WHERE t->>'key1' @@@ ('aaa' ##> 1 ## 'bbb');
SELECT * FROM require_positions WHERE t->>'key1' @@@ pdb.phrase('aaa');
SELECT * FROM require_positions WHERE t->>'key1' @@@ pdb.phrase_prefix(ARRAY['aaa', 'b']);
SELECT * FROM require_positions WHERE t->>'key1' @@@ pdb.regex_phrase(ARRAY['a.*', 'bbb']);

DROP TABLE require_positions;
CREATE TABLE require_positions (
    id serial8 not null primary key,
    t jsonb
);

INSERT INTO require_positions (t) VALUES ('{"key1": "value1", "key2": 3}');
INSERT INTO require_positions (t) VALUES ('{"key1": "value1", "key2": 3}');

CREATE INDEX idxrequire_positions ON require_positions USING bm25 (id, ((t->>'key1')::pdb.ngram(3, 3))) WITH (key_field = 'id');

SELECT * FROM require_positions WHERE ((t->>'key1')::pdb.ngram(3, 3)) ### 'aaa';
SELECT * FROM require_positions WHERE ((t->>'key1')::pdb.ngram(3, 3)) @@@ ('aaa' ##> 1 ## 'bbb');
SELECT * FROM require_positions WHERE ((t->>'key1')::pdb.ngram(3, 3)) @@@ pdb.phrase('aaa');
SELECT * FROM require_positions WHERE ((t->>'key1')::pdb.ngram(3, 3)) @@@ pdb.phrase_prefix(ARRAY['aaa', 'b']);
SELECT * FROM require_positions WHERE ((t->>'key1')::pdb.ngram(3, 3)) @@@ pdb.regex_phrase(ARRAY['a.*', 'bbb']);

DROP TABLE require_positions;
