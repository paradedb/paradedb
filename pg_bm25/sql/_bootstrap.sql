GRANT ALL ON SCHEMA paradedb TO PUBLIC;

CREATE OR REPLACE PROCEDURE paradedb.create_bm25_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb')
LANGUAGE plpgsql
AS $$
DECLARE
    full_table_name TEXT := schema_name || '.' || table_name;
    data_to_insert RECORD;
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_tables WHERE schemaname = schema_name AND tablename = table_name) THEN
        EXECUTE 'CREATE TABLE ' || full_table_name || ' (
            id SERIAL PRIMARY KEY,
            description TEXT,
            rating INTEGER CHECK (
                rating BETWEEN 1
                AND 5
            ),
            category VARCHAR(255),
            in_stock BOOLEAN,
            metadata JSONB
        )';

        FOR data_to_insert IN
            SELECT * FROM (VALUES
                ('Ergonomic metal keyboard', 4, 'Electronics', true, '{"color": "Silver", "location": "United States"}'::JSONB),
                ('Plastic Keyboard', 4, 'Electronics', false, '{"color": "Black", "location": "Canada"}'::JSONB),
                ('Sleek running shoes', 5, 'Footwear', true, '{"color": "Blue", "location": "China"}'::JSONB),
                ('White jogging shoes', 3, 'Footwear', false, '{"color": "White", "location": "United States"}'::JSONB),
                ('Generic shoes', 4, 'Footwear', true, '{"color": "Brown", "location": "Canada"}'::JSONB),
                ('Compact digital camera', 5, 'Photography', false, '{"color": "Black", "location": "China"}'::JSONB),
                ('Hardcover book on history', 2, 'Books', true, '{"color": "Brown", "location": "United States"}'::JSONB),
                ('Organic green tea', 3, 'Groceries', true, '{"color": "Green", "location": "Canada"}'::JSONB),
                ('Modern wall clock', 4, 'Home Decor', false, '{"color": "Silver", "location": "China"}'::JSONB),
                ('Colorful kids toy', 1, 'Toys', true, '{"color": "Multicolor", "location": "United States"}'::JSONB),
                ('Soft cotton shirt', 5, 'Apparel', true, '{"color": "Blue", "location": "Canada"}'::JSONB),
                ('Innovative wireless earbuds', 5, 'Electronics', true, '{"color": "Black", "location": "China"}'::JSONB),
                ('Sturdy hiking boots', 4, 'Footwear', true, '{"color": "Brown", "location": "United States"}'::JSONB),
                ('Elegant glass table', 3, 'Furniture', true, '{"color": "Clear", "location": "Canada"}'::JSONB),
                ('Refreshing face wash', 2, 'Beauty', false, '{"color": "White", "location": "China"}'::JSONB),
                ('High-resolution DSLR', 4, 'Photography', true, '{"color": "Black", "location": "United States"}'::JSONB),
                ('Paperback romantic novel', 3, 'Books', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB),
                ('Freshly ground coffee beans', 5, 'Groceries', true, '{"color": "Brown", "location": "China"}'::JSONB),
                ('Artistic ceramic vase', 4, 'Home Decor', false, '{"color": "Multicolor", "location": "United States"}'::JSONB),
                ('Interactive board game', 3, 'Toys', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB),
                ('Slim-fit denim jeans', 5, 'Apparel', false, '{"color": "Blue", "location": "China"}'::JSONB),
                ('Fast charging power bank', 4, 'Electronics', true, '{"color": "Black", "location": "United States"}'::JSONB),
                ('Comfortable slippers', 3, 'Footwear', true, '{"color": "Brown", "location": "Canada"}'::JSONB),
                ('Classic leather sofa', 5, 'Furniture', false, '{"color": "Brown", "location": "China"}'::JSONB),
                ('Anti-aging serum', 4, 'Beauty', true, '{"color": "White", "location": "United States"}'::JSONB),
                ('Portable tripod stand', 4, 'Photography', true, '{"color": "Black", "location": "Canada"}'::JSONB),
                ('Mystery detective novel', 2, 'Books', false, '{"color": "Multicolor", "location": "China"}'::JSONB),
                ('Organic breakfast cereal', 5, 'Groceries', true, '{"color": "Brown", "location": "United States"}'::JSONB),
                ('Designer wall paintings', 5, 'Home Decor', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB),
                ('Robot building kit', 4, 'Toys', true, '{"color": "Multicolor", "location": "China"}'::JSONB),
                ('Sporty tank top', 4, 'Apparel', true, '{"color": "Blue", "location": "United States"}'::JSONB),
                ('Bluetooth-enabled speaker', 3, 'Electronics', true, '{"color": "Black", "location": "Canada"}'::JSONB),
                ('Winter woolen socks', 5, 'Footwear', false, '{"color": "Gray", "location": "China"}'::JSONB),
                ('Rustic bookshelf', 4, 'Furniture', true, '{"color": "Brown", "location": "United States"}'::JSONB),
                ('Moisturizing lip balm', 4, 'Beauty', true, '{"color": "Pink", "location": "Canada"}'::JSONB),
                ('Lightweight camera bag', 5, 'Photography', false, '{"color": "Black", "location": "China"}'::JSONB),
                ('Historical fiction book', 3, 'Books', true, '{"color": "Multicolor", "location": "United States"}'::JSONB),
                ('Pure honey jar', 4, 'Groceries', true, '{"color": "Yellow", "location": "Canada"}'::JSONB),
                ('Handcrafted wooden frame', 5, 'Home Decor', false, '{"color": "Brown", "location": "China"}'::JSONB),
                ('Plush teddy bear', 4, 'Toys', true, '{"color": "Brown", "location": "United States"}'::JSONB),
                ('Warm woolen sweater', 3, 'Apparel', false, '{"color": "Red", "location": "Canada"}'::JSONB)
                ) AS t(description, rating, category, in_stock, metadata)
        LOOP
            EXECUTE 'INSERT INTO ' || full_table_name || ' (description, rating, category, in_stock, metadata) VALUES ($1, $2, $3, $4, $5)'
            USING data_to_insert.description, data_to_insert.rating, data_to_insert.category, data_to_insert.in_stock, data_to_insert.metadata;
        END LOOP;

    ELSE
        RAISE WARNING 'The table % already exists, skipping.', full_table_name;
    END IF;
END $$;

-- This create_bm25 function to dynamically create index and query functions
-- Example:
--
-- CALL create_bm25(
--     function_name => 'dynamicbm25',
--     schema_name => 'paradedb',
--     table_name => 'bm25_test_table',
--     text_fields => '{"description": {}, "category": {}}'::text
-- );

--- This call will create a new function called 'dynamicbm25', which can be used to query.

CREATE OR REPLACE PROCEDURE paradedb.create_bm25(
    function_name text,
    table_name text,
    key_field text,
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields text DEFAULT '{}',
    numeric_fields text DEFAULT '{}',
    boolean_fields text DEFAULT '{}',
    json_fields text DEFAULT '{}'
)
LANGUAGE plpgsql AS $$
BEGIN
	-- Drop existing index and function if they exist
	CALL paradedb.drop_bm25(
		function_name => function_name,
		schema_name => schema_name
	);

    -- Create the BM25 index
    EXECUTE format('CREATE INDEX %I ON %I.%I USING bm25 ((%I.%I.*)) WITH (key_field=%L, text_fields=%L, numeric_fields=%L, boolean_fields=%L, json_fields=%L);',
                   function_name, schema_name, table_name, schema_name, table_name, key_field, text_fields, numeric_fields, boolean_fields, json_fields);

    -- Create the dynamic function
    EXECUTE format($f$
        CREATE OR REPLACE FUNCTION %I.%I(
            query text,
            offset_rows integer DEFAULT NULL,
            limit_rows integer DEFAULT NULL,
            fuzzy_fields text DEFAULT NULL,
            distance integer DEFAULT NULL,
            transpose_cost_one boolean DEFAULT NULL,
            prefix text DEFAULT NULL,
            regex_fields text DEFAULT NULL,
            max_num_chars integer DEFAULT NULL
        ) RETURNS SETOF %I.%I AS $func$
        DECLARE
            json_string text;
            select_string text;
            main_query text;
        BEGIN
           json_string := json_strip_nulls(
        		json_build_object(
        	    	'index_name', %L,
            		'query', query,
                	'offset_rows', offset_rows,
                	'limit_rows', limit_rows,
                	'fuzzy_fields', fuzzy_fields,
                	'distance', distance,
                	'transpose_cost_one', transpose_cost_one,
                	'prefix', prefix,
                	'regex_fields', regex_fields,
                	'max_num_chars', max_num_chars
            	)
        	)::text;
            select_string := format($m$ SELECT * FROM %I.%I WHERE (%I.ctid)$m$);
            main_query := select_string || '@@@' || '''' || json_string || '''';
        	RETURN QUERY EXECUTE main_query; 
        END;
        $func$ LANGUAGE plpgsql;
    $f$, schema_name, function_name, schema_name, table_name, function_name, schema_name, table_name, table_name);
END;
$$;

CREATE OR REPLACE PROCEDURE paradedb.drop_bm25(
    function_name text,
    schema_name text DEFAULT CURRENT_SCHEMA
)
LANGUAGE plpgsql AS $$
DECLARE
    function_exists int;
    index_exists int;
BEGIN
    -- Check if the index exists
    SELECT INTO index_exists COUNT(*)
    FROM pg_class c
    JOIN pg_namespace n ON c.relnamespace = n.oid
    WHERE n.nspname = schema_name
      AND c.relname = function_name
      AND c.relkind = 'i';  -- 'i' for index

    -- Check if the function exists
    SELECT INTO function_exists COUNT(*)
    FROM pg_proc p
    JOIN pg_namespace n ON p.pronamespace = n.oid
    WHERE n.nspname = schema_name
      AND p.proname = function_name;

    -- Drop the BM25 index if it exists
    IF index_exists > 0 THEN
        EXECUTE format('DROP INDEX %I.%I;', schema_name, function_name);
    END IF;

    -- Drop the dynamic function if it exists
    IF function_exists > 0 THEN
        EXECUTE format('DROP FUNCTION %I.%I(query text, start integer, max_rows integer, fuzzy_fields text, distance integer, transpose_cost_one boolean, prefix text, regex_fields text, max_num_chars integer);',
                       schema_name, function_name);
    END IF;
END;
$$;
