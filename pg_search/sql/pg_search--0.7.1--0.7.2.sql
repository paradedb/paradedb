\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.7.2'" to load this file. \quit

DROP PROCEDURE IF EXISTS paradedb.create_bm25_test_table;
CREATE OR REPLACE PROCEDURE paradedb.create_bm25_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb')
LANGUAGE plpgsql
AS $$
DECLARE
    full_table_name TEXT := schema_name || '.' || table_name;
    data_to_insert RECORD;
    original_client_min_messages TEXT;
BEGIN
    SELECT INTO original_client_min_messages current_setting('client_min_messages');
    SET client_min_messages TO WARNING;

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
            metadata JSONB,
            created_at TIMESTAMP,
            last_updated_date DATE,
            latest_available_time TIME
        )';

        FOR data_to_insert IN
            SELECT * FROM (VALUES
                ('Ergonomic metal keyboard', 4, 'Electronics', true, '{"color": "Silver", "location": "United States"}'::JSONB, TIMESTAMP '2023-05-01 09:12:34', DATE '2023-05-03', TIME '09:12:34'),
                ('Plastic Keyboard', 4, 'Electronics', false, '{"color": "Black", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-15 13:27:09', DATE '2023-04-16', TIME '13:27:09'),
                ('Sleek running shoes', 5, 'Footwear', true, '{"color": "Blue", "location": "China"}'::JSONB, TIMESTAMP '2023-04-28 10:55:43', DATE '2023-04-29', TIME '10:55:43'),
                ('White jogging shoes', 3, 'Footwear', false, '{"color": "White", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-20 16:38:02', DATE '2023-04-22', TIME '16:38:02'),
                ('Generic shoes', 4, 'Footwear', true, '{"color": "Brown", "location": "Canada"}'::JSONB, TIMESTAMP '2023-05-02 08:45:11', DATE '2023-05-03', TIME '08:45:11'),
                ('Compact digital camera', 5, 'Photography', false, '{"color": "Black", "location": "China"}'::JSONB, TIMESTAMP '2023-04-25 11:20:35', DATE '2023-04-26', TIME '11:20:35'),
                ('Hardcover book on history', 2, 'Books', true, '{"color": "Brown", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-18 14:59:27', DATE '2023-04-19', TIME '14:59:27'),
                ('Organic green tea', 3, 'Groceries', true, '{"color": "Green", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-30 09:18:45', DATE '2023-05-01', TIME '09:18:45'),
                ('Modern wall clock', 4, 'Home Decor', false, '{"color": "Silver", "location": "China"}'::JSONB, TIMESTAMP '2023-04-24 12:37:52', DATE '2023-04-25', TIME '12:37:52'),
                ('Colorful kids toy', 1, 'Toys', true, '{"color": "Multicolor", "location": "United States"}'::JSONB, TIMESTAMP '2023-05-04 15:29:12', DATE '2023-05-06', TIME '15:29:12'),
                ('Soft cotton shirt', 5, 'Apparel', true, '{"color": "Blue", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-29 08:10:17', DATE '2023-04-30', TIME '08:10:17'),
                ('Innovative wireless earbuds', 5, 'Electronics', true, '{"color": "Black", "location": "China"}'::JSONB, TIMESTAMP '2023-04-22 10:05:39', DATE '2023-04-23', TIME '10:05:39'),
                ('Sturdy hiking boots', 4, 'Footwear', true, '{"color": "Brown", "location": "United States"}'::JSONB, TIMESTAMP '2023-05-05 13:45:22', DATE '2023-05-07', TIME '13:45:22'),
                ('Elegant glass table', 3, 'Furniture', true, '{"color": "Clear", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-26 17:22:58', DATE '2023-04-28', TIME '17:22:58'),
                ('Refreshing face wash', 2, 'Beauty', false, '{"color": "White", "location": "China"}'::JSONB, TIMESTAMP '2023-04-27 09:52:04', DATE '2023-04-29', TIME '09:52:04'),
                ('High-resolution DSLR', 4, 'Photography', true, '{"color": "Black", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-21 14:30:19', DATE '2023-04-23', TIME '14:30:19'),
                ('Paperback romantic novel', 3, 'Books', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB, TIMESTAMP '2023-05-03 10:08:57', DATE '2023-05-04', TIME '10:08:57'),
                ('Freshly ground coffee beans', 5, 'Groceries', true, '{"color": "Brown", "location": "China"}'::JSONB, TIMESTAMP '2023-04-23 08:40:15', DATE '2023-04-25', TIME '08:40:15'),
                ('Artistic ceramic vase', 4, 'Home Decor', false, '{"color": "Multicolor", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-19 15:17:29', DATE '2023-04-21', TIME '15:17:29'),
                ('Interactive board game', 3, 'Toys', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB, TIMESTAMP '2023-05-01 12:25:06', DATE '2023-05-02', TIME '12:25:06'),
                ('Slim-fit denim jeans', 5, 'Apparel', false, '{"color": "Blue", "location": "China"}'::JSONB, TIMESTAMP '2023-04-28 16:54:33', DATE '2023-04-30', TIME '16:54:33'),
                ('Fast charging power bank', 4, 'Electronics', true, '{"color": "Black", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-17 11:35:52', DATE '2023-04-19', TIME '11:35:52'),
                ('Comfortable slippers', 3, 'Footwear', true, '{"color": "Brown", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-16 09:20:37', DATE '2023-04-17', TIME '09:20:37'),
                ('Classic leather sofa', 5, 'Furniture', false, '{"color": "Brown", "location": "China"}'::JSONB, TIMESTAMP '2023-05-06 14:45:27', DATE '2023-05-08', TIME '14:45:27'),
                ('Anti-aging serum', 4, 'Beauty', true, '{"color": "White", "location": "United States"}'::JSONB, TIMESTAMP '2023-05-09 10:30:15', DATE '2023-05-10', TIME '10:30:15'),
                ('Portable tripod stand', 4, 'Photography', true, '{"color": "Black", "location": "Canada"}'::JSONB, TIMESTAMP '2023-05-07 15:20:48', DATE '2023-05-09', TIME '15:20:48'),
                ('Mystery detective novel', 2, 'Books', false, '{"color": "Multicolor", "location": "China"}'::JSONB, TIMESTAMP '2023-05-04 11:55:23', DATE '2023-05-05', TIME '11:55:23'),
                ('Organic breakfast cereal', 5, 'Groceries', true, '{"color": "Brown", "location": "United States"}'::JSONB, TIMESTAMP '2023-05-02 07:40:59', DATE '2023-05-03', TIME '07:40:59'),
                ('Designer wall paintings', 5, 'Home Decor', true, '{"color": "Multicolor", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-30 14:18:37', DATE '2023-05-01', TIME '14:18:37'),
                ('Robot building kit', 4, 'Toys', true, '{"color": "Multicolor", "location": "China"}'::JSONB, TIMESTAMP '2023-04-29 16:25:42', DATE '2023-05-01', TIME '16:25:42'),
                ('Sporty tank top', 4, 'Apparel', true, '{"color": "Blue", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-27 12:09:53', DATE '2023-04-28', TIME '12:09:53'),
                ('Bluetooth-enabled speaker', 3, 'Electronics', true, '{"color": "Black", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-26 09:34:11', DATE '2023-04-28', TIME '09:34:11'),
                ('Winter woolen socks', 5, 'Footwear', false, '{"color": "Gray", "location": "China"}'::JSONB, TIMESTAMP '2023-04-25 14:55:08', DATE '2023-04-27', TIME '14:55:08'),
                ('Rustic bookshelf', 4, 'Furniture', true, '{"color": "Brown", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-24 08:20:47', DATE '2023-04-25', TIME '08:20:47'),
                ('Moisturizing lip balm', 4, 'Beauty', true, '{"color": "Pink", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-23 13:48:29', DATE '2023-04-24', TIME '13:48:29'),
                ('Lightweight camera bag', 5, 'Photography', false, '{"color": "Black", "location": "China"}'::JSONB, TIMESTAMP '2023-04-22 17:10:55', DATE '2023-04-24', TIME '17:10:55'),
                ('Historical fiction book', 3, 'Books', true, '{"color": "Multicolor", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-21 10:35:40', DATE '2023-04-22', TIME '10:35:40'),
                ('Pure honey jar', 4, 'Groceries', true, '{"color": "Yellow", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-20 15:22:14', DATE '2023-04-22', TIME '15:22:14'),
                ('Handcrafted wooden frame', 5, 'Home Decor', false, '{"color": "Brown", "location": "China"}'::JSONB, TIMESTAMP '2023-04-19 08:55:06', DATE '2023-04-21', TIME '08:55:06'),
                ('Plush teddy bear', 4, 'Toys', true, '{"color": "Brown", "location": "United States"}'::JSONB, TIMESTAMP '2023-04-18 11:40:59', DATE '2023-04-19', TIME '11:40:59'),
                ('Warm woolen sweater', 3, 'Apparel', false, '{"color": "Red", "location": "Canada"}'::JSONB, TIMESTAMP '2023-04-17 14:28:37', DATE '2023-04-18', TIME '14:28:37')
                ) AS t(description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time)
        LOOP
            EXECUTE 'INSERT INTO ' || full_table_name || ' (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)'
            USING data_to_insert.description, data_to_insert.rating, data_to_insert.category, data_to_insert.in_stock, data_to_insert.metadata, data_to_insert.created_at, data_to_insert.last_updated_date, data_to_insert.latest_available_time;
        END LOOP;

    ELSE
        RAISE WARNING 'The table % already exists, skipping.', full_table_name;
    END IF;

    EXECUTE 'SET client_min_messages TO ' || quote_literal(original_client_min_messages);
END $$;

DROP PROCEDURE IF EXISTS paradedb.create_bm25;
CREATE OR REPLACE PROCEDURE paradedb.create_bm25(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields text DEFAULT '{}',
    numeric_fields text DEFAULT '{}',
    boolean_fields text DEFAULT '{}',
    json_fields text DEFAULT '{}',
    datetime_fields text DEFAULT '{}'
)
LANGUAGE plpgsql AS $$
DECLARE
    index_json JSONB;
    original_client_min_messages TEXT;
BEGIN
    SELECT INTO original_client_min_messages current_setting('client_min_messages');
    SET client_min_messages TO WARNING;
    
    IF index_name IS NULL OR index_name = '' THEN
        RAISE EXCEPTION 'no index_name parameter given for bm25 index';
    END IF;

    -- Disallow creation of an index with existing name
    IF EXISTS(SELECT i.schema_name FROM information_schema.schemata i WHERE i.schema_name = index_name) THEN
        RAISE EXCEPTION 'relation "%" already exists', index_name;
    END IF;

    IF table_name IS NULL OR table_name = '' THEN
        RAISE EXCEPTION 'no table_name parameter given for bm25 index "%"', index_name;
    END IF;

    IF key_field IS NULL OR key_field = '' THEN
        RAISE EXCEPTION 'no key_field parameter given for bm25 index "%"', index_name;
    END IF;

    IF text_fields = '{}' AND numeric_fields = '{}' AND boolean_fields = '{}' AND json_fields = '{}' AND datetime_fields = '{}' THEN
        RAISE EXCEPTION 'no text_fields, numeric_fields, boolean_fields, json_fields, or datetime_fields were specified for index %', index_name;
    END IF;

    index_json := jsonb_build_object(
        'index_name', format('%s_bm25_index', index_name),
        'table_name', table_name,
        'key_field', key_field,
        'schema_name', schema_name
    );

    -- Create the new, empty schema.
    EXECUTE format('CREATE SCHEMA %s', index_name);

    -- Create a new BM25 index on the specified table.
    -- The index is created dynamically based on the function parameters.
    EXECUTE format('CREATE INDEX %s_bm25_index ON %I.%I USING bm25 ((%I.*)) WITH (key_field=%L, text_fields=%L, numeric_fields=%L, boolean_fields=%L, json_fields=%L, datetime_fields=%L);',
                   index_name, schema_name, table_name, table_name, key_field, text_fields, numeric_fields, boolean_fields, json_fields, datetime_fields);

    -- Dynamically create a new function for performing searches on the indexed table.
    -- The variable '__paradedb_search_config__' is available to the function_body parameter.
    -- Note that due to how the SQL query is parsed, this variable cannot share a name with
    -- any existing table or column. The possibility of a naming collision is inevitable, but
    -- we choose '__paradedb_search_config__' in hopes of avoiding a collision.
    EXECUTE paradedb.format_bm25_function(
        function_name => format('%I.search', index_name),        	
        return_type => format('SETOF %I.%I', schema_name, table_name),
        function_body => format('RETURN QUERY SELECT * FROM %I.%I WHERE %I @@@ __paradedb_search_config__', schema_name, table_name, table_name),
        index_json => index_json
    );

    EXECUTE paradedb.format_empty_function(
        function_name => format('%I.schema', index_name),
        return_type => 'TABLE(name text, field_type text, stored bool, indexed bool, fast bool, fieldnorms bool, expand_dots bool, tokenizer text, record text, normalizer text)',
        function_body => format('RETURN QUERY SELECT * FROM paradedb.schema_bm25(''%s'')', index_name)
    );

    EXECUTE paradedb.format_hybrid_function(
        function_name => format('%I.rank_hybrid', index_name),
        return_type => format('TABLE(%s bigint, rank_hybrid real)', key_field),
        function_body => '
            WITH similarity AS (
                SELECT
                    __key_field__ as key_field,
                  CASE
                    WHEN (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ()) = 0 THEN
                      0
                    ELSE
                      1 - ((__similarity_query__) - MIN(__similarity_query__) OVER ()) / 
                      (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ())
                    END AS score
                FROM %I.%I
                ORDER BY __similarity_query__
                LIMIT $2
            ),
            bm25 AS (
                SELECT 
                    id as key_field, 
                    rank_bm25 as score 
                FROM paradedb.minmax_bm25($1)
            )
            SELECT
                COALESCE(similarity.key_field, bm25.key_field) AS __key_field__,
                (COALESCE(similarity.score, 0.0) * $3 + COALESCE(bm25.score, 0.0) * $4)::real AS score_hybrid
            FROM similarity
            FULL OUTER JOIN bm25 ON similarity.key_field = bm25.key_field
            ORDER BY score_hybrid DESC;
        ',
        index_json => index_json
    );

    EXECUTE 'SET client_min_messages TO ' || quote_literal(original_client_min_messages);
   END;
$$;

DROP PROCEDURE IF EXISTS paradedb.drop_bm25;
CREATE OR REPLACE PROCEDURE paradedb.drop_bm25(
    index_name text,
    schema_name text DEFAULT CURRENT_SCHEMA
)
LANGUAGE plpgsql AS $$
DECLARE 
    original_client_min_messages TEXT;
BEGIN
    SELECT INTO original_client_min_messages current_setting('client_min_messages');
    SET client_min_messages TO WARNING;

    EXECUTE format('DROP INDEX IF EXISTS %s.%s_bm25_index', schema_name, index_name); 
    EXECUTE format('DROP SCHEMA IF EXISTS %s CASCADE', index_name);
    PERFORM paradedb.drop_bm25_internal(format('%s_bm25_index', index_name));

    EXECUTE 'SET client_min_messages TO ' || quote_literal(original_client_min_messages);
  END;
$$;
