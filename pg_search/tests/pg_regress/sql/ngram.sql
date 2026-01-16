\i common/common_setup.sql

SELECT 'Tokenize me!'::pdb.ngram(3,3,'prefix_only=true')::text[];
SELECT 'Tokenize me!'::pdb.ngram(3,3,'prefix_only=false')::text[];
SELECT 'Tokenize me!'::pdb.ngram(3,3,'positions=true')::text[];
SELECT 'Tokenize me!'::pdb.ngram(3,3,'positions=false')::text[];
SELECT 'Tokenize me!'::pdb.ngram(3,4,'positions=true')::text[];

CREATE TABLE ngram_positions (id serial primary key, description text);
INSERT INTO ngram_positions (description) VALUES ('aaabbb'), ('bbbaaa');
CREATE INDEX ON ngram_positions USING bm25 (id, (description::pdb.ngram(3,3,'positions=true'))) WITH (key_field = id);

SELECT * FROM ngram_positions WHERE description ### ARRAY['aaa', 'aab'];
SELECT * FROM ngram_positions WHERE description ### ARRAY['aab', 'aaa'];
SELECT * FROM ngram_positions WHERE description @@@ ('aaa' ##> 2 ##> 'bbb');
SELECT * FROM ngram_positions WHERE description @@@ ('aaa' ## 2 ## 'bbb');

DROP TABLE ngram_positions;
