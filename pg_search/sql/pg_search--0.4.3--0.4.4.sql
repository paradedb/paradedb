\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.4.4'" to load this file. \quit

-- Use this create_bm25 function to dynamically create index and query functions.
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
--   index_name: The schema in which the table resides. Defaults to the current schema.
--   table_name: The name of the table on which the BM25 index is to be created.
--   key_field: The primary key field of the table.
--   text_fields: JSON object representing the text fields for the index.
--   numeric_fields: JSON object representing the numeric fields for the index.
--   boolean_fields: JSON object representing the boolean fields for the index.
--   json_fields: JSON object representing the json fields for the index.
CREATE OR REPLACE PROCEDURE paradedb.create_bm25(
    index_name text DEFAULT '',
    table_name text DEFAULT '',
    key_field text DEFAULT '',
    schema_name text DEFAULT CURRENT_SCHEMA,
    text_fields text DEFAULT '{}',
    numeric_fields text DEFAULT '{}',
    boolean_fields text DEFAULT '{}',
    json_fields text DEFAULT '{}'
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

    IF table_name IS NULL OR table_name = '' THEN
        RAISE EXCEPTION 'no table_name parameter given for bm25 index "%"', index_name;
    END IF;

    IF key_field IS NULL OR key_field = '' THEN
        RAISE EXCEPTION 'no key_field parameter given for bm25 index "%"', index_name;
    END IF;

    IF text_fields = '{}' AND numeric_fields = '{}' AND boolean_fields = '{}' AND json_fields = '{}' THEN
        RAISE EXCEPTION 'no text_fields, numeric_fields, boolean_fields, or json_fields were specified for index %', index_name;
    END IF;

    index_json := jsonb_build_object(
        'index_name', format('%s_bm25_index', index_name),
        'table_name', table_name,
        'key_field', key_field,
        'schema_name', schema_name
    );

    -- Drop any existing index and function with the same name to avoid conflicts.
    CALL paradedb.drop_bm25(index_name, schema_name => schema_name);

    -- Create the new, empty schema.
    EXECUTE format('CREATE SCHEMA %s', index_name);

    -- Create a new BM25 index on the specified table.
    -- The index is created dynamically based on the function parameters.
    EXECUTE format('CREATE INDEX %s_bm25_index ON %I.%I USING bm25 ((%I.*)) WITH (key_field=%L, text_fields=%L, numeric_fields=%L, boolean_fields=%L, json_fields=%L);',
                   index_name, schema_name, table_name, table_name, key_field, text_fields, numeric_fields, boolean_fields, json_fields);

    -- Dynamically create a new function for performing searches on the indexed table.
    -- The variable '__paradedb_search_config__' is available to the function_body parameter.
    -- Note that due to how the SQL query is parsed, this variable cannot share a name with
    -- any existing table or column. The possibility of a naming collision is inevitable, but
    -- we choose '__paradedb_search_config__' in hopes of avoiding a collision.
    EXECUTE paradedb.format_bm25_function(
        function_name => format('%I.search', index_name),        	
        return_type => format('SETOF %I.%I', schema_name, table_name),
        function_body => format('RETURN QUERY SELECT * FROM %I.%I WHERE %I @@@ __paradedb_search_config__;', schema_name, table_name, table_name),
        index_json => index_json
    );

    EXECUTE paradedb.format_bm25_function(
        function_name => format('%I.highlight', index_name),
        return_type => format('TABLE(%s bigint, highlight_bm25 text)', key_field),
        function_body => 'RETURN QUERY SELECT * FROM paradedb.highlight_bm25(__paradedb_search_config__);',
        index_json => index_json
    );

    EXECUTE paradedb.format_bm25_function(
        function_name => format('%I.rank', index_name),
        return_type => format('TABLE(%s bigint, rank_bm25 real)', key_field),
        function_body => 'RETURN QUERY SELECT * FROM paradedb.rank_bm25(__paradedb_search_config__);',
        index_json => index_json
    );

    EXECUTE paradedb.format_empty_function(
        function_name => format('%I.schema', index_name),
        return_type => 'TABLE(name text, field_type text, stored bool, indexed bool, fast bool, fieldnorms bool, expand_dots bool, tokenizer text, record text, normalizer text)',
        function_body => format('RETURN QUERY SELECT * FROM paradedb.schema_bm25(''%s'');', index_name)
    );

    EXECUTE paradedb.format_hybrid_function(
        function_name => format('%I.rank_hybrid', index_name),
        return_type => format('TABLE(%s bigint, rank_hybrid real)', key_field),
        function_body => '
            WITH similarity AS (
                SELECT
                    __key_field__ as key_field,
                    1 - ((__similarity_query__) - MIN(__similarity_query__) OVER ()) / 
                    (MAX(__similarity_query__) OVER () - MIN(__similarity_query__) OVER ()) AS score
                FROM %I
                ORDER BY __similarity_query__
                LIMIT $2
            ),
            bm25 AS (
                SELECT 
                    __key_field__ as key_field, 
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


-- A helper function to format a search query. The "template" below is used by several
-- search functions, like "search", "rank", and "highlight", so we've extracted the code
-- into a common function.
CREATE OR REPLACE FUNCTION paradedb.format_bm25_function(
    function_name text,
    return_type text,
    function_body text,
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
            prefix boolean DEFAULT NULL, -- Prefix parameter for searches
            regex_fields text DEFAULT NULL, -- Fields where regex search is applied
            max_num_chars integer DEFAULT NULL, -- Maximum character limit for searches
            highlight_field text DEFAULT NULL -- Field name to highlight (highlight func only)
        ) RETURNS %s AS $func$
        DECLARE
            __paradedb_search_config__ JSONB;
        BEGIN
           -- Merge the outer 'index_json' object into the parameters passed to the dynamic function.
           __paradedb_search_config__ := jsonb_strip_nulls(
        		'%s'::jsonb || jsonb_build_object(
            		'query', query,
                	'offset_rows', offset_rows,
                	'limit_rows', limit_rows,
                	'fuzzy_fields', fuzzy_fields,
                	'distance', distance,
                	'transpose_cost_one', transpose_cost_one,
                	'prefix', prefix,
                	'regex_fields', regex_fields,
                	'max_num_chars', max_num_chars,
                    'highlight_field', highlight_field
            	)
        	);
            %s
        END;
        $func$ LANGUAGE plpgsql;
    $f$, function_name, return_type, index_json, function_body);
END;
$outerfunc$ LANGUAGE plpgsql;


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
