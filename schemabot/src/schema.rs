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

use crate::sql::{ParsedStatement, comparison};
use pg_query::{Node, NodeEnum, protobuf as pb};
use std::collections::{HashMap, HashSet};

// Changes are found by subtracting normalized statement sets. Named-object
// matching is needed only to distinguish supported replacements from changes
// that require an explicit, manually designed migration.
pub struct Diff<'a> {
    pub sql: String,
    pub manual: Vec<String>,
    pub manual_objects: Vec<&'a ParsedStatement>,
}

pub fn diff<'a>(
    before: &'a [ParsedStatement],
    after: &[ParsedStatement],
) -> Result<Diff<'a>, String> {
    let old: HashSet<_> = before
        .iter()
        .map(|statement| &statement.comparison)
        .collect();
    let new: HashSet<_> = after
        .iter()
        .map(|statement| &statement.comparison)
        .collect();
    let removed: Vec<_> = before
        .iter()
        .filter(|statement| !new.contains(&statement.comparison))
        .collect();
    let added: Vec<_> = after
        .iter()
        .filter(|statement| !old.contains(&statement.comparison))
        .collect();
    let mut replacements = HashMap::new();
    for statement in &added {
        if let Ok(Some(removal)) = removal(statement) {
            replacements.insert(removal.identity, *statement);
        }
    }

    let mut manual = Vec::new();
    let mut manual_objects = Vec::new();
    let mut drops = Vec::new();
    let mut owner_changes = Vec::new();
    let mut consumed = HashSet::new();
    let mut seen = HashSet::new();
    for statement in removed {
        if !seen.insert(&statement.comparison) {
            continue;
        }
        let removal = match removal(statement) {
            Ok(Some(removal)) => removal,
            Ok(None) => continue,
            Err(error) => {
                manual.push(error);
                manual_objects.push(statement);
                continue;
            }
        };
        if let Some(replacement) = replacements.get(&removal.identity) {
            if matches!(statement.node.node, Some(NodeEnum::CreateSchemaStmt(_))) {
                match schema_owner(statement, replacement) {
                    Ok(sql) => owner_changes.push(sql),
                    Err(error) => {
                        manual.push(error);
                        manual_objects.push(statement);
                    }
                }
                consumed.insert(&replacement.comparison);
                continue;
            }
            if !removal.replaceable {
                manual.push(format!(
                    "cannot replace this object automatically; migrate manually:\nBefore:\n{}\nAfter:\n{}",
                    statement.source, replacement.source
                ));
                manual_objects.push(statement);
                consumed.insert(&replacement.comparison);
                continue;
            }
        }
        drops.push((removal.order, removal.sql));
    }
    // Drop dependents before their supporting routines and types. This is a
    // suggestion, not a catalog dependency graph or an executable-plan proof.
    drops.sort_by_key(|(order, _)| *order);
    let mut output = String::new();
    for sql in drops.into_iter().map(|(_, sql)| sql).chain(owner_changes) {
        output.push_str(&sql);
        output.push_str(";\n");
    }
    for statement in added {
        if consumed.insert(&statement.comparison) {
            output.push_str(&statement.source);
            output.push_str(";\n");
        }
    }
    // Check that synthesized statements are valid SQL before writing stdout.
    crate::sql::parse(&output)?;
    Ok(Diff {
        sql: output,
        manual,
        manual_objects,
    })
}

struct Removal {
    identity: String,
    sql: String,
    replaceable: bool,
    order: u8,
}

fn wrap(node: NodeEnum) -> Node {
    Node { node: Some(node) }
}

fn text(value: &str) -> Node {
    wrap(NodeEnum::String(pb::String {
        sval: value.to_owned(),
    }))
}

fn list(items: Vec<Node>) -> Node {
    wrap(NodeEnum::List(pb::List { items }))
}

fn names(relation: &pb::RangeVar) -> Vec<Node> {
    [
        &relation.catalogname,
        &relation.schemaname,
        &relation.relname,
    ]
    .into_iter()
    .filter(|name| !name.is_empty())
    .map(|name| text(name))
    .collect()
}

fn type_object(names: Vec<Node>) -> Node {
    wrap(NodeEnum::TypeName(pb::TypeName {
        names,
        ..Default::default()
    }))
}

// An explicit CREATE OR REPLACE also covers the suggested DROP for this object.
// Keep the replace flag on the original node; comparison deliberately erases it.
pub fn replacement_drop(statement: &ParsedStatement) -> Result<Option<String>, String> {
    let replaces = match &statement.node.node {
        Some(NodeEnum::CreateFunctionStmt(node)) => node.replace,
        Some(NodeEnum::ViewStmt(node)) => node.replace,
        Some(NodeEnum::DefineStmt(node)) => {
            node.replace && node.kind == pb::ObjectType::ObjectAggregate as i32
        }
        _ => false,
    };
    if replaces {
        Ok(removal(statement)?.map(|removal| removal.identity))
    } else {
        Ok(None)
    }
}

fn removal(statement: &ParsedStatement) -> Result<Option<Removal>, String> {
    use pb::ObjectType as O;
    let unsupported = || {
        format!(
            "cannot remove this statement automatically:\n{}",
            statement.source
        )
    };
    let (kind, object, replaceable, order) = match statement
        .node
        .node
        .as_ref()
        .ok_or_else(unsupported)?
    {
        NodeEnum::CreateFunctionStmt(function) => {
            let mut arguments = Vec::new();
            for parameter in &function.parameters {
                let Some(NodeEnum::FunctionParameter(parameter)) = &parameter.node else {
                    return Err(unsupported());
                };
                if [
                    pb::FunctionParameterMode::FuncParamOut as i32,
                    pb::FunctionParameterMode::FuncParamTable as i32,
                ]
                .contains(&parameter.mode)
                {
                    continue;
                }
                let mut argument = parameter.arg_type.clone().ok_or_else(unsupported)?;
                // PostgreSQL routine identity ignores parameter names, defaults,
                // return columns and type modifiers.
                argument.typmods.clear();
                argument.typemod = -1;
                arguments.push(wrap(NodeEnum::TypeName(argument)));
            }
            let object = wrap(NodeEnum::ObjectWithArgs(pb::ObjectWithArgs {
                objname: function.funcname.clone(),
                objargs: arguments,
                ..Default::default()
            }));
            (
                if function.is_procedure {
                    O::ObjectProcedure
                } else {
                    O::ObjectFunction
                },
                object,
                true,
                2,
            )
        }
        NodeEnum::ViewStmt(view) => (
            O::ObjectView,
            list(names(view.view.as_ref().ok_or_else(unsupported)?)),
            true,
            0,
        ),
        NodeEnum::CreateSchemaStmt(schema) => {
            let name = if schema.schemaname.is_empty() {
                &schema.authrole.as_ref().ok_or_else(unsupported)?.rolename
            } else {
                &schema.schemaname
            };
            (O::ObjectSchema, text(name), false, 4)
        }
        NodeEnum::CreateEnumStmt(enumeration) => (
            O::ObjectType,
            type_object(enumeration.type_name.clone()),
            false,
            3,
        ),
        NodeEnum::CreateDomainStmt(domain) => (
            O::ObjectDomain,
            type_object(domain.domainname.clone()),
            false,
            3,
        ),
        NodeEnum::CompositeTypeStmt(composite) => (
            O::ObjectType,
            type_object(names(composite.typevar.as_ref().ok_or_else(unsupported)?)),
            false,
            3,
        ),
        NodeEnum::CreateCastStmt(cast) => (
            O::ObjectCast,
            list(vec![
                wrap(NodeEnum::TypeName(
                    cast.sourcetype.clone().ok_or_else(unsupported)?,
                )),
                wrap(NodeEnum::TypeName(
                    cast.targettype.clone().ok_or_else(unsupported)?,
                )),
            ]),
            true,
            0,
        ),
        NodeEnum::CreateAmStmt(method) => (O::ObjectAccessMethod, text(&method.amname), false, 1),
        NodeEnum::CreateOpClassStmt(class) => {
            let mut name = vec![text(&class.amname)];
            name.extend(class.opclassname.clone());
            (O::ObjectOpclass, list(name), true, 0)
        }
        NodeEnum::DefineStmt(definition) => {
            let kind = O::try_from(definition.kind).map_err(|_| unsupported())?;
            let object = match kind {
                O::ObjectType => type_object(definition.defnames.clone()),
                O::ObjectOperator => {
                    let mut arguments = vec![Node::default(), Node::default()];
                    for option in &definition.definition {
                        if let Some(NodeEnum::DefElem(option)) = &option.node {
                            match option.defname.as_str() {
                                "leftarg" => {
                                    arguments[0] = *option.arg.clone().ok_or_else(unsupported)?
                                }
                                "rightarg" => {
                                    arguments[1] = *option.arg.clone().ok_or_else(unsupported)?
                                }
                                _ => {}
                            }
                        }
                    }
                    wrap(NodeEnum::ObjectWithArgs(pb::ObjectWithArgs {
                        objname: definition.defnames.clone(),
                        objargs: arguments,
                        ..Default::default()
                    }))
                }
                O::ObjectAggregate => {
                    let [parameters, direct] = definition.args.as_slice() else {
                        return Err(unsupported());
                    };
                    if !matches!(&direct.node, Some(NodeEnum::Integer(value)) if value.ival == -1) {
                        return Err(unsupported());
                    }
                    let Some(NodeEnum::List(parameters)) = &parameters.node else {
                        return Err(unsupported());
                    };
                    let mut arguments = Vec::new();
                    for parameter in &parameters.items {
                        let Some(NodeEnum::FunctionParameter(parameter)) = &parameter.node else {
                            return Err(unsupported());
                        };
                        let mut argument = parameter.arg_type.clone().ok_or_else(unsupported)?;
                        argument.typmods.clear();
                        argument.typemod = -1;
                        arguments.push(wrap(NodeEnum::TypeName(argument)));
                    }
                    wrap(NodeEnum::ObjectWithArgs(pb::ObjectWithArgs {
                        objname: definition.defnames.clone(),
                        objargs: arguments,
                        ..Default::default()
                    }))
                }
                // Ordered-set aggregates require deliberate signature support.
                _ => return Err(unsupported()),
            };
            let mut removal = drop_object(
                kind,
                object,
                matches!(kind, O::ObjectOperator | O::ObjectAggregate),
                if matches!(kind, O::ObjectOperator | O::ObjectAggregate) {
                    0
                } else {
                    3
                },
            )?;
            // A shell type and its full definition are distinct installation
            // steps, even though they address the same catalog object.
            if kind == O::ObjectType {
                removal
                    .identity
                    .push_str(if definition.definition.is_empty() {
                        ":shell"
                    } else {
                        ":definition"
                    });
            }
            return Ok(Some(removal));
        }
        NodeEnum::GrantStmt(grant) if grant.is_grant => {
            let mut revoke = grant.clone();
            revoke.is_grant = false;
            // Removing a grant with GRANT OPTION removes the privilege too.
            revoke.grant_option = false;
            let sql = NodeEnum::GrantStmt(revoke)
                .deparse()
                .map_err(|error| error.to_string())?;
            return Ok(Some(Removal {
                identity: statement.comparison.clone(),
                sql,
                replaceable: true,
                order: 0,
            }));
        }
        // These are installation actions, not independently removable objects.
        NodeEnum::SelectStmt(_)
        | NodeEnum::DoStmt(_)
        | NodeEnum::InsertStmt(_)
        | NodeEnum::AlterFunctionStmt(_)
        | NodeEnum::AlterTypeStmt(_)
        | NodeEnum::CommentStmt(_) => return Ok(None),
        _ => return Err(unsupported()),
    };
    drop_object(kind, object, replaceable, order).map(Some)
}

fn drop_object(
    kind: pb::ObjectType,
    object: Node,
    replaceable: bool,
    order: u8,
) -> Result<Removal, String> {
    let drop = wrap(NodeEnum::DropStmt(pb::DropStmt {
        remove_type: kind as i32,
        objects: vec![object],
        missing_ok: true,
        behavior: pb::DropBehavior::DropRestrict as i32,
        ..Default::default()
    }));
    let sql = drop
        .deparse()
        .map_err(|error| format!("cannot render DROP statement: {error}"))?;
    Ok(Removal {
        identity: comparison(&drop),
        sql,
        replaceable,
        order,
    })
}

fn schema_owner(before: &ParsedStatement, after: &ParsedStatement) -> Result<String, String> {
    let (Some(NodeEnum::CreateSchemaStmt(old)), Some(NodeEnum::CreateSchemaStmt(new))) =
        (&before.node.node, &after.node.node)
    else {
        unreachable!()
    };
    let mut expected = old.clone();
    expected.authrole = new.authrole.clone();
    if comparison(&wrap(NodeEnum::CreateSchemaStmt(expected))) != after.comparison {
        return Err("only supports changing a schema's owner".into());
    }
    let role = new
        .authrole
        .clone()
        .ok_or("schema owner change needs an explicit new owner")?;
    let name = if new.schemaname.is_empty() {
        &role.rolename
    } else {
        &new.schemaname
    };
    NodeEnum::AlterOwnerStmt(Box::new(pb::AlterOwnerStmt {
        object_type: pb::ObjectType::ObjectSchema as i32,
        object: Some(Box::new(text(name))),
        newowner: Some(role),
        ..Default::default()
    }))
    .deparse()
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggest(before: &str, after: &str) -> Result<String, String> {
        let before = crate::sql::parse(before)?;
        let diff = diff(&before, &crate::sql::parse(after)?)?;
        if diff.manual.is_empty() {
            Ok(diff.sql)
        } else {
            Err(diff.manual.join("\n"))
        }
    }

    fn covers(before: &str, after: &str, expected: &str) {
        let suggested = suggest(before, after).unwrap();
        crate::migration::check(expected, std::slice::from_ref(&suggested)).unwrap();
        crate::migration::check(&suggested, &[expected.into()]).unwrap();
    }

    #[test]
    fn additions_keep_pgrx_text_in_dependency_order() {
        let head = "CREATE TYPE app.query; CREATE FUNCTION app.decode(cstring) RETURNS app.query LANGUAGE c AS 'MODULE_PATHNAME', 'decode'; CREATE TYPE app.query (INPUT = app.decode, OUTPUT = app.encode);";
        let suggested = suggest("", head).unwrap();
        assert!(suggested.starts_with("CREATE TYPE app.query;"));
        assert!(
            suggested.find("CREATE FUNCTION").unwrap() < suggested.rfind("CREATE TYPE").unwrap()
        );
        crate::migration::check(head, &[suggested]).unwrap();
    }

    #[test]
    fn body_change_requires_drop_and_new_definition() {
        covers(
            "CREATE FUNCTION app.answer() RETURNS int LANGUAGE sql AS $$SELECT 7$$",
            "CREATE FUNCTION app.answer() RETURNS int LANGUAGE sql AS $$SELECT 8$$",
            "DROP FUNCTION IF EXISTS app.answer(); CREATE FUNCTION app.answer() RETURNS int LANGUAGE sql AS $$SELECT 8$$",
        );
    }

    #[test]
    fn names_defaults_out_columns_and_overloads_have_correct_signatures() {
        let before = "CREATE FUNCTION app.f(a int DEFAULT 7, OUT b int) LANGUAGE sql AS $$SELECT a$$; CREATE FUNCTION app.f(a text) RETURNS text LANGUAGE sql AS $$SELECT a$$;";
        let after = "CREATE FUNCTION app.f(a text) RETURNS text LANGUAGE sql AS $$SELECT a$$;";
        covers(before, after, "DROP FUNCTION IF EXISTS app.f(int)");
        covers(
            "CREATE FUNCTION app.rows(a int) RETURNS TABLE(n int, s text) LANGUAGE sql AS $$SELECT a, 'x'$$",
            "",
            "DROP FUNCTION IF EXISTS app.rows(int)",
        );
    }

    #[test]
    fn procedure_removal_and_array_arguments_are_supported() {
        covers(
            "CREATE PROCEDURE app.run(INOUT items int[]) LANGUAGE plpgsql AS $$BEGIN NULL; END$$",
            "",
            "DROP PROCEDURE IF EXISTS app.run(int[])",
        );
    }

    #[test]
    fn removal_order_puts_operator_and_view_before_routine() {
        let before = "CREATE FUNCTION app.matches(text, text) RETURNS boolean LANGUAGE sql AS $$SELECT true$$; CREATE OPERATOR app.@@@ (LEFTARG = text, RIGHTARG = text, FUNCTION = app.matches); CREATE VIEW app.v AS SELECT app.matches('a', 'b');";
        let suggested = suggest(before, "").unwrap();
        let function = suggested.find("DROP FUNCTION").unwrap();
        assert!(suggested.find("DROP OPERATOR").unwrap() < function);
        assert!(suggested.find("DROP VIEW").unwrap() < function);
    }

    #[test]
    fn overloaded_and_unary_operators_have_distinct_drops() {
        covers(
            "CREATE OPERATOR app.! (RIGHTARG = int, FUNCTION = app.factorial)",
            "",
            "DROP OPERATOR IF EXISTS app.!(NONE, int)",
        );
        let before = "CREATE OPERATOR app.@@@ (LEFTARG = text, RIGHTARG = text, FUNCTION = app.matches); CREATE OPERATOR app.@@@ (LEFTARG = int, RIGHTARG = int, FUNCTION = app.matches_int);";
        let after =
            "CREATE OPERATOR app.@@@ (LEFTARG = text, RIGHTARG = text, FUNCTION = app.matches);";
        covers(before, after, "DROP OPERATOR IF EXISTS app.@@@(int, int)");
    }

    #[test]
    fn view_replacement_and_schema_owner_change_are_structural() {
        covers(
            "CREATE VIEW app.v AS SELECT 7 AS n",
            "CREATE VIEW app.v AS SELECT 8 AS n",
            "DROP VIEW IF EXISTS app.v; CREATE VIEW app.v AS SELECT 8 AS n",
        );
        covers(
            "CREATE SCHEMA \"odd space\" AUTHORIZATION owner1",
            "CREATE SCHEMA \"odd space\" AUTHORIZATION owner2",
            "ALTER SCHEMA \"odd space\" OWNER TO owner2",
        );
    }

    #[test]
    fn enum_changes_need_explicit_planner_support() {
        assert!(
            suggest(
                "CREATE TYPE app.status AS ENUM ('open')",
                "CREATE TYPE app.status AS ENUM ('open', 'closed')"
            )
            .is_err()
        );
        covers(
            "CREATE TYPE app.status AS ENUM ('open')",
            "",
            "DROP TYPE IF EXISTS app.status",
        );
    }

    #[test]
    fn domain_composite_cast_and_access_method_drops_are_valid() {
        covers(
            "CREATE DOMAIN app.positive AS int CHECK (VALUE > 0)",
            "",
            "DROP DOMAIN IF EXISTS app.positive",
        );
        covers(
            "CREATE TYPE app.pair AS (a int, b text)",
            "",
            "DROP TYPE IF EXISTS app.pair",
        );
        covers(
            "CREATE CAST (int AS text) WITH INOUT",
            "",
            "DROP CAST IF EXISTS (int AS text)",
        );
        covers(
            "CREATE ACCESS METHOD app_search TYPE INDEX HANDLER app.handler",
            "",
            "DROP ACCESS METHOD IF EXISTS app_search",
        );
    }

    #[test]
    fn operator_class_identity_includes_access_method() {
        let a = "CREATE OPERATOR CLASS app.ops FOR TYPE int USING paradedb AS STORAGE int;";
        let b = "CREATE OPERATOR CLASS app.ops FOR TYPE int USING btree AS STORAGE int;";
        covers(
            &format!("{a}{b}"),
            a,
            "DROP OPERATOR CLASS IF EXISTS app.ops USING btree",
        );
    }

    #[test]
    fn grants_are_revoked_before_narrower_grants_are_added() {
        covers(
            "GRANT SELECT, INSERT ON app.docs TO reader",
            "GRANT SELECT ON app.docs TO reader",
            "REVOKE SELECT, INSERT ON app.docs FROM reader; GRANT SELECT ON app.docs TO reader",
        );
    }

    #[test]
    fn duplicate_emissions_are_deduplicated() {
        let suggested = suggest(
            "GRANT SELECT ON app.docs TO reader; GRANT SELECT ON app.docs TO reader",
            "",
        )
        .unwrap();
        assert_eq!(crate::sql::parse(&suggested).unwrap().len(), 1);
        crate::migration::check("REVOKE SELECT ON app.docs FROM reader", &[suggested]).unwrap();
    }

    #[test]
    fn installation_actions_are_not_dropped() {
        assert!(suggest("DO $$BEGIN RAISE NOTICE 'x'; END$$; INSERT INTO t VALUES(1); ALTER FUNCTION f() IMMUTABLE;", "").unwrap().is_empty());
    }

    #[test]
    fn sql_body_and_new_parser_syntax_are_preserved() {
        let head = "CREATE FUNCTION app.answer() RETURNS integer LANGUAGE SQL RETURN 9; MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN DELETE RETURNING t.id;";
        covers("", head, head);
    }
    #[test]
    fn comments_are_replaced_by_the_new_comment() {
        covers(
            "COMMENT ON ACCESS METHOD paradedb IS 'old'",
            "COMMENT ON ACCESS METHOD paradedb IS 'new'",
            "COMMENT ON ACCESS METHOD paradedb IS 'new'",
        );
    }

    #[test]
    fn plain_aggregate_removal_and_replacement() {
        covers(
            "CREATE AGGREGATE pdb.agg(jsonb, text) (SFUNC = pdb.trans, STYPE = internal)",
            "",
            "DROP AGGREGATE IF EXISTS pdb.agg(jsonb, text)",
        );
        covers(
            "CREATE AGGREGATE a(int) (SFUNC = int4pl, STYPE = int)",
            "CREATE AGGREGATE a(int) (SFUNC = int4pl, STYPE = int, INITCOND = '0')",
            "DROP AGGREGATE a(int); CREATE AGGREGATE a(int) (SFUNC = int4pl, STYPE = int, INITCOND = '0')",
        );
        assert!(
            suggest(
                "CREATE AGGREGATE a(int ORDER BY int) (SFUNC = int4pl, STYPE = int)",
                ""
            )
            .is_err()
        );
    }

    #[test]
    fn pgrx_fixture_pair_has_clean_sources_and_accepts_named_drops() {
        let before = include_str!("../tests/fixtures/base.sql");
        let after = include_str!("../tests/fixtures/head.sql");
        let suggested = suggest(before, after).unwrap();
        assert!(!suggested.contains("connected objects"));
        assert!(!suggested.contains("--"));
        assert_eq!(crate::sql::parse(&suggested).unwrap().len(), 3);
        crate::migration::check(
            &suggested,
            &[include_str!("../tests/fixtures/migration.sql").into()],
        )
        .unwrap();
    }
}
