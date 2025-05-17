-- test stopwords and stopwords_language

\echo 'Test: Stopwords processing'



-- direct stop words list
SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords => ARRAY['stopword']),
        'something, stopword, else'
);


SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'English'),
        'something and else'
);

-- direct stopwords AND stopwords language
SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'English', stopwords => ARRAY['stopword']),
        'stopword and else'
);


-- Use in search
DROP TABLE IF EXISTS test_stopwords CASCADE;
CREATE TABLE test_stopwords
(
    id    serial8 not null primary key,
    name  text
);


insert into test_stopwords (name)
values
    ('something, stopword, else'), -- those two should be equivalent with the index below
    ('something else'),
    ('something more');


CREATE INDEX idx_stopwords_bm25 ON test_stopwords
    USING bm25 (id, name)
    WITH (
    key_field = 'id',
    text_fields ='{
        "name": {"tokenizer": {"type": "default", "stopwords": ["stopword"]}}
    }'
);

-- "something else" and "something, stopword, else" have the same score
SELECT paradedb.score(id) AS score, name
FROM test_stopwords
WHERE name @@@ '("something" "else")'
ORDER BY name;


-- stopword is filtered out hence when trying to search for it, the result is empty
SELECT paradedb.score(id) AS score, name
FROM test_stopwords
WHERE name @@@ 'and'
ORDER BY name;

DROP TABLE test_stopwords CASCADE;


--- Languages support:

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Danish'),
        'ikke æbler'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Dutch'),
        'geen appels'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'English'),
        'no apples'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Finnish'),
        'ei omenoita'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'French'),
        'pas de pommes'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'German'),
        'keine Äpfel'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Hungarian'),
        'nincs alma'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Italian'),
        'non mele'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Norwegian'),
        'ingen epler'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Portuguese'),
        'sem maçãs'
              );


SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Russian'),
        'нет яблок'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Spanish'),
        'sin manzanas'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Swedish'),
        'inte äpplen'
              );
