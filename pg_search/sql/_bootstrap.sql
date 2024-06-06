GRANT ALL ON SCHEMA paradedb TO PUBLIC;

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

CREATE OR REPLACE FUNCTION paradedb.format_bm25_function(
    function_name text,
    return_type text,
    function_body text,
    index_json jsonb
) RETURNS text AS $$
DECLARE
    formatted_sql text;
BEGIN
    -- Format the dynamic SQL for creating the search functions
    formatted_sql := format($f$
        CREATE OR REPLACE FUNCTION %s(
            query text, -- The search query
            offset_rows integer DEFAULT NULL, -- Offset for paginated results
            limit_rows integer DEFAULT NULL, -- Limit for paginated results
            alias text DEFAULT NULL, -- Alias for disambiguation
            stable_sort boolean DEFAULT NULL -- Stable sort order of results
        ) RETURNS %s AS $func$
        BEGIN
            -- Explicitly cast the 'query' text parameter to 'paradedb.searchqueryinput' type
            RETURN QUERY SELECT * FROM %s(
                query => paradedb.parse(query),
                offset_rows => offset_rows,
                limit_rows => limit_rows,
                alias => alias,
                stable_sort => stable_sort
            );
        END
        $func$ LANGUAGE plpgsql;

        CREATE OR REPLACE FUNCTION %s(
            query paradedb.searchqueryinput, -- The search query
            offset_rows integer DEFAULT NULL, -- Offset for paginated results
            limit_rows integer DEFAULT NULL, -- Limit for paginated results
            alias text DEFAULT NULL, -- Alias for disambiguation
            stable_sort boolean DEFAULT NULL -- Stable sort order of results
        ) RETURNS %s AS $func$
        DECLARE
            __paradedb_search_config__ JSONB;
        BEGIN
            -- Merge the outer 'index_json' object with the parameters passed to the dynamic function.
            __paradedb_search_config__ := %L::jsonb || jsonb_build_object(
                'query', query::text::jsonb,
                'offset_rows', offset_rows,
                'limit_rows', limit_rows,
                'alias', alias,
                'stable_sort', stable_sort
            );
            %s; -- Execute the function body with the constructed JSONB parameter
        END
        $func$ LANGUAGE plpgsql;
    $f$, function_name, return_type, function_name, function_name, return_type, index_json::text, function_body);

    RETURN formatted_sql;
END;
$$ LANGUAGE plpgsql;

-- A helper function to format a hybrid search query
CREATE OR REPLACE FUNCTION paradedb.format_hybrid_function(
    function_name text,
    return_type text,
    function_body text,
    index_json jsonb
) RETURNS text AS $outerfunc$
BEGIN
    DECLARE
        __table_name__ text;
        __schema_name__ text;
        __function_body__ text;
    BEGIN
        __table_name__ := index_json->>'table_name';
        __schema_name__ := index_json->>'schema_name';
        __function_body__ := format(
            function_body,
            __schema_name__,
            __table_name__
        );

        RETURN format($f$
            -- If you add parameters to the function here, you must also add them to the `drop_bm25`
            -- function, or you'll get a runtime "function does not exist" error when you try to drop.
            CREATE OR REPLACE FUNCTION %s(
                bm25_query text,
                similarity_query text,
                similarity_limit_n integer DEFAULT 100,
                bm25_limit_n integer DEFAULT 100,
                similarity_weight real DEFAULT 0.5,
                bm25_weight real DEFAULT 0.5
            ) RETURNS %s AS $func$
            BEGIN
            -- Explicitly cast the 'bm25_query' text parameter to 'paradedb.searchqueryinput' type
            RETURN QUERY SELECT * FROM %s(
                bm25_query => paradedb.parse(bm25_query),
                similarity_query => similarity_query,
                similarity_limit_n => similarity_limit_n,
                bm25_limit_n => bm25_limit_n,
                similarity_weight => similarity_weight,
                bm25_weight => bm25_weight
            );
            END;
            $func$ LANGUAGE plpgsql;

            CREATE OR REPLACE FUNCTION %s(
                bm25_query paradedb.searchqueryinput,
                similarity_query text,
                similarity_limit_n integer DEFAULT 100,
                bm25_limit_n integer DEFAULT 100,
                similarity_weight real DEFAULT 0.5,
                bm25_weight real DEFAULT 0.5
            ) RETURNS %s AS $func$
            DECLARE
                __paradedb_search_config__ JSONB;
                query text;
            BEGIN
            -- Merge the outer 'index_json' object into the parameters passed to the dynamic function.
                __paradedb_search_config__ := jsonb_strip_nulls(
                    '%s'::jsonb || jsonb_build_object(
                        'query', bm25_query::text::jsonb,
                        'limit_rows', bm25_limit_n
                    )
                );

                query := replace(%L, '__similarity_query__', similarity_query);
                query := replace(query, '__key_field__', __paradedb_search_config__ ->>'key_field');

                RETURN QUERY EXECUTE query
                USING __paradedb_search_config__, similarity_limit_n, similarity_weight, bm25_weight;
            END;
            $func$ LANGUAGE plpgsql;
        $f$, function_name, return_type, function_name, function_name, return_type, index_json, __function_body__);
    END;
END;
$outerfunc$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION paradedb.format_empty_function(
    function_name text,
    return_type text,
    function_body text
) RETURNS text AS $outerfunc$
BEGIN
     RETURN format($f$
        -- If you add parameters to the function here, you must also add them to the `drop_bm25`
        -- function, or you'll get a runtime "function does not exist" error when you try to drop.
        CREATE OR REPLACE FUNCTION %s() RETURNS %s AS $func$
        BEGIN
            %s;
        END;
        $func$ LANGUAGE plpgsql;
    $f$, function_name, return_type, function_body);
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
