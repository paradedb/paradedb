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

-- This create_bm25 function to dynamically create index and query functions.
-- This call will create a new function called 'dynamicbm25', which can be used to query.
-- Example:
--
-- CALL create_bm25(
--     schema_name => 'paradedb',
--     table_name => 'bm25_test_table',
--     text_fields => '{"description": {}, "category": {}}'::text
-- );

-- This procedure creates a dynamic BM25 index and a corresponding search function for a given table.
-- Parameters:
--   table_name: The name of the table on which the BM25 index is to be created.
--   key_field: The primary key field of the table.
--   schema_name: The schema in which the table resides. Defaults to the current schema.
--   text_fields: JSON object representing the text fields for the index.
--   numeric_fields: JSON object representing the numeric fields for the index.
--   boolean_fields: JSON object representing the boolean fields for the index.
--   json_fields: JSON object representing the json fields for the index.
CREATE OR REPLACE PROCEDURE paradedb.create_bm25(
    schema_name text,
    table_name text,
    key_field text,
    table_schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields text DEFAULT '{}',
    numeric_fields text DEFAULT '{}',
    boolean_fields text DEFAULT '{}',
    json_fields text DEFAULT '{}'
)
LANGUAGE plpgsql AS $$
DECLARE
    index_name text;
    index_json JSONB;
BEGIN
    index_name := format('%s_bm25', schema_name);
    index_json := jsonb_build_object(
        'schema_name', schema_name,
        'table_name', table_name,
        'key_field', key_field,
        'table_schema_name', table_schema_name,
        'index_name', index_name
    );

    -- Drop any existing index and function with the same name to avoid conflicts.
    CALL paradedb.drop_bm25(schema_name);

    -- Create the new, empty schema.
    EXECUTE format('CREATE SCHEMA %I', schema_name);

    -- Create a new BM25 index on the specified table.
    -- The index is created dynamically based on the function parameters.
    EXECUTE format('CREATE INDEX %I ON %I USING bm25 ((%I.%I.*)) WITH (key_field=%L, text_fields=%L, numeric_fields=%L, boolean_fields=%L, json_fields=%L);',
                   index_name, table_name, table_schema_name, table_name, key_field, text_fields, numeric_fields, boolean_fields, json_fields);

    -- Dynamically create a new function for performing searches on the indexed table.
    -- Note the EXECUTE in the query function. The format_bm25_query function is just
    -- a Rust helper that returns a string, so that string must dynamically be executed.
    EXECUTE paradedb.format_bm25_function(
        format('%I.search', schema_name),
        format('SETOF %I.%I', table_schema_name, table_name),
        'EXECUTE paradedb.format_bm25_query',
        index_json
    );

   END;
$$;

-- A helper function to format a search query. The "template" below is used by several
-- search functions, like "search", "rank", and "highlight", so we've extracted the code
-- into a common function.
CREATE OR REPLACE FUNCTION paradedb.format_bm25_function(
    function_name text,
    return_type text,
    query_function text,
    index_json jsonb
) RETURNS text AS $outerfunc$
BEGIN
     RETURN format($f$
        -- If you add parameters to the function here, you must also add them to the `drop_bm25`
        -- function, or you'll get a runtime "function does not exist" error when you try to drop.
        CREATE OR REPLACE FUNCTION %s(
            query text, -- The search query
            offset_rows integer DEFAULT NULL, -- Offset for paginated results
            limit_rows integer DEFAULT NULL, -- Limit for paginated results
            fuzzy_fields text DEFAULT NULL, -- Fields where fuzzy search is applied
            distance integer DEFAULT NULL, -- Distance parameter for fuzzy search
            transpose_cost_one boolean DEFAULT NULL, -- Transpose cost parameter for fuzzy search
            prefix text DEFAULT NULL, -- Prefix parameter for searches
            regex_fields text DEFAULT NULL, -- Fields where regex search is applied
            max_num_chars integer DEFAULT NULL -- Maximum character limit for searches
        ) RETURNS %s AS $func$
        DECLARE
            search_config JSON;
        BEGIN
           search_config := jsonb_strip_nulls(
        		'%s'::jsonb || jsonb_build_object(
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
        	);
        	RETURN QUERY %s(search_config); 
        END;
        $func$ LANGUAGE plpgsql;
    $f$, function_name, return_type, index_json, query_function);
END;
$outerfunc$ LANGUAGE plpgsql;

CREATE OR REPLACE PROCEDURE paradedb.drop_bm25(
    schema_name text
)
LANGUAGE plpgsql AS $$
DECLARE
    index_name text;
    function_exists int;
    index_exists int;
BEGIN
    index_name := format('%s_bm25', schema_name);
    EXECUTE format('DROP SCHEMA IF EXISTS %s CASCADE', schema_name);
    EXECUTE format('DROP INDEX IF EXISTS %I', index_name); 
  END;
$$;
