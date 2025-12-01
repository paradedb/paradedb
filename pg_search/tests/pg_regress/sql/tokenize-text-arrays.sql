\i common/common_setup.sql

DROP TABLE IF EXISTS index_text_array;
CREATE TABLE index_text_array(
    id serial8 not null primary key,
    arr text[]
);
INSERT INTO index_text_array (arr) VALUES (ARRAY['red', 'blue', 'blue green']), (ARRAY['blue green']);
CREATE INDEX idxindex_text_array ON index_text_array USING bm25 (id, arr) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxindex_text_array') ORDER BY name;

SELECT * FROM index_text_array WHERE arr === 'red';
SELECT * FROM index_text_array WHERE arr === 'blue';
SELECT * FROM index_text_array WHERE arr === 'blue green';

DROP INDEX idxindex_text_array;
CREATE INDEX idxindex_text_array ON index_text_array USING bm25 (id, (arr::pdb.literal)) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxindex_text_array') ORDER BY name;

SELECT * FROM index_text_array WHERE arr === 'red';
SELECT * FROM index_text_array WHERE arr === 'blue';
SELECT * FROM index_text_array WHERE arr === 'blue green';

DROP TABLE index_text_array;

CREATE TABLE index_varchar_array(
    id serial8 not null primary key,
    arr varchar[]
);
INSERT INTO index_varchar_array (arr) VALUES (ARRAY['red', 'blue', 'blue green']), (ARRAY['blue green']);
CREATE INDEX idxindex_varchar_array ON index_varchar_array USING bm25 (id, (arr::pdb.literal)) WITH (key_field = 'id');

SELECT * FROM paradedb.schema('idxindex_varchar_array') ORDER BY name;

SELECT * FROM index_varchar_array WHERE arr === 'red';
SELECT * FROM index_varchar_array WHERE arr === 'blue';
SELECT * FROM index_varchar_array WHERE arr === 'blue green';

DROP TABLE index_varchar_array;
