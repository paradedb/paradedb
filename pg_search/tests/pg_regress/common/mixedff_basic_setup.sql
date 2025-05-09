CREATE EXTENSION IF NOT EXISTS pg_search;

-- Disable parallel workers to avoid differences in plans
SET max_parallel_workers_per_gather = 0;
SET enable_indexscan to OFF;

-- Drop any existing test tables from this group
DROP TABLE IF EXISTS mixed_numeric_string_test CASCADE;

-- Create test table for mixed numeric/string testing
CREATE TABLE mixed_numeric_string_test (
    id TEXT PRIMARY KEY,
    numeric_field1 INTEGER NOT NULL,
    numeric_field2 BIGINT NOT NULL,
    string_field1 TEXT NOT NULL,
    string_field2 TEXT NOT NULL,
    string_field3 TEXT NOT NULL,
    content TEXT
);

-- Create index with both numeric and string fast fields
CREATE INDEX mixed_test_search ON mixed_numeric_string_test USING bm25 (
    id,
    numeric_field1,
    numeric_field2,
    string_field1,
    string_field2,
    string_field3,
    content
) WITH (
    key_field = 'id',
    text_fields = '{"string_field1": {"tokenizer": {"type": "default"}, "fast": true}, "string_field2": {"tokenizer": {"type": "default"}, "fast": true}, "string_field3": {"tokenizer": {"type": "default"}, "fast": true}, "content": {"tokenizer": {"type": "default"}}}',
    numeric_fields = '{"numeric_field1": {"fast": true}, "numeric_field2": {"fast": true}}'
);

-- Insert sample data
INSERT INTO mixed_numeric_string_test (id, numeric_field1, numeric_field2, string_field1, string_field2, string_field3, content) VALUES
('mix1', 100, 10000, 'Apple', 'Red', 'Fruit', 'This is a red apple'),
('mix2', 200, 20000, 'Banana', 'Yellow', 'Fruit', 'This is a yellow banana'),
('mix3', 300, 30000, 'Carrot', 'Orange', 'Vegetable', 'This is an orange carrot'),
('mix4', 400, 40000, 'Donut', 'Brown', 'Dessert', 'This is a chocolate donut'),
('mix5', 500, 50000, 'Egg', 'White', 'Protein', 'This is a white egg'); 
