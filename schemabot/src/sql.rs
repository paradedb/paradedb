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
    let input = input.replace("@extschema@", "\"@extschema@\"");
    let result = pg_query::parse(&input).map_err(|error| format!("SQL parse error: {error}"))?;
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
