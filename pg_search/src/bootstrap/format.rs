// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use serde_json::Value;

pub fn format_bm25_function(
    function_name: &str,
    return_type: &str,
    function_body: &str,
    index_json: &Value,
) -> String {
    let index_json_str = serde_json::to_string(&index_json).unwrap();
    let formatted_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}(
            query text,
            offset_rows integer DEFAULT NULL,
            limit_rows integer DEFAULT NULL,
            alias text DEFAULT NULL,
            stable_sort boolean DEFAULT NULL,
            highlight_field text DEFAULT NULL,
            postfix text DEFAULT NULL,
            prefix text DEFAULT NULL,
            max_num_chars integer DEFAULT NULL
        ) RETURNS {return_type} AS $func$
        BEGIN
            RETURN QUERY SELECT * FROM {function_name}(
                query => paradedb.parse(query),
                offset_rows => offset_rows,
                limit_rows => limit_rows,
                alias => alias,
                stable_sort => stable_sort,
                highlight_field => highlight_field,
                postfix => postfix,
                prefix => prefix,
                max_num_chars => max_num_chars
            );
        END
        $func$ LANGUAGE plpgsql;

        CREATE OR REPLACE FUNCTION {function_name}(
            query paradedb.searchqueryinput,
            offset_rows integer DEFAULT NULL,
            limit_rows integer DEFAULT NULL,
            alias text DEFAULT NULL,
            stable_sort boolean DEFAULT NULL,
            highlight_field text DEFAULT NULL,
            postfix text DEFAULT NULL,
            prefix text DEFAULT NULL,
            max_num_chars integer DEFAULT NULL
        ) RETURNS {return_type} AS $func$
        DECLARE
            __paradedb_search_config__ JSONB;
        BEGIN
            __paradedb_search_config__ := '{index_json_str}'::jsonb || jsonb_build_object(
                'query', query::text::jsonb,
                'offset_rows', offset_rows,
                'limit_rows', limit_rows,
                'alias', alias,
                'stable_sort', stable_sort,
                'highlight_field', highlight_field,
                'postfix', postfix,
                'prefix', prefix,
                'max_num_chars', max_num_chars
            );
            {function_body};
        END
        $func$ LANGUAGE plpgsql;
        "#,
    );

    formatted_sql
}

pub fn format_hybrid_function(
    function_name: &str,
    return_type: &str,
    function_body: &str,
    index_json: &Value,
) -> String {
    let formatted_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}(
            bm25_query text,
            similarity_query text,
            similarity_limit_n integer DEFAULT 100,
            bm25_limit_n integer DEFAULT 100,
            similarity_weight real DEFAULT 0.5,
            bm25_weight real DEFAULT 0.5
        ) RETURNS {return_type} AS $func$
        BEGIN
            RETURN QUERY SELECT * FROM {function_name}(
                bm25_query => paradedb.parse(bm25_query),
                similarity_query => similarity_query,
                similarity_limit_n => similarity_limit_n,
                bm25_limit_n => bm25_limit_n,
                similarity_weight => similarity_weight,
                bm25_weight => bm25_weight
            );
        END
        $func$ LANGUAGE plpgsql;

        CREATE OR REPLACE FUNCTION {function_name}(
            bm25_query paradedb.searchqueryinput,
            similarity_query text,
            similarity_limit_n integer DEFAULT 100,
            bm25_limit_n integer DEFAULT 100,
            similarity_weight real DEFAULT 0.5,
            bm25_weight real DEFAULT 0.5
        ) RETURNS {return_type} AS $func$
        DECLARE
            __paradedb_search_config__ JSONB;
            query text;
        BEGIN
            __paradedb_search_config__ := jsonb_strip_nulls(
                '{index_json}'::jsonb || jsonb_build_object(
                    'query', bm25_query::text::jsonb,
                    'limit_rows', bm25_limit_n
                )
            );

            query := replace('{function_body}', '__similarity_query__', similarity_query);
            query := replace(query, '__key_field__', __paradedb_search_config__ ->>'key_field');

            RETURN QUERY EXECUTE query
            USING __paradedb_search_config__, similarity_limit_n, similarity_weight, bm25_weight;
        END
        $func$ LANGUAGE plpgsql;
        "#,
        function_name = function_name,
        return_type = return_type,
        index_json = serde_json::to_string(&index_json).unwrap(),
        function_body = function_body
    );

    formatted_sql
}

pub fn format_empty_function(
    function_name: &str,
    return_type: &str,
    function_body: &str,
) -> String {
    let formatted_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}() RETURNS {return_type} AS $func$
        BEGIN
            {function_body};
        END
        $func$ LANGUAGE plpgsql;
        "#,
        function_name = function_name,
        return_type = return_type,
        function_body = function_body
    );

    formatted_sql
}
