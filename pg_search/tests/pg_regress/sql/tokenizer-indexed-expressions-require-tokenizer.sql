DROP TABLE IF EXISTS expr;
CREATE TABLE expr
(
    id serial8 not null primary key,
    t  text
);

INSERT INTO expr (t) VALUES ('This is a TEST');
INSERT INTO expr (t) VALUES ('This is also a TEST');
CREATE INDEX idxexpr
    ON expr
        USING bm25 (
                    id,
                    -- will cause an ERROR as it needs to be cast to a tokenizer
                    (lower(t))
            )
    WITH (key_field = 'id');

CREATE INDEX idxexpr
    ON expr
        USING bm25 (
                    id,
                    (lower(t)::pdb.literal)
            )
    WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxexpr') ORDER BY name;

SELECT * FROM expr WHERE lower(t) &&& 'This is a TEST';

SELECT * FROM expr WHERE lower(t) @@@ lower('This is a TEST');  -- returns nothing
SELECT * FROM expr WHERE lower(t) &&& lower('This is a TEST');
SELECT * FROM expr WHERE lower(t) ||| lower('This is a TEST');
SELECT * FROM expr WHERE lower(t) ### lower('This is a TEST');
SELECT * FROM expr WHERE lower(t) === lower('This is a TEST');

SELECT * FROM expr WHERE t @@@ lower('This is a TEST');  -- returns nothing
SELECT * FROM expr WHERE t &&& lower('This is a TEST');
SELECT * FROM expr WHERE t ||| lower('This is a TEST');
SELECT * FROM expr WHERE t ### lower('This is a TEST');
SELECT * FROM expr WHERE t === lower('This is a TEST');