-- Custom stopword lists do not yet have a working tokenizer cast equivalent.
\i common/common_setup.sql

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
    USING paradedb (id, name)
    WITH (
    text_fields ='{
        "name": {"tokenizer": {"type": "default", "stopwords": ["stopword"]}}
    }'
);

-- "something else" and "something, stopword, else" have the same score
SELECT pdb.score(id) AS score, name
FROM test_stopwords
WHERE name @@@ pdb.parse_with_field('("something" "else")')
ORDER BY name;


-- stopword is filtered out hence when trying to search for it, the result is empty
SELECT pdb.score(id) AS score, name
FROM test_stopwords
WHERE name ||| 'and'
ORDER BY name;

DROP TABLE test_stopwords CASCADE;
