// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::index::FieldName;
use crate::api::HashMap;
use crate::api::Varno;
use crate::nodecast;
use crate::postgres::var::find_one_var;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{
    default, direct_function_call, extension_sql, pg_extern, pg_guard, pg_sys, AnyElement,
    FromDatum, IntoDatum, PgList,
};
use std::ptr::addr_of_mut;

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";
const DEFAULT_SNIPPET_MAX_NUM_CHARS: i32 = 150;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetConfig {
    pub start_tag: String,
    pub end_tag: String,
    pub max_num_chars: usize,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum SnippetType {
    Text(FieldName, pg_sys::Oid, SnippetConfig),
    Positions(FieldName, pg_sys::Oid),
}

impl SnippetType {
    pub fn field(&self) -> &FieldName {
        match self {
            SnippetType::Text(field, _, _) => field,
            SnippetType::Positions(field, _) => field,
        }
    }

    pub fn funcoid(&self) -> pg_sys::Oid {
        match self {
            SnippetType::Text(_, funcoid, _) => *funcoid,
            SnippetType::Positions(_, funcoid) => *funcoid,
        }
    }

    pub fn nodeoid(&self) -> pg_sys::Oid {
        match self {
            SnippetType::Text(_, _, _) => pg_sys::TEXTOID,
            SnippetType::Positions(_, _) => pg_sys::INT4ARRAYOID,
        }
    }
}

struct Context<'a> {
planning_rti: pg_sys::Index,
    attname_lookup: &'a HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    snippet_funcoid: pg_sys::Oid,
    snippet_positions_funcoid: pg_sys::Oid,
    snippet_type: Vec<SnippetType>,
}

#[pg_extern(name = "snippet", stable, parallel_safe)]
fn snippet_from_relation(
    field: AnyElement,
    start_tag: default!(String, "'<b>'"),
    end_tag: default!(String, "'</b>'"),
    max_num_chars: default!(i32, "150"),
) -> Option<String> {
    None
}

#[pg_extern(name = "snippet_positions", stable, parallel_safe)]
fn snippet_positions_from_relation(field: AnyElement) -> Option<Vec<Vec<i32>>> {
    None
}

extension_sql!(
    r#"
ALTER FUNCTION snippet SUPPORT placeholder_support;
"#,
    name = "snippet_placeholder",
    requires = [snippet_from_relation, placeholder_support]
);

extension_sql!(
    r#"
ALTER FUNCTION snippet_positions SUPPORT placeholder_support;
"#,
    name = "snippet_positions_placeholder",
    requires = [snippet_positions_from_relation, placeholder_support]
);

pub fn snippet_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.snippet(anyelement, text, text, int)".into_datum()],
        )
        .expect("the `paradedb.snippet(anyelement, text, text, int) type should exist")
    }
}

pub fn snippet_positions_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.snippet_positions(anyelement)".into_datum()],
        )
        .expect("the `paradedb.snippet_positions(anyelement) type should exist")
    }
}

pub unsafe fn uses_snippets(
    rti: pg_sys::Index,
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    node: *mut pg_sys::Node,
    snippet_funcoid: pg_sys::Oid,
    snippet_positions_funcoid: pg_sys::Oid,
) -> Vec<SnippetType> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let context = data.cast::<Context>();

            if (*funcexpr).funcid == (*context).snippet_funcoid {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                if let Some(snippet_type) = extract_snippet_text(args, context) {
                    (*context).snippet_type.push(snippet_type);
                } else {
                    panic!("`paradedb.snippet()`'s arguments must be literals")
                }
            }

            if (*funcexpr).funcid == (*context).snippet_positions_funcoid {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                if let Some(snippet_type) = extract_snippet_positions(args, context) {
                    (*context).snippet_type.push(snippet_type);
                } else {
                    panic!("`paradedb.snippet_positions()`'s arguments must be literals")
                }
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    let mut context = Context {
        planning_rti,
        attname_lookup,
        snippet_funcoid,
        snippet_positions_funcoid,
        snippet_type: vec![],
    };

    walker(node, addr_of_mut!(context).cast());
    context.snippet_type
}

#[inline(always)]
unsafe fn extract_snippet_text(
    args: PgList<pg_sys::Node>,
    context: *mut Context,
) -> Option<SnippetType> {
    assert!(args.len() == 4);

    let field_arg = find_one_var(args.get_ptr(0).unwrap());
    let start_arg = nodecast!(Const, T_Const, args.get_ptr(1).unwrap());
    let end_arg = nodecast!(Const, T_Const, args.get_ptr(2).unwrap());
    let max_num_chars_arg = nodecast!(Const, T_Const, args.get_ptr(3).unwrap());

    if let (Some(field_arg), Some(start_arg), Some(end_arg), Some(max_num_chars_arg)) =
        (field_arg, start_arg, end_arg, max_num_chars_arg)
    {
        let attname = (*context)
            .attname_lookup
            .get(&((*context).planning_rti as _, (*field_arg).varattno as _))
            .cloned()
            .expect("Var attname should be in lookup");
        let start_tag = String::from_datum((*start_arg).constvalue, (*start_arg).constisnull);
        let end_tag = String::from_datum((*end_arg).constvalue, (*end_arg).constisnull);
        let max_num_chars = i32::from_datum(
            (*max_num_chars_arg).constvalue,
            (*max_num_chars_arg).constisnull,
        );

        Some(SnippetType::Text(
            attname,
            (*context).snippet_funcoid,
            SnippetConfig {
                start_tag: start_tag.unwrap_or_else(|| DEFAULT_SNIPPET_PREFIX.to_string()),
                end_tag: end_tag.unwrap_or_else(|| DEFAULT_SNIPPET_POSTFIX.to_string()),
                max_num_chars: max_num_chars.unwrap_or(DEFAULT_SNIPPET_MAX_NUM_CHARS) as usize,
            },
        ))
    } else {
        None
    }
}

#[inline(always)]
unsafe fn extract_snippet_positions(
    args: PgList<pg_sys::Node>,
    context: *mut Context,
) -> Option<SnippetType> {
    assert!(args.len() == 1);

    let field_arg = find_one_var(args.get_ptr(0).unwrap());

    if let Some(field_arg) = field_arg {
        let attname = (*context)
            .attname_lookup
            .get(&((*context).planning_rti as _, (*field_arg).varattno as _))
            .cloned()
            .expect("Var attname should be in lookup");

        Some(SnippetType::Positions(
            attname,
            (*context).snippet_positions_funcoid,
        ))
    } else {
        None
    }
}
