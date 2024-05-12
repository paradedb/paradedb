\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.7.0'" to load this file. \quit

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

    IF text_fields = '{}' AND numeric_fields = '{}' AND boolean_fields = '{}' AND json_fields = '{}' THEN
        RAISE EXCEPTION 'no text_fields, numeric_fields, boolean_fields, or json_fields were specified for index %', index_name;
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
