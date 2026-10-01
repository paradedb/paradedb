-- Two regex tokenizers with different patterns in one index each keep their own pattern.
-- Both used to register under the same name, so one field was indexed with the other's pattern.
DROP TABLE IF EXISTS regex_patterns;

CREATE TABLE regex_patterns
(
    id serial8 not null primary key,
    t  text
);

INSERT INTO regex_patterns (t)
VALUES ('abc123'), ('def'), ('456');

CREATE INDEX idx_regex_patterns ON regex_patterns USING paradedb
    (
     id,
     (t::pdb.regex_pattern('[0-9]+', 'alias=digits')),
     (t::pdb.regex_pattern('[a-z]+', 'alias=letters'))
        );

SELECT name, tokenizer FROM paradedb.schema('idx_regex_patterns') WHERE name IN ('digits', 'letters') ORDER BY name;

SELECT id, t FROM regex_patterns WHERE (t::pdb.regex_pattern('[0-9]+', 'alias=digits')) === '123' ORDER BY id;
SELECT id, t FROM regex_patterns WHERE (t::pdb.regex_pattern('[0-9]+', 'alias=digits')) === '456' ORDER BY id;
SELECT id, t FROM regex_patterns WHERE (t::pdb.regex_pattern('[a-z]+', 'alias=letters')) === 'abc' ORDER BY id;
SELECT id, t FROM regex_patterns WHERE (t::pdb.regex_pattern('[a-z]+', 'alias=letters')) === 'def' ORDER BY id;

DROP TABLE regex_patterns;
