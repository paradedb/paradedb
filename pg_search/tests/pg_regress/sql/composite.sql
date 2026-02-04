-- Composite Type Tests for pg_search
-- Tests basic functionality, features, and error cases

\i common/composite_setup.sql

------------------------------------------------------------
-- TEST: Basic composite type indexing
------------------------------------------------------------

-- Create composite type
CREATE TYPE product_info AS (
    name TEXT,
    description TEXT,
    price FLOAT
);

-- Create table with composite type in index
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    price FLOAT
);

-- Create index using composite type expression
CREATE INDEX idx_products
ON products
USING bm25 (id, (ROW(name, description, price)::product_info))
WITH (key_field='id');

-- Insert test data
INSERT INTO products (name, description, price) VALUES ('Widget', 'A useful widget', 19.99);
INSERT INTO products (name, description, price) VALUES ('Gadget', 'An amazing gadget', 29.99);
INSERT INTO products (name, description, price) VALUES ('Gizmo', 'A fantastic gizmo', 39.99);

-- Query on composite field - search by name
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM products WHERE id @@@ pdb.parse('name:Widget');
SELECT COUNT(*) FROM products WHERE id @@@ pdb.parse('name:Widget');

-- Query on composite field - search by description
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM products WHERE id @@@ pdb.parse('description:amazing');
SELECT COUNT(*) FROM products WHERE id @@@ pdb.parse('description:amazing');

------------------------------------------------------------
-- TEST: Composite type with more than 32 fields
------------------------------------------------------------

-- Create composite type with 35 text fields
CREATE TYPE large_composite AS (
    field_1 TEXT, field_2 TEXT, field_3 TEXT, field_4 TEXT, field_5 TEXT,
    field_6 TEXT, field_7 TEXT, field_8 TEXT, field_9 TEXT, field_10 TEXT,
    field_11 TEXT, field_12 TEXT, field_13 TEXT, field_14 TEXT, field_15 TEXT,
    field_16 TEXT, field_17 TEXT, field_18 TEXT, field_19 TEXT, field_20 TEXT,
    field_21 TEXT, field_22 TEXT, field_23 TEXT, field_24 TEXT, field_25 TEXT,
    field_26 TEXT, field_27 TEXT, field_28 TEXT, field_29 TEXT, field_30 TEXT,
    field_31 TEXT, field_32 TEXT, field_33 TEXT, field_34 TEXT, field_35 TEXT
);

CREATE TABLE large_table (
    id SERIAL PRIMARY KEY,
    field_1 TEXT, field_2 TEXT, field_3 TEXT, field_4 TEXT, field_5 TEXT,
    field_6 TEXT, field_7 TEXT, field_8 TEXT, field_9 TEXT, field_10 TEXT,
    field_11 TEXT, field_12 TEXT, field_13 TEXT, field_14 TEXT, field_15 TEXT,
    field_16 TEXT, field_17 TEXT, field_18 TEXT, field_19 TEXT, field_20 TEXT,
    field_21 TEXT, field_22 TEXT, field_23 TEXT, field_24 TEXT, field_25 TEXT,
    field_26 TEXT, field_27 TEXT, field_28 TEXT, field_29 TEXT, field_30 TEXT,
    field_31 TEXT, field_32 TEXT, field_33 TEXT, field_34 TEXT, field_35 TEXT
);

CREATE INDEX idx_large ON large_table USING bm25 (id, (ROW(
    field_1, field_2, field_3, field_4, field_5,
    field_6, field_7, field_8, field_9, field_10,
    field_11, field_12, field_13, field_14, field_15,
    field_16, field_17, field_18, field_19, field_20,
    field_21, field_22, field_23, field_24, field_25,
    field_26, field_27, field_28, field_29, field_30,
    field_31, field_32, field_33, field_34, field_35
)::large_composite)) WITH (key_field='id');

INSERT INTO large_table (field_1, field_20, field_35) VALUES ('alpha', 'beta', 'gamma');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM large_table WHERE id @@@ pdb.parse('field_1:alpha');
SELECT COUNT(*) FROM large_table WHERE id @@@ pdb.parse('field_1:alpha');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM large_table WHERE id @@@ pdb.parse('field_35:gamma');
SELECT COUNT(*) FROM large_table WHERE id @@@ pdb.parse('field_35:gamma');

------------------------------------------------------------
-- TEST: Composite type with 100 fields
------------------------------------------------------------

-- Create composite type with 100 text fields (abbreviated for readability)
DO $$
DECLARE
    type_def TEXT := 'CREATE TYPE huge_composite AS (';
    table_def TEXT := 'CREATE TABLE huge_table (id SERIAL PRIMARY KEY';
    index_cols TEXT := '';
BEGIN
    FOR i IN 1..100 LOOP
        type_def := type_def || format('f%s TEXT', lpad(i::text, 3, '0'));
        table_def := table_def || format(', f%s TEXT', lpad(i::text, 3, '0'));
        index_cols := index_cols || format('f%s', lpad(i::text, 3, '0'));
        IF i < 100 THEN
            type_def := type_def || ', ';
            index_cols := index_cols || ', ';
        END IF;
    END LOOP;
    type_def := type_def || ')';
    table_def := table_def || ')';

    EXECUTE type_def;
    EXECUTE table_def;
    EXECUTE format('CREATE INDEX idx_huge ON huge_table USING bm25 (id, (ROW(%s)::huge_composite)) WITH (key_field=''id'')', index_cols);
END $$;

INSERT INTO huge_table (f001, f050, f100) VALUES ('first_field', 'middle_field', 'last_field');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM huge_table WHERE id @@@ pdb.parse('f001:first_field');
SELECT COUNT(*) FROM huge_table WHERE id @@@ pdb.parse('f001:first_field');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM huge_table WHERE id @@@ pdb.parse('f050:middle_field');
SELECT COUNT(*) FROM huge_table WHERE id @@@ pdb.parse('f050:middle_field');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM huge_table WHERE id @@@ pdb.parse('f100:last_field');
SELECT COUNT(*) FROM huge_table WHERE id @@@ pdb.parse('f100:last_field');

------------------------------------------------------------
-- TEST: Anonymous ROW expressions are rejected (ERROR expected)
------------------------------------------------------------

CREATE TABLE anon_test (
    id SERIAL PRIMARY KEY,
    a TEXT,
    b TEXT
);

-- This should fail: anonymous ROW without type cast
\set ON_ERROR_STOP off
CREATE INDEX idx_anon_test ON anon_test USING bm25 (id, ROW(a, b)) WITH (key_field='id');
\set ON_ERROR_STOP on

DROP TABLE anon_test;

------------------------------------------------------------
-- TEST: Nested composites are rejected (ERROR expected)
-- NOTE: Domain over composite test removed because error message contains
-- dynamic OID that varies between runs, causing flaky tests.
------------------------------------------------------------

CREATE TYPE inner_composite AS (
    inner_field TEXT
);

CREATE TYPE outer_composite AS (
    outer_field TEXT,
    nested inner_composite
);

CREATE TABLE nested_test (
    id SERIAL PRIMARY KEY,
    field1 TEXT,
    field2 inner_composite
);

-- This should fail: nested composite
\set ON_ERROR_STOP off
CREATE INDEX idx_nested_test ON nested_test USING bm25 (id, (ROW(field1, field2)::outer_composite)) WITH (key_field='id');
\set ON_ERROR_STOP on

DROP TABLE nested_test;
DROP TYPE outer_composite;
DROP TYPE inner_composite;

------------------------------------------------------------
-- TEST: Domain over composite is rejected (ERROR expected)
-- Uses DO block to mask dynamic OID in error message
------------------------------------------------------------

CREATE TYPE base_composite AS (
    field1 TEXT,
    field2 TEXT
);

CREATE DOMAIN composite_domain AS base_composite;

CREATE TABLE domain_test (
    id SERIAL PRIMARY KEY,
    data composite_domain
);

-- Use function to catch error and return standardized message
CREATE OR REPLACE FUNCTION test_domain_rejection() RETURNS TEXT AS $$
BEGIN
    EXECUTE 'CREATE INDEX idx_domain_test ON domain_test USING bm25 (id, data) WITH (key_field=''id'')';
    RETURN 'UNEXPECTED: No error raised for domain over composite';
EXCEPTION
    WHEN OTHERS THEN
        IF SQLERRM LIKE '%invalid postgres oid%' THEN
            RETURN 'OK: Domain over composite correctly rejected';
        ELSE
            RETURN 'UNEXPECTED ERROR: ' || SQLERRM;
        END IF;
END;
$$ LANGUAGE plpgsql;

SELECT test_domain_rejection() AS domain_test_result;

DROP FUNCTION test_domain_rejection();

DROP TABLE domain_test;
DROP DOMAIN composite_domain;
DROP TYPE base_composite;

------------------------------------------------------------
-- TEST: Duplicate field names between composites rejected (ERROR expected)
------------------------------------------------------------

CREATE TYPE comp_a AS (shared_name TEXT, unique_a TEXT);
CREATE TYPE comp_b AS (shared_name TEXT, unique_b TEXT);

CREATE TABLE dup_comp_test (
    id SERIAL PRIMARY KEY,
    a_field TEXT,
    b_field TEXT,
    c_field TEXT,
    d_field TEXT
);

-- This should fail: 'shared_name' appears in both composites
\set ON_ERROR_STOP off
CREATE INDEX idx_dup_comp ON dup_comp_test USING bm25 (
    id,
    (ROW(a_field, b_field)::comp_a),
    (ROW(c_field, d_field)::comp_b)
) WITH (key_field='id');
\set ON_ERROR_STOP on

DROP TABLE dup_comp_test;
DROP TYPE comp_a;
DROP TYPE comp_b;

------------------------------------------------------------
-- TEST: Duplicate field with regular column rejected (ERROR expected)
------------------------------------------------------------

CREATE TYPE dup_field_comp AS (name TEXT, other TEXT);

CREATE TABLE dup_field_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT
);

-- This should fail: 'name' appears both as column and composite field
\set ON_ERROR_STOP off
CREATE INDEX idx_dup_field ON dup_field_test USING bm25 (
    id,
    name,
    (ROW(name, description)::dup_field_comp)
) WITH (key_field='id');
\set ON_ERROR_STOP on

DROP TABLE dup_field_test;
DROP TYPE dup_field_comp;

------------------------------------------------------------
-- TEST: NULL handling in composite fields
------------------------------------------------------------

CREATE TYPE nullable_comp AS (name TEXT, description TEXT, price FLOAT);

CREATE TABLE nullable_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    price FLOAT
);

CREATE INDEX idx_nullable ON nullable_test USING bm25 (
    id, (ROW(name, description, price)::nullable_comp)
) WITH (key_field='id');

-- Insert with NULL fields
INSERT INTO nullable_test (name, description, price) VALUES ('Product A', 'Has description', 10.00);
INSERT INTO nullable_test (name, description, price) VALUES ('Product B', NULL, 20.00);
INSERT INTO nullable_test (name, description, price) VALUES ('Product C', 'Another desc', NULL);

-- Should find non-NULL fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM nullable_test WHERE id @@@ pdb.parse('name:"Product C"');
SELECT COUNT(*) FROM nullable_test WHERE id @@@ pdb.parse('name:"Product C"');

------------------------------------------------------------
-- TEST: REINDEX with composite types
------------------------------------------------------------

CREATE TYPE reindex_comp AS (name TEXT, description TEXT, price FLOAT);

CREATE TABLE reindex_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    price FLOAT
);

CREATE INDEX idx_reindex ON reindex_test USING bm25 (
    id, (ROW(name, description, price)::reindex_comp)
) WITH (key_field='id');

INSERT INTO reindex_test (name, description, price) VALUES ('Widget', 'A useful widget', 19.99);

-- Perform REINDEX
REINDEX INDEX idx_reindex;

-- Verify data is still searchable after REINDEX
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM reindex_test WHERE id @@@ pdb.parse('name:Widget');
SELECT COUNT(*) FROM reindex_test WHERE id @@@ pdb.parse('name:Widget');

------------------------------------------------------------
-- TEST: Large values in composite fields
------------------------------------------------------------

CREATE TYPE large_val_comp AS (title TEXT, content TEXT, metadata TEXT);

CREATE TABLE large_val_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    content TEXT,
    metadata TEXT
);

CREATE INDEX idx_large_val ON large_val_test USING bm25 (
    id, (ROW(title, content, metadata)::large_val_comp)
) WITH (key_field='id');

-- Insert with large text values
INSERT INTO large_val_test (title, content, metadata) VALUES (
    'Large Document',
    repeat('This is a large content block. ', 100),
    repeat('metadata value ', 50)
);

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM large_val_test WHERE id @@@ pdb.parse('title:Large');
SELECT COUNT(*) FROM large_val_test WHERE id @@@ pdb.parse('title:Large');

SELECT COUNT(*) FROM large_val_test;

------------------------------------------------------------
-- TEST: Full pipeline with complex queries
------------------------------------------------------------

CREATE TYPE full_pipeline_comp AS (name TEXT, description TEXT, category TEXT, tags TEXT);

CREATE TABLE full_pipeline_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    tags TEXT
);

CREATE INDEX idx_full_pipeline ON full_pipeline_test USING bm25 (
    id, (ROW(name, description, category, tags)::full_pipeline_comp)
) WITH (key_field='id');

INSERT INTO full_pipeline_test (name, description, category, tags) VALUES
    ('Laptop', 'Powerful laptop computer', 'Electronics', 'computer tech'),
    ('Mouse', 'Wireless mouse', 'Electronics', 'computer accessories'),
    ('Book', 'Programming guide', 'Books', 'computer education'),
    ('Chair', 'Ergonomic office chair', 'Furniture', 'office comfort'),
    ('Desk', 'Standing desk', 'Furniture', 'office workspace');

-- Test search on each field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('name:Laptop');
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('name:Laptop');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('description:Wireless');
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('description:Wireless');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('category:Electronics');
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('category:Electronics');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('tags:office');
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('tags:office');

-- Test complex queries with OR and AND using pdb.parse with tantivy syntax
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('category:books OR tags:computer');
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('category:books OR tags:computer');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('category:electronics AND tags:accessories');
SELECT COUNT(*) FROM full_pipeline_test WHERE id @@@ pdb.parse('category:electronics AND tags:accessories');

-- Verify total rows
SELECT COUNT(*) FROM full_pipeline_test;

------------------------------------------------------------
-- TEST: Multiple composites with distinct field names
------------------------------------------------------------

CREATE TYPE multi_comp_a AS (title TEXT, body TEXT);
CREATE TYPE multi_comp_b AS (author TEXT, category TEXT);

CREATE TABLE multi_comp_test (
    id SERIAL PRIMARY KEY,
    title TEXT,
    body TEXT,
    author TEXT,
    category TEXT
);

CREATE INDEX idx_multi_comp ON multi_comp_test USING bm25 (
    id,
    (ROW(title, body)::multi_comp_a),
    (ROW(author, category)::multi_comp_b)
) WITH (key_field='id');

INSERT INTO multi_comp_test (title, body, author, category) VALUES
    ('PostgreSQL Guide', 'Learn about databases', 'Alice', 'tech'),
    ('Cooking Tips', 'How to make pasta', 'Bob', 'food');

-- Search works on fields from BOTH composites
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('title:PostgreSQL');
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('title:PostgreSQL');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('body:pasta');
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('body:pasta');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('author:Alice');
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('author:Alice');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('category:food');
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('category:food');

-- Cross-composite search using pdb.parse with AND
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('title:guide AND author:alice');
SELECT COUNT(*) FROM multi_comp_test WHERE id @@@ pdb.parse('title:guide AND author:alice');

------------------------------------------------------------
-- TEST: Complex hybrid index (composite + regular columns)
------------------------------------------------------------

CREATE TYPE hybrid_comp_a AS (description TEXT, notes TEXT);
CREATE TYPE hybrid_comp_b AS (tags TEXT, keywords TEXT);

CREATE TABLE hybrid_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    notes TEXT,
    category TEXT,
    tags TEXT,
    keywords TEXT
);

CREATE INDEX idx_hybrid ON hybrid_test USING bm25 (
    id,
    name,
    (ROW(description, notes)::hybrid_comp_a),
    category,
    (ROW(tags, keywords)::hybrid_comp_b)
) WITH (key_field='id');

INSERT INTO hybrid_test (name, description, notes, category, tags, keywords) VALUES
    ('Widget', 'A useful widget', 'Some notes here', 'tools', 'gadget,useful', 'tool widget'),
    ('Gizmo', 'An amazing gizmo', 'More notes', 'electronics', 'device,tech', 'electronic gizmo');

-- Test regular column
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('name:Widget');
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('name:Widget');

-- Test first composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('description:amazing');
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('description:amazing');

-- Test second regular column
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('category:tools');
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('category:tools');

-- Test second composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('tags:tech');
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('tags:tech');

-- Test cross-type query using pdb.parse with AND
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('name:widget AND description:useful');
SELECT COUNT(*) FROM hybrid_test WHERE id @@@ pdb.parse('name:widget AND description:useful');

------------------------------------------------------------
-- TEST: Mixed expressions (columns + IMMUTABLE functions)
------------------------------------------------------------

CREATE TYPE article_search AS (
    title TEXT,
    body TEXT,
    title_upper TEXT
);

CREATE TABLE articles (
    id SERIAL PRIMARY KEY,
    title TEXT,
    body TEXT,
    created_at TIMESTAMP
);

-- Mix simple columns (title, body) with expression (upper - IMMUTABLE function)
CREATE INDEX idx_articles
ON articles
USING bm25 (
    id,
    (ROW(title, body, upper(title))::article_search)
)
WITH (key_field='id');

INSERT INTO articles (title, body, created_at) VALUES
    ('First Post', 'This is the first post', '2024-01-15'),
    ('Second Post', 'This is the second post', '2024-02-20');

-- Search by title (simple column)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS title_search FROM articles WHERE id @@@ pdb.parse('title:First');
SELECT COUNT(*) AS title_search FROM articles WHERE id @@@ pdb.parse('title:First');

-- Search by title_upper (expression result - uppercase)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS title_upper_search FROM articles WHERE id @@@ pdb.parse('title_upper:FIRST');
SELECT COUNT(*) AS title_upper_search FROM articles WHERE id @@@ pdb.parse('title_upper:FIRST');

------------------------------------------------------------
-- TEST: Mixed data sizes (empty, small, medium, large, NULL)
------------------------------------------------------------

CREATE TYPE mixed_composite AS (
    small TEXT,
    medium TEXT,
    large TEXT
);

CREATE TABLE mixed_data (
    id SERIAL PRIMARY KEY,
    small TEXT,
    medium TEXT,
    large TEXT
);

CREATE INDEX idx_mixed
ON mixed_data
USING bm25 (id, (ROW(small, medium, large)::mixed_composite))
WITH (key_field='id');

-- Test with mixed sizes: empty, small, medium, large, NULL
INSERT INTO mixed_data (small, medium, large) VALUES
    ('', 'medium text here', NULL),
    ('tiny', NULL, 'this is a much longer text that goes on and on'),
    (NULL, NULL, NULL);

-- Verify all rows were indexed
SELECT COUNT(*) AS mixed_total FROM mixed_data;

-- Verify searchable non-NULL values
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS found_medium FROM mixed_data WHERE id @@@ pdb.parse('medium:medium');
SELECT COUNT(*) AS found_medium FROM mixed_data WHERE id @@@ pdb.parse('medium:medium');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS found_tiny FROM mixed_data WHERE id @@@ pdb.parse('small:tiny');
SELECT COUNT(*) AS found_tiny FROM mixed_data WHERE id @@@ pdb.parse('small:tiny');

------------------------------------------------------------
-- TEST: Composite fields exist in index schema
------------------------------------------------------------

CREATE TYPE verify_composite AS (
    first_field TEXT,
    second_field TEXT
);

CREATE TABLE verify_table (
    id SERIAL PRIMARY KEY,
    first_field TEXT,
    second_field TEXT
);

CREATE INDEX verify_idx
ON verify_table
USING bm25 (id, (ROW(first_field, second_field)::verify_composite))
WITH (key_field='id');

-- Verify the composite fields exist in the index schema
SELECT EXISTS (SELECT 1 FROM paradedb.schema('verify_idx') WHERE name = 'first_field') AS first_field_exists;
SELECT EXISTS (SELECT 1 FROM paradedb.schema('verify_idx') WHERE name = 'second_field') AS second_field_exists;

-- Verify they work by indexing and searching
INSERT INTO verify_table (first_field, second_field) VALUES ('hello', 'world');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS first_field_search FROM verify_table WHERE id @@@ pdb.parse('first_field:hello');
SELECT COUNT(*) AS first_field_search FROM verify_table WHERE id @@@ pdb.parse('first_field:hello');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS second_field_search FROM verify_table WHERE id @@@ pdb.parse('second_field:world');
SELECT COUNT(*) AS second_field_search FROM verify_table WHERE id @@@ pdb.parse('second_field:world');

------------------------------------------------------------
-- TEST: Comprehensive schema verification with search
------------------------------------------------------------

CREATE TYPE product_schema AS (
    product_name TEXT,
    product_desc TEXT,
    product_price FLOAT
);

CREATE TABLE products_schema (
    id SERIAL PRIMARY KEY,
    product_name TEXT,
    product_desc TEXT,
    product_price FLOAT
);

CREATE INDEX idx_products_schema
ON products_schema
USING bm25 (id, (ROW(product_name, product_desc, product_price)::product_schema))
WITH (key_field='id');

-- Query the index schema to verify composite fields exist
SELECT EXISTS (
    SELECT 1 FROM paradedb.schema('idx_products_schema')
    WHERE name IN ('product_name', 'product_desc', 'product_price')
) AS fields_exist;

-- Verify all three fields are present
SELECT COUNT(*) AS field_count FROM paradedb.schema('idx_products_schema')
WHERE name IN ('product_name', 'product_desc', 'product_price');

-- Insert data and verify it's searchable on each field
INSERT INTO products_schema (product_name, product_desc, product_price) VALUES ('TestProduct', 'TestDescription', 99.99);

-- Search each field to prove it was indexed
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS name_search FROM products_schema WHERE id @@@ pdb.parse('product_name:TestProduct');
SELECT COUNT(*) AS name_search FROM products_schema WHERE id @@@ pdb.parse('product_name:TestProduct');

EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS desc_search FROM products_schema WHERE id @@@ pdb.parse('product_desc:TestDescription');
SELECT COUNT(*) AS desc_search FROM products_schema WHERE id @@@ pdb.parse('product_desc:TestDescription');

------------------------------------------------------------
-- TEST: Tokenizer types in composite fields (pdb.simple)
------------------------------------------------------------

CREATE TYPE tokenized_fields AS (
    title TEXT,
    title_simple pdb.simple
);

CREATE TABLE tokenized_test (
    id SERIAL PRIMARY KEY,
    title TEXT
);

CREATE INDEX idx_tokenized ON tokenized_test USING bm25 (
    id,
    (ROW(title, title)::tokenized_fields)
) WITH (key_field='id');

-- Validate that pdb.simple tokenizer was applied (should show 'default' tokenizer for title_simple)
SELECT * FROM paradedb.schema('idx_tokenized') ORDER BY name;

INSERT INTO tokenized_test (title) VALUES ('Running and Jumping');

-- Search on the simple tokenizer field (lowercased)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS simple_tokenizer_search FROM tokenized_test WHERE id @@@ pdb.parse('title_simple:running');
SELECT COUNT(*) AS simple_tokenizer_search FROM tokenized_test WHERE id @@@ pdb.parse('title_simple:running');

-- Search on the default text field (should also find it)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS default_text_search FROM tokenized_test WHERE id @@@ pdb.parse('title:running');
SELECT COUNT(*) AS default_text_search FROM tokenized_test WHERE id @@@ pdb.parse('title:running');

------------------------------------------------------------
-- TEST: Ngram tokenizer in composite fields
------------------------------------------------------------

CREATE TYPE ngram_fields AS (
    content TEXT,
    content_ngram pdb.ngram(2, 4)
);

CREATE TABLE ngram_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

CREATE INDEX idx_ngram ON ngram_test USING bm25 (
    id,
    (ROW(content, content)::ngram_fields)
) WITH (key_field='id');

-- Validate that ngram tokenizer was applied (should show 'ngram_mingram:2_maxgram:4...' for content_ngram)
SELECT * FROM paradedb.schema('idx_ngram') ORDER BY name;

INSERT INTO ngram_test (content) VALUES ('PostgreSQL database');

-- Search with partial match via ngram - 'gres' should match 'PostgreSQL'
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS ngram_partial_search FROM ngram_test WHERE id @@@ pdb.parse('content_ngram:gres');
SELECT COUNT(*) AS ngram_partial_search FROM ngram_test WHERE id @@@ pdb.parse('content_ngram:gres');

-- Default text field should NOT match partial 'gres' (no ngram)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS default_no_partial FROM ngram_test WHERE id @@@ pdb.parse('content:gres');
SELECT COUNT(*) AS default_no_partial FROM ngram_test WHERE id @@@ pdb.parse('content:gres');

------------------------------------------------------------
-- TEST: Stemmer tokenizer in composite fields
------------------------------------------------------------

CREATE TYPE stemmer_fields AS (
    content TEXT,
    content_stemmed pdb.simple('stemmer=english')
);

CREATE TABLE stemmer_test (
    id SERIAL PRIMARY KEY,
    content TEXT
);

CREATE INDEX idx_stemmer ON stemmer_test USING bm25 (
    id,
    (ROW(content, content)::stemmer_fields)
) WITH (key_field='id');

-- Validate that stemmer tokenizer was applied (should show 'default[stemmer=English]' for content_stemmed)
SELECT * FROM paradedb.schema('idx_stemmer') ORDER BY name;

INSERT INTO stemmer_test (content) VALUES
    ('running quickly'),
    ('he runs fast');

-- Stemmed search: 'run' should match 'running' and 'runs' (both stem to 'run')
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS stemmer_search FROM stemmer_test WHERE id @@@ pdb.parse('content_stemmed:run');
SELECT COUNT(*) AS stemmer_search FROM stemmer_test WHERE id @@@ pdb.parse('content_stemmed:run');

-- Default text field should NOT match 'run' (no stemming)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) AS default_no_stem FROM stemmer_test WHERE id @@@ pdb.parse('content:run');
SELECT COUNT(*) AS default_no_stem FROM stemmer_test WHERE id @@@ pdb.parse('content:run');

------------------------------------------------------------
-- SMOKE TESTS: Comprehensive feature coverage
------------------------------------------------------------

-- Create composite type for smoke tests (using pdb tokenizer types)
CREATE TYPE smoke_product AS (
    name TEXT,
    description TEXT,
    category pdb.literal  -- keyword tokenizer with fast field
);

-- Create table with columns that will be indexed via composite type
CREATE TABLE smoke_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    description TEXT,
    category TEXT,
    rating FLOAT,
    price FLOAT,
    in_stock BOOLEAN
);

-- Insert test data
INSERT INTO smoke_test (name, description, category, rating, price, in_stock) VALUES
    ('Running Shoes', 'Lightweight running shoes for athletes', 'Footwear', 4.5, 89.99, true),
    ('Wireless Keyboard', 'Ergonomic wireless keyboard with backlight', 'Electronics', 4.2, 79.99, true),
    ('Gaming Mouse', 'High precision gaming mouse with RGB', 'Electronics', 4.8, 59.99, true),
    ('Yoga Mat', 'Non-slip yoga mat for exercise', 'Sports', 4.0, 29.99, true),
    ('Coffee Maker', 'Automatic drip coffee maker', 'Kitchen', 3.9, 49.99, false),
    ('Hiking Boots', 'Waterproof hiking boots for trails', 'Footwear', 4.6, 129.99, true),
    ('Bluetooth Speaker', 'Portable bluetooth speaker waterproof', 'Electronics', 4.3, 39.99, true),
    ('Tennis Racket', 'Professional tennis racket lightweight', 'Sports', 4.1, 149.99, false);

-- Create BM25 index on composite type
-- FLOAT fields (rating, price) are automatically fast
-- category uses pdb.literal (keyword tokenizer with fast) defined in composite type
CREATE INDEX smoke_idx ON smoke_test
USING bm25 (id, (ROW(name, description, category)::smoke_product), rating, price)
WITH (key_field = 'id');

------------------------------------------------------------
-- TEST: Scoring with pdb.score()
------------------------------------------------------------
\echo '=== TEST: Scoring ==='

-- Basic score (tie-breaker on id)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes')
ORDER BY score DESC, id;
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes')
ORDER BY score DESC, id;

-- Score with multiple matches (tie-breaker on id)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:keyboard')
ORDER BY score DESC, id;
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:keyboard')
ORDER BY score DESC, id;

------------------------------------------------------------
-- TEST: Snippets with pdb.snippet()
------------------------------------------------------------
\echo '=== TEST: Snippets ==='

-- Snippet on field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippet(description) as snippet
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes')
ORDER BY id;
SELECT id, pdb.snippet(description) as snippet
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes')
ORDER BY id;

-- Snippet with custom tags
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippet(description, start_tag => '**', end_tag => '**') as snippet
FROM smoke_test
WHERE id @@@ pdb.parse('description:wireless')
ORDER BY id;
SELECT id, pdb.snippet(description, start_tag => '**', end_tag => '**') as snippet
FROM smoke_test
WHERE id @@@ pdb.parse('description:wireless')
ORDER BY id;

------------------------------------------------------------
-- TEST: TopN Scan (ORDER BY with LIMIT)
------------------------------------------------------------
\echo '=== TEST: TopN Scan ==='

-- Basic TopN with EXPLAIN
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating
FROM smoke_test
WHERE id @@@ pdb.parse('name:Shoes OR name:Keyboard OR name:Mouse')
ORDER BY rating DESC, id
LIMIT 3;

-- Execute TopN query
SELECT id, name, rating
FROM smoke_test
WHERE id @@@ pdb.parse('name:Shoes OR name:Keyboard OR name:Mouse')
ORDER BY rating DESC, id
LIMIT 3;

-- TopN with score ordering
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:keyboard')
ORDER BY score DESC, id
LIMIT 2;
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:keyboard')
ORDER BY score DESC, id
LIMIT 2;

------------------------------------------------------------
-- TEST: Aggregates
------------------------------------------------------------
\echo '=== TEST: Aggregates ==='

-- COUNT
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) as total FROM smoke_test WHERE id @@@ pdb.parse('category:Electronics');
SELECT COUNT(*) as total FROM smoke_test WHERE id @@@ pdb.parse('category:Electronics');

-- SUM
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT SUM(price) as total_price FROM smoke_test WHERE id @@@ pdb.parse('category:Electronics');
SELECT SUM(price) as total_price FROM smoke_test WHERE id @@@ pdb.parse('category:Electronics');

-- AVG
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT AVG(rating) as avg_rating FROM smoke_test WHERE id @@@ pdb.parse('description:shoes OR description:boots');
SELECT AVG(rating) as avg_rating FROM smoke_test WHERE id @@@ pdb.parse('description:shoes OR description:boots');

-- Multiple aggregates
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) as count, SUM(price) as total, AVG(rating) as avg_rating, MIN(price) as min_price, MAX(price) as max_price
FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics OR category:Footwear');
SELECT COUNT(*) as count, SUM(price) as total, AVG(rating) as avg_rating, MIN(price) as min_price, MAX(price) as max_price
FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics OR category:Footwear');

------------------------------------------------------------
-- TEST: GROUP BY with Aggregates
------------------------------------------------------------
\echo '=== TEST: GROUP BY ==='

-- GROUP BY on field (tie-breaker on category)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT category, COUNT(*) as count, AVG(rating) as avg_rating
FROM smoke_test
WHERE id @@@ pdb.all()
GROUP BY category
ORDER BY count DESC, category;
SELECT category, COUNT(*) as count, AVG(rating) as avg_rating
FROM smoke_test
WHERE id @@@ pdb.all()
GROUP BY category
ORDER BY count DESC, category;

------------------------------------------------------------
-- TEST: pdb.agg() Window Function
------------------------------------------------------------
\echo '=== TEST: pdb.agg() Window Function ==='

-- pdb.agg with terms aggregation (requires TopN query)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, category,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as category_facets
FROM smoke_test
WHERE id @@@ pdb.all()
ORDER BY id
LIMIT 3;
SELECT id, name, category,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as category_facets
FROM smoke_test
WHERE id @@@ pdb.all()
ORDER BY id
LIMIT 3;

-- pdb.agg with avg aggregation (requires TopN query)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating,
       pdb.agg('{"avg": {"field": "rating"}}'::jsonb) OVER () as avg_rating
FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics')
ORDER BY id
LIMIT 3;
SELECT id, name, rating,
       pdb.agg('{"avg": {"field": "rating"}}'::jsonb) OVER () as avg_rating
FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics')
ORDER BY id
LIMIT 3;

-- pdb.agg with stats aggregation
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, price,
       pdb.agg('{"stats": {"field": "price"}}'::jsonb) OVER () as price_stats
FROM smoke_test
WHERE id @@@ pdb.all()
ORDER BY id
LIMIT 3;
SELECT id, name, price,
       pdb.agg('{"stats": {"field": "price"}}'::jsonb) OVER () as price_stats
FROM smoke_test
WHERE id @@@ pdb.all()
ORDER BY id
LIMIT 3;

------------------------------------------------------------
-- TEST: Range Queries with pdb.range()
------------------------------------------------------------
\echo '=== TEST: Range Queries ==='

-- Range on FLOAT field using PostgreSQL range type
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, price FROM smoke_test
WHERE price @@@ pdb.range(numrange(50.0, 100.0, '[]'))
ORDER BY price, id;
SELECT id, name, price FROM smoke_test
WHERE price @@@ pdb.range(numrange(50.0, 100.0, '[]'))
ORDER BY price, id;

-- Range combined with text search
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:boots') AND rating >= 4.0
ORDER BY rating DESC, id;
SELECT id, name, rating FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:boots') AND rating >= 4.0
ORDER BY rating DESC, id;

------------------------------------------------------------
-- TEST: Fuzzy Search (parse syntax with ~N for edit distance)
------------------------------------------------------------
\echo '=== TEST: Fuzzy Search ==='

-- Fuzzy search using tantivy parse syntax (~ for fuzzy, ~1 for edit distance 1)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('name:runnin~1')
ORDER BY id;
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('name:runnin~1')
ORDER BY id;

-- Fuzzy search with more distance (~2 for edit distance 2)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('name:keyboar~2')
ORDER BY id;
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('name:keyboar~2')
ORDER BY id;

------------------------------------------------------------
-- TEST: Phrase with Slop (parse syntax with ~N for slop)
------------------------------------------------------------
\echo '=== TEST: Phrase with Slop ==='

-- Phrase with slop 0 (exact phrase match) - double quotes for phrase
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('description:"running shoes"')
ORDER BY id;
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('description:"running shoes"')
ORDER BY id;

-- Phrase with slop 2 (allows 2 words between terms) - ~2 after phrase for slop
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('description:"lightweight shoes"~2')
ORDER BY id;
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('description:"lightweight shoes"~2')
ORDER BY id;

------------------------------------------------------------
-- TEST: Mixed Conditions (BM25 + Regular SQL)
------------------------------------------------------------
\echo '=== TEST: Mixed Conditions ==='

-- BM25 search with regular column filter
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating, in_stock
FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics') AND rating > 4.0 AND in_stock = true
ORDER BY rating DESC, id;
SELECT id, name, rating, in_stock
FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics') AND rating > 4.0 AND in_stock = true
ORDER BY rating DESC, id;

-- BM25 search with price range
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, price
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:boots') AND price BETWEEN 50 AND 150
ORDER BY price, id;
SELECT id, name, price
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:boots') AND price BETWEEN 50 AND 150
ORDER BY price, id;

-- BM25 search with IN clause
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, category
FROM smoke_test
WHERE id @@@ pdb.all() AND category IN ('Electronics', 'Footwear')
ORDER BY id;
SELECT id, name, category
FROM smoke_test
WHERE id @@@ pdb.all() AND category IN ('Electronics', 'Footwear')
ORDER BY id;

------------------------------------------------------------
-- TEST: EXPLAIN Plan Verification
------------------------------------------------------------
\echo '=== TEST: EXPLAIN Plans ==='

-- NormalScanExecState / MixedFastFieldExecState - basic search
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE id @@@ pdb.parse('name:Shoes');

-- TopNScanExecState - search with ORDER BY score DESC and LIMIT
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test
WHERE id @@@ pdb.parse('description:shoes OR description:keyboard')
ORDER BY score DESC, id
LIMIT 3;

-- TopNScanExecState - search with ORDER BY fast field and LIMIT
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating
FROM smoke_test
WHERE id @@@ pdb.parse('name:Shoes OR name:Keyboard OR name:Mouse')
ORDER BY rating DESC, id
LIMIT 3;

-- ParadeDB Aggregate Scan - aggregate query with fast fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT COUNT(*) FROM smoke_test
WHERE id @@@ pdb.parse('category:Electronics');

-- Verify TopN with pdb.agg window function
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name,
       pdb.agg('{"terms": {"field": "category"}}'::jsonb) OVER () as facets
FROM smoke_test
WHERE id @@@ pdb.all()
ORDER BY id
LIMIT 3;

------------------------------------------------------------
-- TEST: Field existence with pdb.exists()
------------------------------------------------------------
\echo '=== TEST: Field Existence ==='

-- Check field existence using pdb.exists() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE category @@@ pdb.exists()
ORDER BY id;
SELECT id, name FROM smoke_test
WHERE category @@@ pdb.exists()
ORDER BY id;

------------------------------------------------------------
-- TEST: pdb functions on composite fields
------------------------------------------------------------
\echo '=== TEST: pdb functions on composite fields ==='

-- pdb.term() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE name @@@ pdb.term('running') ORDER BY id;
SELECT id, name FROM smoke_test WHERE name @@@ pdb.term('running') ORDER BY id;

-- pdb.match() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE description @@@ pdb.match('shoes') ORDER BY id;
SELECT id, name FROM smoke_test WHERE description @@@ pdb.match('shoes') ORDER BY id;

-- pdb.regex() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE description @@@ pdb.regex('.*shoes.*') ORDER BY id;
SELECT id, name FROM smoke_test WHERE description @@@ pdb.regex('.*shoes.*') ORDER BY id;

-- pdb.fuzzy_term() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE name @@@ pdb.fuzzy_term('runnin', distance => 1) ORDER BY id;
SELECT id, name FROM smoke_test WHERE name @@@ pdb.fuzzy_term('runnin', distance => 1) ORDER BY id;

-- pdb.phrase() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE description @@@ pdb.phrase('running shoes') ORDER BY id;
SELECT id, name FROM smoke_test WHERE description @@@ pdb.phrase('running shoes') ORDER BY id;

-- pdb.phrase_prefix() on composite field (takes array of terms)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE description @@@ pdb.phrase_prefix(ARRAY['lightweight', 'run']) ORDER BY id;
SELECT id, name FROM smoke_test WHERE description @@@ pdb.phrase_prefix(ARRAY['lightweight', 'run']) ORDER BY id;

-- pdb.range() on FLOAT field (not composite, but verify it works alongside)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating FROM smoke_test WHERE rating @@@ pdb.range(numrange(4.0, 5.0, '[]')) ORDER BY id;
SELECT id, name, rating FROM smoke_test WHERE rating @@@ pdb.range(numrange(4.0, 5.0, '[]')) ORDER BY id;

-- Multiple pdb functions combined with AND
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test
WHERE name @@@ pdb.term('running') AND description @@@ pdb.match('lightweight')
ORDER BY id;
SELECT id, name FROM smoke_test
WHERE name @@@ pdb.term('running') AND description @@@ pdb.match('lightweight')
ORDER BY id;

-- pdb.term() with different composite fields (category uses pdb.literal - case sensitive)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name FROM smoke_test WHERE category @@@ pdb.term('Footwear') ORDER BY id;
SELECT id, name FROM smoke_test WHERE category @@@ pdb.term('Footwear') ORDER BY id;

------------------------------------------------------------
-- TEST: TopN queries with pdb functions on composite fields
------------------------------------------------------------
\echo '=== TEST: TopN with pdb functions on composite fields ==='

-- TopN with pdb.term() on composite field, ORDER BY score
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test WHERE name @@@ pdb.term('shoes')
ORDER BY score DESC, id LIMIT 3;
SELECT id, name, pdb.score(id) as score
FROM smoke_test WHERE name @@@ pdb.term('shoes')
ORDER BY score DESC, id LIMIT 3;

-- TopN with pdb.match() on composite field, ORDER BY rating (fast field)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, rating
FROM smoke_test WHERE description @@@ pdb.match('wireless OR shoes')
ORDER BY rating DESC, id LIMIT 3;
SELECT id, name, rating
FROM smoke_test WHERE description @@@ pdb.match('wireless OR shoes')
ORDER BY rating DESC, id LIMIT 3;

-- TopN with pdb.regex() on composite field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test WHERE description @@@ pdb.regex('.*boot.*')
ORDER BY score DESC, id LIMIT 2;
SELECT id, name, pdb.score(id) as score
FROM smoke_test WHERE description @@@ pdb.regex('.*boot.*')
ORDER BY score DESC, id LIMIT 2;

-- TopN with multiple composite field conditions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, name, pdb.score(id) as score
FROM smoke_test WHERE name @@@ pdb.term('wireless') OR description @@@ pdb.match('keyboard')
ORDER BY score DESC, id LIMIT 2;
SELECT id, name, pdb.score(id) as score
FROM smoke_test WHERE name @@@ pdb.term('wireless') OR description @@@ pdb.match('keyboard')
ORDER BY score DESC, id LIMIT 2;

------------------------------------------------------------
-- TEST: JSON fields in composite types
------------------------------------------------------------

CREATE TYPE json_composite AS (
    metadata JSONB,
    tags TEXT[]
);

CREATE TABLE json_test (
    id SERIAL PRIMARY KEY,
    metadata JSONB,
    tags TEXT[]
);

CREATE INDEX idx_json_composite ON json_test USING bm25 (
    id,
    (ROW(metadata, tags)::json_composite)
) WITH (key_field='id');

-- Validate schema shows JSON field
SELECT * FROM paradedb.schema('idx_json_composite') ORDER BY name;

INSERT INTO json_test (metadata, tags) VALUES
    ('{"title": "PostgreSQL Guide", "author": "John", "year": 2024}', ARRAY['database', 'tutorial']),
    ('{"title": "Search Engine Basics", "author": "Jane", "year": 2023}', ARRAY['search', 'guide']);

-- Search JSON field using key_field with full path
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, metadata FROM json_test WHERE id @@@ pdb.parse('metadata.title:PostgreSQL');
SELECT id, metadata FROM json_test WHERE id @@@ pdb.parse('metadata.title:PostgreSQL');

-- Search JSON field using JSON path operator
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, metadata FROM json_test WHERE metadata->>'title' @@@ 'PostgreSQL';
SELECT id, metadata FROM json_test WHERE metadata->>'title' @@@ 'PostgreSQL';

-- Search JSON nested path using key_field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, metadata FROM json_test WHERE id @@@ pdb.parse('metadata.author:John');
SELECT id, metadata FROM json_test WHERE id @@@ pdb.parse('metadata.author:John');

-- Search JSON nested path using JSON path operator
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, metadata FROM json_test WHERE metadata->>'author' @@@ 'John';
SELECT id, metadata FROM json_test WHERE metadata->>'author' @@@ 'John';

------------------------------------------------------------
-- TEST: Array fields in composite types
------------------------------------------------------------

-- Search array field using key_field
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, tags FROM json_test WHERE id @@@ pdb.parse('tags:database');
SELECT id, tags FROM json_test WHERE id @@@ pdb.parse('tags:database');

-- Search array field using source column
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, tags FROM json_test WHERE tags @@@ pdb.parse('database');
SELECT id, tags FROM json_test WHERE tags @@@ pdb.parse('database');

------------------------------------------------------------
-- TEST: Multiple tokenizers per field using composite types
------------------------------------------------------------

CREATE TYPE multi_tokenizer_composite AS (
    desc_standard TEXT,          -- Standard word tokenization
    desc_ngram pdb.ngram(3, 3)   -- Partial matching with ngram (min=3, max=3)
);

CREATE TABLE multi_tokenizer_test (
    id SERIAL PRIMARY KEY,
    description TEXT
);

CREATE INDEX idx_multi_tokenizer ON multi_tokenizer_test USING bm25 (
    id,
    (ROW(description, description::pdb.ngram(3,3))::multi_tokenizer_composite)
) WITH (key_field='id');

-- Validate schema shows both fields with different tokenizers
SELECT * FROM paradedb.schema('idx_multi_tokenizer') ORDER BY name;

INSERT INTO multi_tokenizer_test (description) VALUES
    ('PostgreSQL is a powerful database'),
    ('MySQL is also popular'),
    ('Search engines use indexing');

-- Search using standard tokenizer via source column (full word match)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description FROM multi_tokenizer_test WHERE description @@@ pdb.parse('powerful');
SELECT id, description FROM multi_tokenizer_test WHERE description @@@ pdb.parse('powerful');

-- Search using ngram tokenizer via key_field (partial match - 'owe' is inside 'powerful')
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description FROM multi_tokenizer_test WHERE id @@@ pdb.parse('desc_ngram:owe');
SELECT id, description FROM multi_tokenizer_test WHERE id @@@ pdb.parse('desc_ngram:owe');

-- Search using ngram tokenizer via tokenizer cast expression (matches ROW arg at position 1)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description FROM multi_tokenizer_test WHERE description::pdb.ngram(3,3) @@@ pdb.parse('owe');
SELECT id, description FROM multi_tokenizer_test WHERE description::pdb.ngram(3,3) @@@ pdb.parse('owe');

-- Same query with simple string shows explicit field name in Tantivy Query (parse_with_field)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description FROM multi_tokenizer_test WHERE description::pdb.ngram(3,3) @@@ 'owe';
SELECT id, description FROM multi_tokenizer_test WHERE description::pdb.ngram(3,3) @@@ 'owe';

------------------------------------------------------------
-- TEST: Non-text expressions in composite fields
------------------------------------------------------------

CREATE TYPE numeric_expr_composite AS (
    original_price FLOAT,
    discounted_price FLOAT
);

CREATE TABLE numeric_expr_test (
    id SERIAL PRIMARY KEY,
    price FLOAT
);

CREATE INDEX idx_numeric_expr ON numeric_expr_test USING bm25 (
    id,
    (ROW(price, price * 0.9)::numeric_expr_composite)
) WITH (key_field='id');

-- Validate schema
SELECT * FROM paradedb.schema('idx_numeric_expr') ORDER BY name;

INSERT INTO numeric_expr_test (price) VALUES (100.00), (200.00), (50.00);

-- Range query on original price (use the expression 'price' which maps to 'original_price' field)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, price FROM numeric_expr_test WHERE price @@@ pdb.range(numrange(75, 150, '[]'));
SELECT id, price FROM numeric_expr_test WHERE price @@@ pdb.range(numrange(75, 150, '[]'));

-- Range query on calculated discounted price (use the expression 'price * 0.9' which maps to 'discounted_price' field)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, price FROM numeric_expr_test WHERE (price * 0.9) @@@ pdb.range(numrange(80, 100, '[]'));
SELECT id, price FROM numeric_expr_test WHERE (price * 0.9) @@@ pdb.range(numrange(80, 100, '[]'));

\i common/composite_cleanup.sql
