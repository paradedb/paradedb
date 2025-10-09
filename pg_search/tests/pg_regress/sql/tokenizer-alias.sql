DROP TABLE IF EXISTS need_alias;
CREATE TABLE need_alias(
    id serial8 not null primary key,
    title text,
    description text
);

INSERT INTO need_alias (title, description) VALUES ('the title', 'the description');

CREATE INDEX idxneed_alias ON need_alias USING bm25 (id, ((title || ' ' || description)::pdb.simple)) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxneed_alias') ORDER BY name;