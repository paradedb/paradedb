DROP TABLE IF EXISTS stopwords_lang;
CREATE TABLE stopwords_lang(
    id serial8 not null primary key,
    t text
);

CREATE INDEX idxstopwords_lang ON stopwords_lang USING bm25 (id, (t::pdb.simple('stopwords_language=english'))) WITH (key_field = 'id');

INSERT INTO stopwords_lang (t) VALUES ('how many of these are in the stopwords list?');

SELECT * FROM stopwords_lang WHERE t @@@ 'are in the';  -- runtime tantivy error
SELECT * FROM stopwords_lang WHERE t @@@ 'are in the stopwords list?'; -- finds the row
SELECT * FROM stopwords_lang WHERE t &&& 'are in the';
SELECT * FROM stopwords_lang WHERE t ||| 'are in the';
SELECT * FROM stopwords_lang WHERE t ### 'are in the';
SELECT * FROM stopwords_lang WHERE t === 'are';

SELECT * FROM stopwords_lang WHERE t @@@ 'stopwords list?';
SELECT * FROM stopwords_lang WHERE t &&& 'stopwords list?';
SELECT * FROM stopwords_lang WHERE t ||| 'stopwords list?';
SELECT * FROM stopwords_lang WHERE t ### 'stopwords list?';
SELECT * FROM stopwords_lang WHERE t === 'stopwords';
