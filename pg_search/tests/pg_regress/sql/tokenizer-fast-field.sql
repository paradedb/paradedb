DROP TABLE IF EXISTS tokenizer_fast;
CREATE TABLE tokenizer_fast (
    id serial8 not null primary key,
    t text
);

INSERT INTO tokenizer_fast (t) VALUES ('This is a TEST');

CREATE INDEX idxtokenizer_fast ON tokenizer_fast USING bm25 (id, (t::pdb.literal)) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxtokenizer_fast') ORDER BY name;
SELECT * FROM tokenizer_fast WHERE t &&& 'This is a TEST';