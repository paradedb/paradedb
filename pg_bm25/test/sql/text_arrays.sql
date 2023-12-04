CREATE TABLE example_table (
    id SERIAL PRIMARY KEY,
    text_array TEXT[],
    varchar_array VARCHAR[]
);

INSERT INTO example_table (text_array, varchar_array) VALUES 
('{"text1", "text2", "text3"}', '{"vtext1", "vtext2"}'),
('{"another", "array", "of", "texts"}', '{"vtext3", "vtext4", "vtext5"}'),
('{"single element"}', '{"single varchar element"}');

CREATE INDEX ON example_table
USING bm25 ((example_table.*))
WITH (key_field='id', text_fields='{text_array: {}, varchar_array: {}}');

SELECT * FROM example_table WHERE example_table @@@ 'text_array:text1';
SELECT * FROM example_table WHERE example_table @@@ 'text_array:"single element"';
SELECT * FROM example_table WHERE example_table @@@ 'varchar_array:varchar OR text_array:array';
