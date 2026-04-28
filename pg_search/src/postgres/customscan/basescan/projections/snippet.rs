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

use std::ptr::addr_of_mut;

use crate::api::{FieldName, HashMap, Varno};
use crate::nodecast;
use crate::postgres::customscan::parameterized_value::ParameterizedValue;
use crate::postgres::var::find_one_var;

use pgrx::pg_sys::expression_tree_walker;
use pgrx::{
    default, direct_function_call, extension_sql, pg_extern, pg_guard, pg_sys, AnyElement,
    IntoDatum, PgList,
};
use std::sync::OnceLock;
use tantivy::snippet::{SnippetGenerator, SnippetSortOrder};

const DEFAULT_SNIPPET_PREFIX: &str = "<b>";
const DEFAULT_SNIPPET_POSTFIX: &str = "</b>";
const DEFAULT_SNIPPET_MAX_NUM_CHARS: i32 = 150;
const DEFAULT_SNIPPET_LIMIT: i32 = 5;
const DEFAULT_SNIPPET_OFFSET: i32 = 0;

/// The limit and offset for "fragments" (essentially, matches with a small amount of context).
///
/// Both fields are wrapped in `ParameterizedValue` so they can carry either a
/// planning-time `Const` or an extern `Param` resolved at execution time.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct FragmentPositionsConfig {
    pub limit: Option<ParameterizedValue<i32>>,
    pub offset: Option<ParameterizedValue<i32>>,
}

impl FragmentPositionsConfig {
    /// Resolve the LIMIT against the executor state. Returns `None` if there is
    /// no LIMIT or the parameter resolves to NULL.
    pub unsafe fn resolve_limit(&self, estate: *mut pg_sys::EState) -> Option<usize> {
        self.limit.as_ref().and_then(|v| {
            v.resolve(estate).map(|raw| {
                assert!(raw >= 0, "limit must not be negative");
                raw as usize
            })
        })
    }

    /// Resolve the OFFSET against the executor state. Returns `None` if there is
    /// no OFFSET or the parameter resolves to NULL.
    pub unsafe fn resolve_offset(&self, estate: *mut pg_sys::EState) -> Option<usize> {
        self.offset.as_ref().and_then(|v| {
            v.resolve(estate).map(|raw| {
                assert!(raw >= 0, "offset must not be negative");
                raw as usize
            })
        })
    }
}

/// The limit and offset for snippets (the concatenation of multiple fragments up to a particular
/// size limit).
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetPositionsConfig {
    pub limit: Option<ParameterizedValue<i32>>,
    pub offset: Option<ParameterizedValue<i32>>,
}

impl SnippetPositionsConfig {
    /// Resolve the LIMIT, falling back to `DEFAULT_SNIPPET_LIMIT` when absent or NULL.
    pub unsafe fn resolve_limit_or_default(&self, estate: *mut pg_sys::EState) -> usize {
        let limit = self
            .limit
            .as_ref()
            .and_then(|v| v.resolve(estate))
            .unwrap_or(DEFAULT_SNIPPET_LIMIT);
        assert!(limit >= 0, "limit must not be negative");
        limit as usize
    }

    /// Resolve the OFFSET, falling back to `DEFAULT_SNIPPET_OFFSET` when absent or NULL.
    pub unsafe fn resolve_offset_or_default(&self, estate: *mut pg_sys::EState) -> usize {
        let offset = self
            .offset
            .as_ref()
            .and_then(|v| v.resolve(estate))
            .unwrap_or(DEFAULT_SNIPPET_OFFSET);
        assert!(offset >= 0, "offset must not be negative");
        offset as usize
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct SnippetConfig {
    pub start_tag: ParameterizedValue<String>,
    pub end_tag: ParameterizedValue<String>,
    pub max_num_chars: ParameterizedValue<i32>,
}

impl SnippetConfig {
    pub unsafe fn resolve_start_tag(&self, estate: *mut pg_sys::EState) -> String {
        resolve_tag_or_default(&self.start_tag, estate, DEFAULT_SNIPPET_PREFIX)
    }

    pub unsafe fn resolve_end_tag(&self, estate: *mut pg_sys::EState) -> String {
        resolve_tag_or_default(&self.end_tag, estate, DEFAULT_SNIPPET_POSTFIX)
    }

    pub unsafe fn resolve_max_num_chars(&self, estate: *mut pg_sys::EState) -> usize {
        let v = self
            .max_num_chars
            .resolve(estate)
            .unwrap_or(DEFAULT_SNIPPET_MAX_NUM_CHARS);
        assert!(v >= 0, "max_num_chars must not be negative");
        v as usize
    }
}

unsafe fn resolve_tag_or_default(
    tag: &ParameterizedValue<String>,
    estate: *mut pg_sys::EState,
    default: &str,
) -> String {
    tag.resolve(estate).unwrap_or_else(|| default.to_string())
}

// TODO: `SnippetType` is used as a `HashMap` key, so `SnippetConfig` fields
// (which contain `ParameterizedValue<String>`) cannot use `resolve_mut` to
// convert Param → Static in place — mutating a key would corrupt the map.
// The clean fix is to separate identity (field name + param IDs → key) from
// mutable config (resolved tags, generator, const nodes → value) into a
// single `HashMap<SnippetId, SnippetState>`. Until then, snippet resolution
// uses `resolve()` (clones per call) instead of `resolve_mut()`.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum SnippetType {
    SingleText(FieldName, SnippetConfig, FragmentPositionsConfig),
    MultipleText(
        FieldName,
        SnippetConfig,
        SnippetPositionsConfig,
        // Sort order is held as a string so a parameterized `sort_by` arg can
        // reach execution time before being converted to `SnippetSortOrder`.
        ParameterizedValue<String>,
    ),
    Positions(FieldName, FragmentPositionsConfig),
}

/// Parse a `sort_by` string into a `SnippetSortOrder`. NULL falls back to the
/// default (`Score`); any other value reports a SQL error at execution time
/// (parameterized values can't be validated at planning time).
fn parse_sort_order(s: Option<&str>) -> SnippetSortOrder {
    match s {
        None | Some("score") => SnippetSortOrder::Score,
        Some("position") => SnippetSortOrder::Position,
        Some(_) => {
            pgrx::error!("invalid sort_by value for pdb.snippets: must be 'score' or 'position'")
        }
    }
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
            SnippetType::Positions(_, _) => pg_sys::INT4ARRAYOID, // integer[][]
        }
    }

    pub unsafe fn configure_generator(
        &self,
        generator: &mut SnippetGenerator,
        estate: *mut pg_sys::EState,
    ) {
        match self {
            SnippetType::SingleText(_, config, positions_config) => {
                let limit = positions_config.resolve_limit(estate);
                let offset = positions_config.resolve_offset(estate);
                if limit.is_some() || offset.is_some() {
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
                if let Some(limit) = limit {
                    generator.set_matches_limit(limit);
                }
                if let Some(offset) = offset {
                    generator.set_matches_offset(offset);
                }
                generator.set_max_num_chars(config.resolve_max_num_chars(estate));
            }
            SnippetType::MultipleText(_, config, positions_config, sort_by) => {
                // We always use a (default) limit and offset for positions, as we might
                // potentially produce a huge array otherwise.
                generator.set_snippets_limit(positions_config.resolve_limit_or_default(estate));
                generator.set_snippets_offset(positions_config.resolve_offset_or_default(estate));
                generator.set_max_num_chars(config.resolve_max_num_chars(estate));
                let resolved = sort_by.resolve(estate);
                generator.set_sort_order(parse_sort_order(resolved.as_deref()));
            }
            SnippetType::Positions(_, positions_config) => {
                // Positions are expected to be fairly small, so we always render all of them by
                // default.
                if let Some(limit) = positions_config.resolve_limit(estate) {
                    generator.set_matches_limit(limit);
                }
                if let Some(offset) = positions_config.resolve_offset(estate) {
                    generator.set_matches_offset(offset);
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
pub mod pdb {
    use pgrx::callconv::{BoxRet, FcInfo};
    use pgrx::datum::Datum;
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, ReturnsError, ReturnsRef, SqlMappingRef, SqlTranslatable, TypeOrigin,
    };
    use pgrx::{default, pg_extern, pg_sys, AnyElement, IntoDatum};

    // Newtype wrapper for Vec<Vec<i32>> to implement custom IntoDatum
    // This ensures it serializes as a proper 2D PostgreSQL integer array
    // instead of an array of JSON strings.
    //
    // Note: PostgreSQL doesn't differentiate between integer[] and integer[][]
    // at the type level - both are represented as integer[] (internally _int4).
    // The difference is in the array dimensions metadata stored with each value.
    #[repr(transparent)]
    #[derive(Clone)]
    pub struct IntArray2D(pub Vec<Vec<i32>>);

    impl IntoDatum for IntArray2D {
        fn into_datum(self) -> Option<pg_sys::Datum> {
            if self.0.is_empty() {
                return Some(pg_sys::Datum::from(
                    std::ptr::null_mut::<pg_sys::ArrayType>(),
                ));
            }

            unsafe {
                // Flatten the 2D array and collect dimensions
                let mut datums: Vec<pg_sys::Datum> = Vec::new();
                let mut nulls: Vec<bool> = Vec::new();

                let outer_len = self.0.len();
                let inner_len = if outer_len > 0 { self.0[0].len() } else { 0 };

                for row in &self.0 {
                    for &val in row {
                        datums.push(val.into_datum().unwrap());
                        nulls.push(false);
                    }
                }

                // Set up dimensions: [outer_dim, inner_dim]
                let dims = [outer_len as i32, inner_len as i32];
                let lbs = [1i32, 1i32]; // Lower bounds are 1 for PostgreSQL arrays

                // Construct the 2D array using pg_sys::construct_md_array
                let array_datum = pg_sys::construct_md_array(
                    datums.as_mut_ptr(),
                    nulls.as_mut_ptr(),
                    2, // ndims (2D array)
                    dims.as_ptr() as *mut i32,
                    lbs.as_ptr() as *mut i32,
                    pg_sys::INT4OID, // element type OID (integer)
                    4,               // typlen (int4 is 4 bytes)
                    true,            // typbyval (int4 is passed by value)
                    #[allow(clippy::useless_conversion)]
                    pg_sys::TYPALIGN_INT.try_into().unwrap(), // typalign (char type, architecture-specific)
                );

                Some(pg_sys::Datum::from(array_datum))
            }
        }

        fn type_oid() -> pg_sys::Oid {
            pg_sys::INT4ARRAYOID
        }
    }

    unsafe impl SqlTranslatable for IntArray2D {
        const TYPE_IDENT: &'static str = pgrx::pgrx_resolved_type!(IntArray2D);
        const TYPE_ORIGIN: TypeOrigin = TypeOrigin::External;
        const ARGUMENT_SQL: Result<SqlMappingRef, ArgumentError> =
            Ok(SqlMappingRef::literal("integer[]"));
        const RETURN_SQL: Result<ReturnsRef, ReturnsError> =
            Ok(ReturnsRef::One(SqlMappingRef::literal("integer[]")));
    }

    unsafe impl BoxRet for IntArray2D {
        unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
            self.into_datum()
                .map(|datum| fcinfo.return_raw_datum(datum))
                .unwrap_or_else(Datum::null)
        }
    }

    #[allow(unused_variables)]
    #[pg_extern(name = "snippet", stable, parallel_safe)]
    fn snippet_from_relation(
        field: AnyElement,
        start_tag: default!(String, "'<b>'"),
        end_tag: default!(String, "'</b>'"),
        max_num_chars: default!(i32, "150"),
        limit: default!(Option<i32>, "NULL"),
        offset: default!(Option<i32>, "NULL"),
    ) -> String {
        panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
    }

    #[allow(unused_variables)]
    #[pg_extern(name = "snippets", stable, parallel_safe)]
    fn snippets_from_relation(
        field: AnyElement,
        start_tag: default!(String, "'<b>'"),
        end_tag: default!(String, "'</b>'"),
        max_num_chars: default!(i32, "150"),
        limit: default!(Option<i32>, "NULL"),
        offset: default!(Option<i32>, "NULL"),
        sort_by: default!(String, "'score'"),
    ) -> Vec<String> {
        panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
    }

    #[allow(unused_variables)]
    #[pg_extern(
        name = "snippet_positions",
        stable,
        parallel_safe,
        sql = r#"
CREATE OR REPLACE FUNCTION "pdb"."snippet_positions"(
    "field" anyelement,
    "limit" INT DEFAULT NULL,
    "offset" INT DEFAULT NULL
) RETURNS integer[]  -- Note: PostgreSQL doesn't distinguish integer[] from integer[][] at the type level
STABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'snippet_positions_from_relation_wrapper';
"#
    )]
    fn snippet_positions_from_relation(
        field: AnyElement,
        limit: default!(Option<i32>, "NULL"),
        offset: default!(Option<i32>, "NULL"),
    ) -> IntArray2D {
        panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
    }
}

// In `0.19.0`, we renamed `paradedb.snippet*` functions to `pdb.snippet*`.
// This is a backwards compatibility shim to ensure that old queries continue to work.
#[warn(deprecated)]
#[allow(unused_variables)]
#[pg_extern(name = "snippet", stable, parallel_safe)]
fn paradedb_snippet_from_relation(
    field: AnyElement,
    start_tag: default!(String, "'<b>'"),
    end_tag: default!(String, "'</b>'"),
    max_num_chars: default!(i32, "150"),
    limit: default!(Option<i32>, "NULL"),
    offset: default!(Option<i32>, "NULL"),
) -> Option<String> {
    panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
}

#[warn(deprecated)]
#[allow(unused_variables)]
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
    panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
}

#[warn(deprecated)]
#[allow(unused_variables)]
#[pg_extern(
    name = "snippet_positions",
    stable,
    parallel_safe,
    sql = r#"
CREATE OR REPLACE FUNCTION "paradedb"."snippet_positions"(
    "field" anyelement,
    "limit" INT DEFAULT NULL,
    "offset" INT DEFAULT NULL
) RETURNS integer[]
STABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'paradedb_snippet_positions_from_relation_wrapper';
"#
)]
fn paradedb_snippet_positions_from_relation(
    field: AnyElement,
    limit: default!(Option<i32>, "NULL"),
    offset: default!(Option<i32>, "NULL"),
) -> pdb::IntArray2D {
    panic!("Unsupported query shape. Please report at https://github.com/paradedb/paradedb/issues/new/choose");
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
    static OID_CACHE: OnceLock<[pg_sys::Oid; 2]> = OnceLock::new();
    *OID_CACHE.get_or_init(|| {
        resolve_funcoids(&[
            "pdb.snippet(anyelement, text, text, int, int, int)",
            "paradedb.snippet(anyelement, text, text, int, int, int)",
        ])
    })
}

pub fn snippets_funcoids() -> [pg_sys::Oid; 2] {
    static OID_CACHE: OnceLock<[pg_sys::Oid; 2]> = OnceLock::new();
    *OID_CACHE.get_or_init(|| {
        resolve_funcoids(&[
            "pdb.snippets(anyelement, text, text, int, int, int, text)",
            "paradedb.snippets(anyelement, text, text, int, int, int, text)",
        ])
    })
}

pub fn snippet_positions_funcoids() -> [pg_sys::Oid; 2] {
    static OID_CACHE: OnceLock<[pg_sys::Oid; 2]> = OnceLock::new();
    *OID_CACHE.get_or_init(|| {
        resolve_funcoids(&[
            "pdb.snippet_positions(anyelement, int, int)",
            "paradedb.snippet_positions(anyelement, int, int)",
        ])
    })
}

fn resolve_funcoids(signatures: &[&str; 2]) -> [pg_sys::Oid; 2] {
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

/// Resolve the field arg (always arg 0) of a snippet function to its
/// indexed `FieldName`. Returns `None` if the arg isn't a single Var.
#[inline]
unsafe fn extract_snippet_field_attname(
    args: &PgList<pg_sys::Node>,
    planning_rti: pg_sys::Index,
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
) -> Option<FieldName> {
    let field_arg = find_one_var(args.get_ptr(0).unwrap())?;
    Some(
        attname_lookup
            .get(&(planning_rti as _, (*field_arg).varattno as _))
            .cloned()
            .expect("Var attname should be in lookup"),
    )
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

    let attname = extract_snippet_field_attname(&args, planning_rti, attname_lookup)?;

    // All formatting/limit args may be either Const (resolved at planning
    // time) or extern Param (resolved at execution time in GENERIC plan
    // mode). The legacy code panicked on Param here.
    let start_tag = ParameterizedValue::<String>::from_node(args.get_ptr(1).unwrap())
        .unwrap_or_else(|| ParameterizedValue::Static(DEFAULT_SNIPPET_PREFIX.to_string()));
    let end_tag = ParameterizedValue::<String>::from_node(args.get_ptr(2).unwrap())
        .unwrap_or_else(|| ParameterizedValue::Static(DEFAULT_SNIPPET_POSTFIX.to_string()));
    let max_num_chars = ParameterizedValue::<i32>::from_node(args.get_ptr(3).unwrap())
        .unwrap_or(ParameterizedValue::Static(DEFAULT_SNIPPET_MAX_NUM_CHARS));
    let limit = ParameterizedValue::<i32>::from_node(args.get_ptr(4).unwrap());
    let offset = ParameterizedValue::<i32>::from_node(args.get_ptr(5).unwrap());

    Some(SnippetType::SingleText(
        attname,
        SnippetConfig {
            start_tag,
            end_tag,
            max_num_chars,
        },
        FragmentPositionsConfig { limit, offset },
    ))
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

    let attname = extract_snippet_field_attname(&args, planning_rti, attname_lookup)?;

    let start_tag = ParameterizedValue::<String>::from_node(args.get_ptr(1).unwrap())
        .unwrap_or_else(|| ParameterizedValue::Static(DEFAULT_SNIPPET_PREFIX.to_string()));
    let end_tag = ParameterizedValue::<String>::from_node(args.get_ptr(2).unwrap())
        .unwrap_or_else(|| ParameterizedValue::Static(DEFAULT_SNIPPET_POSTFIX.to_string()));
    let max_num_chars = ParameterizedValue::<i32>::from_node(args.get_ptr(3).unwrap())
        .unwrap_or(ParameterizedValue::Static(DEFAULT_SNIPPET_MAX_NUM_CHARS));
    let limit = ParameterizedValue::<i32>::from_node(args.get_ptr(4).unwrap());
    let offset = ParameterizedValue::<i32>::from_node(args.get_ptr(5).unwrap());
    // sort_by is a String at the SQL level; we defer the conversion to
    // SnippetSortOrder until execution time so a parameterized value can
    // also flow through.
    let sort_by = ParameterizedValue::<String>::from_node(args.get_ptr(6).unwrap())
        .unwrap_or_else(|| ParameterizedValue::Static("score".to_string()));

    Some(SnippetType::MultipleText(
        attname,
        SnippetConfig {
            start_tag,
            end_tag,
            max_num_chars,
        },
        SnippetPositionsConfig { limit, offset },
        sort_by,
    ))
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

    let attname = extract_snippet_field_attname(&args, planning_rti, attname_lookup)?;

    let limit = ParameterizedValue::<i32>::from_node(args.get_ptr(1).unwrap());
    let offset = ParameterizedValue::<i32>::from_node(args.get_ptr(2).unwrap());

    Some(SnippetType::Positions(
        attname,
        FragmentPositionsConfig { limit, offset },
    ))
}
