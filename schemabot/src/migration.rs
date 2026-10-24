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

use crate::sql::ParsedStatement;
use pg_query::{Node, NodeEnum, protobuf::ObjectType};
use std::collections::{BTreeMap, HashSet};

fn statements(sql: &str) -> Result<BTreeMap<String, String>, String> {
    Ok(crate::sql::parse(sql)?
        .into_iter()
        .map(|statement| (statement.comparison, statement.source))
        .collect())
}

pub fn check(expected: &str, fragments: &[String]) -> Result<(), String> {
    let expected = statements(expected)?;
    let mut actual = HashSet::new();
    // Parse each file separately: concatenation could hide its first statement
    // behind a trailing line comment in the previous fragment.
    for fragment in fragments {
        for statement in crate::sql::parse(fragment)? {
            if let Some(drop) = crate::schema::replacement_drop(&statement)? {
                actual.insert(drop);
            }
            actual.insert(statement.comparison);
        }
    }
    let missing: Vec<_> = expected
        .iter()
        .filter(|(key, _)| !actual.contains(*key))
        .map(|(_, source)| format!("{source};"))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "{} required schema statement(s) missing from migration fragments:\n{}",
            missing.len(),
            missing.join("\n")
        ));
    }
    println!(
        "All {} required schema statements found in migration fragments",
        expected.len()
    );
    Ok(())
}

fn type_name(names: &[Node]) -> Result<Vec<String>, String> {
    names
        .iter()
        .map(|name| match &name.node {
            Some(NodeEnum::String(name)) => Ok(name.sval.clone()),
            _ => Err("expected a type name identifier".into()),
        })
        .collect()
}

// Manual coverage proves only that a fragment addresses the affected object;
// reviewers still need to check that its ALTER or DROP implements the change.
pub fn check_manual(expected: &[&ParsedStatement], fragments: &[String]) -> Result<(), String> {
    let mut actual = HashSet::new();
    for fragment in fragments {
        for statement in crate::sql::parse(fragment)? {
            match statement.node.node {
                Some(NodeEnum::AlterTypeStmt(node)) => {
                    actual.insert(type_name(&node.type_name)?);
                }
                Some(NodeEnum::AlterEnumStmt(node)) => {
                    actual.insert(type_name(&node.type_name)?);
                }
                Some(NodeEnum::DropStmt(node))
                    if node.remove_type == ObjectType::ObjectType as i32 =>
                {
                    for object in node.objects {
                        if let Some(NodeEnum::TypeName(name)) = object.node {
                            actual.insert(type_name(&name.names)?);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut missing = Vec::new();
    for statement in expected {
        let names = match &statement.node.node {
            Some(NodeEnum::CreateEnumStmt(node)) => Some(&node.type_name),
            Some(NodeEnum::DefineStmt(node)) if node.kind == ObjectType::ObjectType as i32 => {
                Some(&node.defnames)
            }
            _ => None,
        };
        let covered = names
            .map(|names| {
                let expected = type_name(names)?;
                Ok::<_, String>(actual.iter().any(|actual| {
                    actual == &expected
                        || ((actual.len() == 1 || expected.len() == 1)
                            && actual.last() == expected.last())
                }))
            })
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or(false);
        if !covered {
            missing.push(format!("{};", statement.source));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "manual migration required for these objects (add an ALTER TYPE or DROP TYPE targeting the same type; other object kinds require SchemaBot support):\n{}",
            missing.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_types_match_qualified_and_unqualified_names() {
        for definition in [
            "CREATE TYPE TestTable AS ENUM ('Orders')",
            "CREATE TYPE TestTable (INPUT = decode, OUTPUT = encode)",
            "CREATE TYPE paradedb.testtable AS ENUM ('Orders')",
        ] {
            let parsed = crate::sql::parse(definition).unwrap();
            let expected: Vec<_> = parsed.iter().collect();
            for fragment in [
                "ALTER TYPE paradedb.testtable ADD VALUE 'Parts'",
                "ALTER TYPE testtable ADD VALUE 'Parts'",
                "ALTER TYPE paradedb.testtable SET (STORAGE = extended)",
                "DROP TYPE paradedb.testtable",
                "DROP TYPE testtable",
            ] {
                check_manual(&expected, &[fragment.into()]).unwrap();
            }
            assert!(check_manual(&expected, &["DROP TYPE unrelated".into()]).is_err());
        }
        let parsed =
            crate::sql::parse("CREATE TYPE paradedb.testtable AS ENUM ('Orders')").unwrap();
        assert!(
            check_manual(
                &parsed.iter().collect::<Vec<_>>(),
                &["DROP TYPE other.testtable".into()]
            )
            .is_err()
        );
        // Quoted identifiers remain case sensitive.
        let parsed = crate::sql::parse("CREATE TYPE \"TestTable\" AS ENUM ('Orders')").unwrap();
        assert!(
            check_manual(
                &parsed.iter().collect::<Vec<_>>(),
                &["DROP TYPE paradedb.testtable".into()]
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_statements_in_any_order() {
        check(
            "CREATE TABLE t(x int); GRANT SELECT ON t TO reader",
            &["GRANT SELECT ON t TO reader; CREATE TABLE t(x int)".into()],
        )
        .unwrap();
    }

    #[test]
    fn checks_grants_and_revokes_and_accepts_extra_statements() {
        assert!(
            check(
                "REVOKE ALL ON t FROM PUBLIC",
                &["CREATE TABLE t(x int)".into()]
            )
            .is_err()
        );
        check(
            "GRANT SELECT ON t TO reader",
            &["GRANT SELECT ON t TO reader; SELECT 1;".into()],
        )
        .unwrap();
    }

    #[test]
    fn checks_fragments_individually() {
        check(
            "CREATE TABLE t(x int); CREATE TABLE u(x int)",
            &[
                "CREATE TABLE t(x int); -- end".into(),
                "CREATE TABLE u(x int);".into(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn rejects_invalid_sql_and_different_definitions() {
        assert!(check("CREATE TABLE t(x int)", &["CREATE TABLE (".into()]).is_err());
        assert!(check("CREATE TABLE t(x int)", &["CREATE TABLE t(x text)".into()]).is_err());
    }
    #[test]
    fn accepts_released_named_drop() {
        check(
            "DROP FUNCTION IF EXISTS vector_info(regclass, text)",
            &[include_str!("../../pg_search/sql/pg_search--0.25.11--0.26.0.sql").into()],
        )
        .unwrap();
    }

    #[test]
    fn reports_missing_sql() {
        let error = check(
            "CREATE TABLE t(x int); CREATE TABLE u(x text)",
            &["CREATE TABLE t(x int)".into()],
        )
        .unwrap_err();
        assert!(error.contains("1 required"));
        assert!(error.contains("CREATE TABLE u(x text);"));
        assert!(!error.contains("CREATE TABLE t"));
    }

    #[test]
    fn explicit_replacements_cover_drop_and_matching_definition() {
        for (drop, definition) in [
            (
                "DROP FUNCTION app.f(int)",
                "CREATE FUNCTION app.f(x int) RETURNS int LANGUAGE sql AS $$SELECT x + 1$$",
            ),
            (
                "DROP PROCEDURE app.p(int)",
                "CREATE PROCEDURE app.p(x int) LANGUAGE sql AS $$SELECT x$$",
            ),
            ("DROP VIEW app.v", "CREATE VIEW app.v AS SELECT 2 AS n"),
            (
                "DROP AGGREGATE app.a(int)",
                "CREATE AGGREGATE app.a(int) (SFUNC = int4pl, STYPE = int, INITCOND = '0')",
            ),
        ] {
            let expected = format!("{drop}; {definition}");
            let replacement = definition.replacen("CREATE", "CREATE OR REPLACE", 1);
            check(&expected, std::slice::from_ref(&replacement)).unwrap();
            // A plain CREATE still requires the explicit DROP.
            assert!(check(&expected, &[definition.into()]).is_err());
            // Replacement of another object or definition cannot cover this change.
            assert!(check(&expected, &[replacement.replace("app.", "other.")]).is_err());
            assert!(
                check(
                    &expected,
                    &[replacement
                        .replace("int", "bigint")
                        .replace("SELECT 2", "SELECT 3")]
                )
                .is_err()
            );
        }
    }
}
