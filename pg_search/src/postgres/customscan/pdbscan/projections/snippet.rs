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

use std::ptr::addr_of_mut;

use crate::api::{FieldName, HashMap, Varno};
use crate::nodecast;
use crate::postgres::var::find_one_var;

use pgrx::pg_sys::expression_tree_walker;
use pgrx::{
    default, direct_function_call, extension_sql, pg_extern, pg_guard, pg_sys, AnyElement,
    FromDatum, IntoDatum, PgList,
};
use tantivy::snippet::{SnippetGenerator, SnippetSortOrder};

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";
const DEFAULT_SNIPPET_MAX_NUM_CHARS: i32 = 150;
const DEFAULT_SNIPPET_LIMIT: i32 = 5;
const DEFAULT_SNIPPET_OFFSET: i32 = 0;

/// The limit and offset for "fragments" (essentially, matches with a small amount of context).
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct FragmentPositionsConfig {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl FragmentPositionsConfig {
    pub fn limit(&self) -> Option<usize> {
        self.limit.map(|v| {
            assert!(v >= 0, "limit must not be negative");
            v as usize
        })
    }

    pub fn offset(&self) -> Option<usize> {
        self.offset.map(|v| {
            assert!(v >= 0, "offset must not be negative");
            v as usize
        })
    }
}

/// The limit and offset for snippets (the concatenation of multiple fragments up to a particular
/// size limit).
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetPositionsConfig {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl SnippetPositionsConfig {
    pub fn limit_or_default(&self) -> usize {
        let limit = self.limit.unwrap_or(DEFAULT_SNIPPET_LIMIT);
        assert!(limit >= 0, "limit must not be negative");
        limit as usize
    }

    pub fn offset_or_default(&self) -> usize {
        let offset = self.offset.unwrap_or(DEFAULT_SNIPPET_OFFSET);
        assert!(offset >= 0, "offset must not be negative");
        offset as usize
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetConfig {
    pub start_tag: String,
    pub end_tag: String,
    pub max_num_chars: usize,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum SnippetType {
    SingleText(FieldName, SnippetConfig, FragmentPositionsConfig),
    MultipleText(
        FieldName,
        SnippetConfig,
        SnippetPositionsConfig,
        SnippetSortOrder,
    ),
    Positions(FieldName, FragmentPositionsConfig),
}

impl SnippetType {
    pub fn field(&self) -> &FieldName {
        match self {
            SnippetType::SingleText(field, _, _) => field,
            SnippetType::MultipleText(field, _, _, _) => field,
            SnippetType::Positions(field, _) => field,
        }
    }

    pub fn nodeoid(&self) -> pg_sys::Oid {
        match self {
            SnippetType::SingleText(_, _, _) => pg_sys::TEXTOID,
            SnippetType::MultipleText(_, _, _, _) => pg_sys::TEXTARRAYOID,
            SnippetType::Positions(_, _) => pg_sys::INT4ARRAYOID,
        }
    }

    pub fn configure_generator(&self, generator: &mut SnippetGenerator) {
        match self {
            SnippetType::SingleText(_, config, positions_config) => {
                if positions_config.limit().is_some() || positions_config.offset().is_some() {
                    pg_sys::panic::ErrorReport::new(
                        pgrx::PgSqlErrorCode::ERRCODE_WARNING_DEPRECATED_FEATURE,
                        "using `limit` or `offset` with `pdb.snippet` is deprecated",
                        pgrx::function_name!(),
                    )
                        .set_detail("rather than using `pdb.snippet` with a `limit` and `offset`, please use the `pdb.snippets` function")
                        .set_hint("use `pdb.snippets` instead")
                        .report(pgrx::PgLogLevel::WARNING);
                }
                // Do not use a limit or offset unless they have been specified: otherwise we might
                // not highlight all matches in the configured `max_num_chars`.
                if let Some(limit) = positions_config.limit() {
                    generator.set_matches_limit(limit as usize);
                }
                if let Some(offset) = positions_config.offset() {
                    generator.set_matches_offset(offset as usize);
                }
                generator.set_max_num_chars(config.max_num_chars);
            }
            SnippetType::MultipleText(_, config, positions_config, sort_order) => {
                // We always use a (default) limit and offset for positions, as we might
                // potentially produce a huge array otherwise.
                generator.set_snippets_limit(positions_config.limit_or_default());
                generator.set_snippets_offset(positions_config.offset_or_default());
                generator.set_max_num_chars(config.max_num_chars);
                generator.set_sort_order(*sort_order);
            }
            SnippetType::Positions(_, positions_config) => {
                // Positions are expected to be fairly small, so we always render all of them by
                // default.
                if let Some(limit) = positions_config.limit() {
                    generator.set_matches_limit(limit as usize);
                }
                if let Some(offset) = positions_config.offset() {
                    generator.set_matches_offset(offset as usize);
                }
                // If SnippetType::Positions, set max_num_chars to u32::MAX because the entire doc must be considered
                // This assumes text fields can be no more than u32::MAX bytes.
                generator.set_max_num_chars(u32::MAX as usize);
            }
        };
    }
}

struct Context<'a> {
    planning_rti: pg_sys::Index,
    attname_lookup: &'a HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    snippet_funcoids: [pg_sys::Oid; 2],
    snippets_funcoids: [pg_sys::Oid; 2],
    snippet_positions_funcoids: [pg_sys::Oid; 2],
    snippet_type: Vec<SnippetType>,
}

#[pgrx::pg_schema]
mod pdb {
    use pgrx::{default, pg_extern, AnyElement};

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

    #[pg_extern(name = "snippets", stable, parallel_safe)]
    fn snippets_from_relation(
        field: AnyElement,
        start_tag: default!(String, "'<b>'"),
        end_tag: default!(String, "'</b>'"),
        max_num_chars: default!(i32, "150"),
        limit: default!(Option<i32>, "NULL"),
        offset: default!(Option<i32>, "NULL"),
        sort_by: default!(String, "'score'"),
    ) -> Option<Vec<String>> {
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
}

// In `0.19.0`, we renamed `paradedb.snippet*` functions to `pdb.snippet*`.
// This is a backwards compatibility shim to ensure that old queries continue to work.
#[warn(deprecated)]
#[pg_extern(name = "snippet", stable, parallel_safe)]
fn paradedb_snippet_from_relation(
    field: AnyElement,
    start_tag: default!(String, "'<b>'"),
    end_tag: default!(String, "'</b>'"),
    max_num_chars: default!(i32, "150"),
    limit: default!(Option<i32>, "NULL"),
    offset: default!(Option<i32>, "NULL"),
) -> Option<String> {
    None
}

#[warn(deprecated)]
#[pg_extern(name = "snippets", stable, parallel_safe)]
fn paradedb_snippets_from_relation(
    field: AnyElement,
    start_tag: default!(String, "'<b>'"),
    end_tag: default!(String, "'</b>'"),
    max_num_chars: default!(i32, "150"),
    limit: default!(Option<i32>, "NULL"),
    offset: default!(Option<i32>, "NULL"),
    sort_by: default!(String, "'score'"),
) -> Option<Vec<String>> {
    None
}

#[warn(deprecated)]
#[pg_extern(name = "snippet_positions", stable, parallel_safe)]
fn paradedb_snippet_positions_from_relation(
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
    requires = [pdb::snippet_from_relation, placeholder_support]
);

extension_sql!(
    r#"
    ALTER FUNCTION pdb.snippet_positions SUPPORT paradedb.placeholder_support;
    "#,
    name = "snippet_positions_placeholder",
    requires = [pdb::snippet_positions_from_relation, placeholder_support]
);

extension_sql!(
    r#"
    ALTER FUNCTION paradedb.snippet SUPPORT paradedb.placeholder_support;
    "#,
    name = "paradedb_snippet_placeholder",
    requires = [paradedb_snippet_from_relation, placeholder_support]
);

extension_sql!(
    r#"
    ALTER FUNCTION paradedb.snippet_positions SUPPORT paradedb.placeholder_support;
    "#,
    name = "paradedb_snippet_positions_placeholder",
    requires = [
        paradedb_snippet_positions_from_relation,
        placeholder_support
    ]
);

pub fn snippet_funcoids() -> [pg_sys::Oid; 2] {
    const SIGNATURES: &[&str; 2] = &[
        "pdb.snippet(anyelement, text, text, int, int, int)",
        "paradedb.snippet(anyelement, text, text, int, int, int)",
    ];
    get_snippet_funcoids(SIGNATURES)
}

pub fn snippets_funcoids() -> [pg_sys::Oid; 2] {
    const SIGNATURES: &[&str; 2] = &[
        "pdb.snippets(anyelement, text, text, int, int, int, text)",
        "paradedb.snippets(anyelement, text, text, int, int, int, text)",
    ];
    get_snippet_funcoids(SIGNATURES)
}

pub fn snippet_positions_funcoids() -> [pg_sys::Oid; 2] {
    const SIGNATURES: &[&str; 2] = &[
        "pdb.snippet_positions(anyelement, int, int)",
        "paradedb.snippet_positions(anyelement, int, int)",
    ];
    get_snippet_funcoids(SIGNATURES)
}

fn get_snippet_funcoids(signatures: &[&str; 2]) -> [pg_sys::Oid; 2] {
    unsafe {
        signatures
            .iter()
            .map(|signature| {
                let cstr =
                    std::ffi::CString::new(*signature).expect("signature contained interior NUL");
                direct_function_call::<pg_sys::Oid>(
                    pg_sys::regprocedurein,
                    &[cstr.as_c_str().into_datum()],
                )
                .unwrap_or_else(|| panic!("the `{}` function should exist", signature))
            })
            .collect::<Vec<pg_sys::Oid>>()
            .try_into()
            .expect("expected exactly 2 snippet funcoids")
    }
}

pub unsafe fn uses_snippets(
    planning_rti: pg_sys::Index,
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    node: *mut pg_sys::Node,
    snippet_funcoids: [pg_sys::Oid; 2],
    snippets_funcoids: [pg_sys::Oid; 2],
    snippet_positions_funcoids: [pg_sys::Oid; 2],
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

            if let Some(snippet_type) = extract_snippet(
                funcexpr,
                (*context).planning_rti,
                (*context).snippet_funcoids,
                (*context).attname_lookup,
            ) {
                (*context).snippet_type.push(snippet_type);
            }

            if let Some(snippet_type) = extract_snippets(
                funcexpr,
                (*context).planning_rti,
                (*context).snippets_funcoids,
                (*context).attname_lookup,
            ) {
                (*context).snippet_type.push(snippet_type);
            }

            if let Some(snippet_type) = extract_snippet_positions(
                funcexpr,
                (*context).planning_rti,
                (*context).snippet_positions_funcoids,
                (*context).attname_lookup,
            ) {
                (*context).snippet_type.push(snippet_type);
            }
        }

        expression_tree_walker(node, Some(walker), data)
    }

    let mut context = Context {
        planning_rti,
        attname_lookup,
        snippet_funcoids,
        snippets_funcoids,
        snippet_positions_funcoids,
        snippet_type: vec![],
    };

    walker(node, addr_of_mut!(context).cast());
    context.snippet_type
}

#[inline(always)]
pub unsafe fn extract_snippet(
    func: *mut pg_sys::FuncExpr,
    planning_rti: pg_sys::Index,
    snippet_funcoids: [pg_sys::Oid; 2],
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
) -> Option<SnippetType> {
    if !snippet_funcoids.iter().any(|&oid| oid == (*func).funcid) {
        return None;
    }
    let args = PgList::<pg_sys::Node>::from_pg((*func).args);
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

        Some(SnippetType::SingleText(
            attname,
            SnippetConfig {
                start_tag: start_tag.unwrap_or_else(|| DEFAULT_SNIPPET_PREFIX.to_string()),
                end_tag: end_tag.unwrap_or_else(|| DEFAULT_SNIPPET_POSTFIX.to_string()),
                max_num_chars: max_num_chars.unwrap_or(DEFAULT_SNIPPET_MAX_NUM_CHARS) as usize,
            },
            FragmentPositionsConfig { limit, offset },
        ))
    } else {
        panic!("`pdb.snippets()`'s arguments must be literals")
    }
}

#[inline(always)]
pub unsafe fn extract_snippets(
    func: *mut pg_sys::FuncExpr,
    planning_rti: pg_sys::Index,
    snippets_funcoids: [pg_sys::Oid; 2],
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
) -> Option<SnippetType> {
    if !snippets_funcoids.iter().any(|&oid| oid == (*func).funcid) {
        return None;
    }
    let args = PgList::<pg_sys::Node>::from_pg((*func).args);
    assert!(args.len() == 7);

    let field_arg = find_one_var(args.get_ptr(0).unwrap());
    let start_arg = nodecast!(Const, T_Const, args.get_ptr(1).unwrap());
    let end_arg = nodecast!(Const, T_Const, args.get_ptr(2).unwrap());
    let max_num_chars_arg = nodecast!(Const, T_Const, args.get_ptr(3).unwrap());
    let limit_arg = nodecast!(Const, T_Const, args.get_ptr(4).unwrap());
    let offset_arg = nodecast!(Const, T_Const, args.get_ptr(5).unwrap());
    let sort_by_arg = nodecast!(Const, T_Const, args.get_ptr(6).unwrap());

    if let (
        Some(field_arg),
        Some(start_arg),
        Some(end_arg),
        Some(max_num_chars_arg),
        Some(limit_arg),
        Some(offset_arg),
        Some(sort_by_arg),
    ) = (
        field_arg,
        start_arg,
        end_arg,
        max_num_chars_arg,
        limit_arg,
        offset_arg,
        sort_by_arg,
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
        let sort_by = String::from_datum((*sort_by_arg).constvalue, (*sort_by_arg).constisnull)
            .unwrap_or_else(|| "score".to_string());

        let sort_order = match sort_by.as_str() {
            "score" => SnippetSortOrder::Score,
            "position" => SnippetSortOrder::Position,
            _ => panic!("invalid sort_by value for pdb.snippets: must be 'score' or 'position'"),
        };

        Some(SnippetType::MultipleText(
            attname,
            SnippetConfig {
                start_tag: start_tag.unwrap_or_else(|| DEFAULT_SNIPPET_PREFIX.to_string()),
                end_tag: end_tag.unwrap_or_else(|| DEFAULT_SNIPPET_POSTFIX.to_string()),
                max_num_chars: max_num_chars.unwrap_or(DEFAULT_SNIPPET_MAX_NUM_CHARS) as usize,
            },
            SnippetPositionsConfig { limit, offset },
            sort_order,
        ))
    } else {
        panic!("`pdb.snippets()`'s arguments must be literals")
    }
}

#[inline(always)]
pub unsafe fn extract_snippet_positions(
    func: *mut pg_sys::FuncExpr,
    planning_rti: pg_sys::Index,
    snippet_positions_funcoids: [pg_sys::Oid; 2],
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
) -> Option<SnippetType> {
    if !snippet_positions_funcoids
        .iter()
        .any(|&oid| oid == (*func).funcid)
    {
        return None;
    }
    let args = PgList::<pg_sys::Node>::from_pg((*func).args);
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
            FragmentPositionsConfig { limit, offset },
        ))
    } else {
        panic!("`pdb.extract_snippet_positions()`'s arguments must be literals")
    }
}
