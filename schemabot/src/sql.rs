// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use pg_query::{Node, NodeEnum};
use serde_json::Value;

pub struct ParsedStatement {
    pub node: Node,
    pub source: String,
    pub comparison: String,
}

// Compare parsed SQL without source offsets, which change with formatting.
// Only numeric locations are metadata: a tablespace's string location is SQL.
pub fn comparison(node: &Node) -> String {
    let mut normalized = node.clone();
    if let Some(NodeEnum::CreateFunctionStmt(function)) = &mut normalized.node {
        function.replace = false;
        function.options.sort_by_key(|option| match &option.node {
            Some(NodeEnum::DefElem(option)) => option.defname.clone(),
            _ => String::new(),
        });
    }
    match &mut normalized.node {
        Some(NodeEnum::DropStmt(drop)) => {
            drop.missing_ok = false;
            for object in &mut drop.objects {
                if let Some(NodeEnum::ObjectWithArgs(args)) = &mut object.node {
                    args.objfuncargs.clear();
                }
            }
        }
        Some(NodeEnum::DefineStmt(definition)) => definition.replace = false,
        Some(NodeEnum::ViewStmt(view)) => view.replace = false,
        _ => {}
    }
    let mut json = serde_json::to_value(normalized).expect("protobuf node serialization");
    remove_offsets(&mut json);
    json.to_string()
}

fn remove_offsets(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.retain(|name, value| {
                !(value.is_number()
                    && (name == "location" || name.ends_with("_location") || name == "stmt_len"))
            });
            fields.values_mut().for_each(remove_offsets);
        }
        Value::Array(items) => items.iter_mut().for_each(remove_offsets),
        _ => {}
    }
}

pub fn parse(input: &str) -> Result<Vec<ParsedStatement>, String> {
    let input = input.trim_start();
    let input = match input.split_once('\n') {
        Some((guard, sql)) if guard.starts_with("\\echo Use ") && guard.ends_with("\\quit") => sql,
        _ => input,
    };
    let result = pg_query::parse(input).map_err(|error| format!("SQL parse error: {error}"))?;
    result
        .protobuf
        .stmts
        .into_iter()
        .map(|statement| {
            let start = statement.stmt_location as usize;
            let end = match statement.stmt_len {
                0 => input.len(),
                length => start + length as usize,
            };
            let slice = &input[start..end];
            let tokens =
                pg_query::scan(slice).map_err(|error| format!("SQL scan error: {error}"))?;
            let start = start
                + tokens
                    .tokens
                    .iter()
                    .find(|token| {
                        !matches!(
                            token.token(),
                            pg_query::protobuf::Token::SqlComment
                                | pg_query::protobuf::Token::CComment
                        )
                    })
                    .ok_or("empty SQL statement")?
                    .start as usize;
            let node = *statement.stmt.ok_or("empty parser statement")?;
            Ok(ParsedStatement {
                comparison: comparison(&node),
                node,
                source: input[start..end].trim().trim_end_matches(';').to_owned(),
            })
        })
        .collect()
}

pub fn read(path: &str) -> Result<Vec<ParsedStatement>, String> {
    let content = std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    parse(&content).map_err(|error| format!("{path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_formatting_comments_and_type_aliases() {
        assert_eq!(
            parse("CREATE TABLE t(x integer)").unwrap()[0].comparison,
            parse("-- header\n create table t (x int);").unwrap()[0].comparison,
        );
    }

    #[test]
    fn preserves_sql_bodies_and_string_contents() {
        let sql = "CREATE FUNCTION app.f(\"limit\" int, \"json\" json) RETURNS text LANGUAGE sql AS $$SELECT 'é;a;b -- /* text */'$$;";
        let parsed = parse(sql).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].source, sql.trim_end_matches(';'));
        assert_ne!(
            parsed[0].comparison,
            parse(&sql.replace("a;b", "a;c")).unwrap()[0].comparison
        );
    }

    #[test]
    fn accepts_only_the_initial_psql_extension_guard() {
        assert_eq!(parse("\\echo Use \"ALTER EXTENSION x UPDATE\" to load this file. \\quit\nCREATE TABLE t(x int);").unwrap()[0].comparison, parse("CREATE TABLE t(x int)").unwrap()[0].comparison);
        assert!(parse("SELECT 1;\n\\quit").is_err());
    }

    #[test]
    fn normalizes_function_options_and_replace() {
        assert_eq!(
            parse("CREATE OR REPLACE FUNCTION f() RETURNS integer IMMUTABLE LANGUAGE sql AS $$SELECT 1$$").unwrap()[0].comparison,
            parse("CREATE FUNCTION f() RETURNS int LANGUAGE sql IMMUTABLE AS $$SELECT 1$$").unwrap()[0].comparison,
        );
    }

    #[test]
    fn tablespace_paths_are_not_position_metadata() {
        assert_ne!(
            parse("CREATE TABLESPACE ts LOCATION '/mnt/one'").unwrap()[0].comparison,
            parse("CREATE TABLESPACE ts LOCATION '/mnt/two'").unwrap()[0].comparison
        );
    }
    #[test]
    fn normalizes_drop_spelling_and_replace_for_views_and_aggregates() {
        for (a, b) in [
            (
                "DROP FUNCTION IF EXISTS vector_info(index regclass, field text)",
                "DROP FUNCTION vector_info(regclass, text)",
            ),
            ("DROP PROCEDURE IF EXISTS f(a int)", "DROP PROCEDURE f(int)"),
            (
                "DROP AGGREGATE IF EXISTS pdb.agg(a jsonb, b text)",
                "DROP AGGREGATE pdb.agg(jsonb, text)",
            ),
            (
                "CREATE OR REPLACE VIEW v AS SELECT 1",
                "CREATE VIEW v AS SELECT 1",
            ),
            (
                "CREATE OR REPLACE AGGREGATE a(int) (SFUNC = int4pl, STYPE = int)",
                "CREATE AGGREGATE a(int) (SFUNC = int4pl, STYPE = int)",
            ),
        ] {
            assert_eq!(
                parse(a).unwrap()[0].comparison,
                parse(b).unwrap()[0].comparison,
                "{a}"
            );
        }
        assert_ne!(
            parse("DROP FUNCTION f(int)").unwrap()[0].comparison,
            parse("DROP FUNCTION f(text)").unwrap()[0].comparison
        );
        assert_ne!(
            parse("DROP FUNCTION f(int)").unwrap()[0].comparison,
            parse("DROP FUNCTION f(int) CASCADE").unwrap()[0].comparison
        );
    }
}
