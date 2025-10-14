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

use crate::api::FieldName;
use crate::api::HashMap;
use crate::api::Varno;
use crate::nodecast;
use crate::postgres::var::find_one_var;
use pgrx::pg_sys::expression_tree_walker;
use pgrx::{direct_function_call, pg_guard, pg_sys, FromDatum, IntoDatum, PgList};
use std::ptr::addr_of_mut;

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";
const DEFAULT_SNIPPET_MAX_NUM_CHARS: i32 = 150;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetPositionsConfig {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetConfig {
    pub start_tag: String,
    pub end_tag: String,
    pub max_num_chars: usize,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum SnippetType {
    Text(
        FieldName,
        pg_sys::Oid,
        SnippetConfig,
        SnippetPositionsConfig,
    ),
    Positions(FieldName, pg_sys::Oid, SnippetPositionsConfig),
}

impl SnippetType {
    pub fn field(&self) -> &FieldName {
        match self {
            SnippetType::Text(field, _, _, _) => field,
            SnippetType::Positions(field, _, _) => field,
        }
    }

    pub fn funcoid(&self) -> pg_sys::Oid {
        match self {
            SnippetType::Text(_, funcoid, _, _) => *funcoid,
            SnippetType::Positions(_, funcoid, _) => *funcoid,
        }
    }

    pub fn nodeoid(&self) -> pg_sys::Oid {
        match self {
            SnippetType::Text(_, _, _, _) => pg_sys::TEXTOID,
            SnippetType::Positions(_, _, _) => pg_sys::INT4ARRAYOID,
        }
    }

    pub fn limit(&self) -> Option<i32> {
        let limit = match self {
            SnippetType::Text(_, _, _, positions_config) => positions_config.limit,
            SnippetType::Positions(_, _, positions_config) => positions_config.limit,
        };

        assert!(limit.unwrap_or(0) >= 0, "limit must not be negative");
        limit
    }

    pub fn offset(&self) -> Option<i32> {
        let offset = match self {
            SnippetType::Text(_, _, _, positions_config) => positions_config.offset,
            SnippetType::Positions(_, _, positions_config) => positions_config.offset,
        };

        assert!(offset.unwrap_or(0) >= 0, "offset must not be negative");
        offset
    }
}

struct Context<'a> {
    planning_rti: pg_sys::Index,
    attname_lookup: &'a HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    snippet_funcoid: pg_sys::Oid,
    snippet_positions_funcoid: pg_sys::Oid,
    snippet_type: Vec<SnippetType>,
}

#[pgrx::pg_schema]
mod pdb {
    use pgrx::{default, extension_sql, pg_extern, AnyElement};

    #[pg_extern(name = "snippet", stable, parallel_safe)]
    fn snippet_from_relation(
        field: AnyElement,
        start_tag: default!(String, "'<b>'"),
        end_tag: default!(String, "'</b>'"),
        max_num_chars: default!(i32, "150"),
        limit: default!(Option<i32>, "NULL"),
        offset: default!(Option<i32>, "NULL"),
    ) -> Option<String> {
        None
    }

    #[pg_extern(name = "snippet_positions", stable, parallel_safe)]
    fn snippet_positions_from_relation(
        field: AnyElement,
        limit: default!(Option<i32>, "NULL"),
        offset: default!(Option<i32>, "NULL"),
    ) -> Option<Vec<Vec<i32>>> {
        None
    }

    extension_sql!(
        r#"
    ALTER FUNCTION pdb.snippet SUPPORT paradedb.placeholder_support;
    "#,
        name = "snippet_placeholder",
        requires = [snippet_from_relation, placeholder_support]
    );

    extension_sql!(
        r#"
    ALTER FUNCTION pdb.snippet_positions SUPPORT paradedb.placeholder_support;
    "#,
        name = "snippet_positions_placeholder",
        requires = [snippet_positions_from_relation, placeholder_support]
    );
}

pub fn snippet_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"pdb.snippet(anyelement, text, text, int, int, int)".into_datum()],
        )
        .expect("the `pdb.snippet(anyelement, text, text, int, int, int) type should exist")
    }
}

pub fn snippet_positions_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"pdb.snippet_positions(anyelement, int, int)".into_datum()],
        )
        .expect("the `pdb.snippet_positions(anyelement, int, int) type should exist")
    }
}

pub unsafe fn uses_snippets(
    planning_rti: pg_sys::Index,
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
                if let Some(snippet_type) = extract_snippet_text(
                    args,
                    (*context).planning_rti,
                    (*context).snippet_funcoid,
                    (*context).attname_lookup,
                ) {
                    (*context).snippet_type.push(snippet_type);
                } else {
                    panic!("`pdb.snippet()`'s arguments must be literals")
                }
            }

            if (*funcexpr).funcid == (*context).snippet_positions_funcoid {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                if let Some(snippet_type) = extract_snippet_positions(
                    args,
                    (*context).planning_rti,
                    (*context).snippet_positions_funcoid,
                    (*context).attname_lookup,
                ) {
                    (*context).snippet_type.push(snippet_type);
                } else {
                    panic!("`pdb.snippet_positions()`'s arguments must be literals")
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
pub unsafe fn extract_snippet_text(
    args: PgList<pg_sys::Node>,
    planning_rti: pg_sys::Index,
    snippet_funcoid: pg_sys::Oid,
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
) -> Option<SnippetType> {
    assert!(args.len() == 6);

    let field_arg = find_one_var(args.get_ptr(0).unwrap());
    let start_arg = nodecast!(Const, T_Const, args.get_ptr(1).unwrap());
    let end_arg = nodecast!(Const, T_Const, args.get_ptr(2).unwrap());
    let max_num_chars_arg = nodecast!(Const, T_Const, args.get_ptr(3).unwrap());
    let limit_arg = nodecast!(Const, T_Const, args.get_ptr(4).unwrap());
    let offset_arg = nodecast!(Const, T_Const, args.get_ptr(5).unwrap());

    if let (
        Some(field_arg),
        Some(start_arg),
        Some(end_arg),
        Some(max_num_chars_arg),
        Some(limit_arg),
        Some(offset_arg),
    ) = (
        field_arg,
        start_arg,
        end_arg,
        max_num_chars_arg,
        limit_arg,
        offset_arg,
    ) {
        let attname = attname_lookup
            .get(&(planning_rti as _, (*field_arg).varattno as _))
            .cloned()
            .expect("Var attname should be in lookup");
        let start_tag = String::from_datum((*start_arg).constvalue, (*start_arg).constisnull);
        let end_tag = String::from_datum((*end_arg).constvalue, (*end_arg).constisnull);
        let max_num_chars = i32::from_datum(
            (*max_num_chars_arg).constvalue,
            (*max_num_chars_arg).constisnull,
        );
        let limit = i32::from_datum((*limit_arg).constvalue, (*limit_arg).constisnull);
        let offset = i32::from_datum((*offset_arg).constvalue, (*offset_arg).constisnull);

        Some(SnippetType::Text(
            attname,
            snippet_funcoid,
            SnippetConfig {
                start_tag: start_tag.unwrap_or_else(|| DEFAULT_SNIPPET_PREFIX.to_string()),
                end_tag: end_tag.unwrap_or_else(|| DEFAULT_SNIPPET_POSTFIX.to_string()),
                max_num_chars: max_num_chars.unwrap_or(DEFAULT_SNIPPET_MAX_NUM_CHARS) as usize,
            },
            SnippetPositionsConfig { limit, offset },
        ))
    } else {
        None
    }
}

#[inline(always)]
pub unsafe fn extract_snippet_positions(
    args: PgList<pg_sys::Node>,
    planning_rti: pg_sys::Index,
    snippet_positions_funcoid: pg_sys::Oid,
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
) -> Option<SnippetType> {
    assert!(args.len() == 3);

    let field_arg = find_one_var(args.get_ptr(0).unwrap());
    let limit_arg = nodecast!(Const, T_Const, args.get_ptr(1).unwrap());
    let offset_arg = nodecast!(Const, T_Const, args.get_ptr(2).unwrap());

    if let (Some(field_arg), Some(limit_arg), Some(offset_arg)) = (field_arg, limit_arg, offset_arg)
    {
        let attname = attname_lookup
            .get(&(planning_rti as _, (*field_arg).varattno as _))
            .cloned()
            .expect("Var attname should be in lookup");

        let limit = i32::from_datum((*limit_arg).constvalue, (*limit_arg).constisnull);
        let offset = i32::from_datum((*offset_arg).constvalue, (*offset_arg).constisnull);

        Some(SnippetType::Positions(
            attname,
            snippet_positions_funcoid,
            SnippetPositionsConfig { limit, offset },
        ))
    } else {
        None
    }
}
