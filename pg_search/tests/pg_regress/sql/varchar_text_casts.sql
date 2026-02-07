\i common/common_setup.sql

-- Test text/varchar casts for tokenizer types and fieldname
SELECT pdb.tokenize_unicode_words('good job'::varchar);
SELECT pdb.tokenize_unicode_words('good job'::text);

DROP TABLE IF EXISTS varchar_text_casts;
CREATE TABLE varchar_text_casts(id int, content text);
INSERT INTO varchar_text_casts VALUES (1, 'a b'), (2, 'a c');

CREATE INDEX varchar_text_casts_idx ON varchar_text_casts
USING bm25 (id, (content::pdb.unicode_words))
WITH (key_field = id);

SELECT id FROM varchar_text_casts
WHERE id @@@ paradedb.phrase('content'::varchar, ARRAY['a', 'b'])
ORDER BY id;

SELECT id FROM varchar_text_casts
WHERE id @@@ paradedb.phrase('content'::text, ARRAY['a', 'b'])
ORDER BY id;

DROP TABLE varchar_text_casts CASCADE;
