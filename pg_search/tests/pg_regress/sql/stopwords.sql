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
        paradedb.tokenizer('default', stopwords_language => 'Czech'),
        'bez jablek'
              );

SELECT * FROM paradedb.tokenize(
        paradedb.tokenizer('default', stopwords_language => 'Polish'),
        'bez jabłek'
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
