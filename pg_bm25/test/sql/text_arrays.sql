CREATE TABLE example_table (
    id SERIAL PRIMARY KEY,
    text_array TEXT[]
);

INSERT INTO example_table (text_array) VALUES 
('{"text1", "text2", "text3"}'),
('{"another", "array", "of", "texts"}'),
('{"single element"}');

CREATE INDEX ON example_table
USING bm25 ((example_table.*))
WITH (text_fields='{text_array: {}}');

SELECT * FROM example_table WHERE example_table @@@ 'text_array:text1';
SELECT * FROM example_table WHERE example_table @@@ 'text_array:"single element"';
