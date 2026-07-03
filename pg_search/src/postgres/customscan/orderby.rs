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

//! Shared utilities for analyzing sort expressions in `ORDER BY` clauses.
//!
//! This module is shared between custom scans (BaseScan, AggregateScan) and the planner hook
//! to ensure that Top K compatibility validation logic is consistent across the codebase.
//!
//! This sharing is required to workaround <https://github.com/paradedb/paradedb/issues/3455>,
//! ensuring that we only replace window functions with ParadeDB placeholders
//! when we are certain that the query can be executed as a Top K query.

use crate::api::FieldName;
use crate::index::directory::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, MAX_TOPK_FEATURES};
use crate::nodecast;
use crate::postgres::catalog::{
    lookup_collation_locale, lookup_database_collation_locale, CollationLocale, CollationProvider,
};
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::options::{SortByDirection, SortByField};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::{
    fieldname_from_var, find_one_var_and_fieldname, strip_identity_wrappers, VarContext,
};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList};
use std::cell::RefCell;

/// The type of sort expression found in an ORDER BY clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortExpressionType {
    /// Sorting by search score: `ORDER BY pdb.score(...)`
    Score,
    /// Sorting by a lowercased field: `ORDER BY lower(col)`
    Lower,
    /// Sorting by a raw field: `ORDER BY col`
    Raw,
    /// Sorting by an expression that matched an indexed expression in `pg_index.indexprs`.
    IndexedExpression,
}

/// Reason why pathkeys cannot be used for Top K execution
#[derive(Debug, Clone)]
pub enum UnusableReason {
    /// ORDER BY has too many columns (more than MAX_TOPK_FEATURES)
    TooManyColumns { count: usize, max: usize },
    /// Only a prefix of the ORDER BY columns can be pushed down
    PrefixOnly { matched: usize },
    /// Columns are not indexed with fast=true or not sortable
    NotSortable,
    /// We cannot pushdown collations that are not byte-ordered (C-like)
    UnsafeCollation,
}

#[derive(Debug, Clone)]
pub enum PathKeyInfo {
    /// There are no PathKeys at all.
    None,
    /// There were PathKeys, but we cannot execute them.
    Unusable(UnusableReason),
    /// There were PathKeys, but we can only execute a non-empty prefix of them.
    UsablePrefix(Vec<OrderByStyle>),
    /// There are some PathKeys, and we can execute all of them.
    UsableAll(Vec<OrderByStyle>),
}

impl PathKeyInfo {
    pub fn is_usable(&self) -> bool {
        match self {
            PathKeyInfo::UsablePrefix(_) | PathKeyInfo::UsableAll(_) => true,
            PathKeyInfo::None | PathKeyInfo::Unusable(_) => false,
        }
    }

    pub fn pathkeys(&self) -> Option<&Vec<OrderByStyle>> {
        match self {
            PathKeyInfo::UsablePrefix(pathkeys) | PathKeyInfo::UsableAll(pathkeys) => {
                Some(pathkeys)
            }
            PathKeyInfo::None | PathKeyInfo::Unusable(_) => None,
        }
    }
}

/// Helper function to get the OID of the text lower function
pub fn text_lower_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"pg_catalog.lower(text)".into_datum()],
        )
        .expect("the `pg_catalog.lower(text)` function should exist")
    }
}

unsafe fn extract_score_var(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
        if score_funcoids().contains(&(*funcexpr).funcid) {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            if args.len() == 1 {
                return nodecast!(Var, T_Var, args.get_ptr(0).unwrap());
            }
        }
    }
    None
}

unsafe fn extract_lower_var(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    let funcexpr = nodecast!(FuncExpr, T_FuncExpr, node)?;
    if (*funcexpr).funcid == text_lower_funcoid() {
        let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
        if args.len() == 1 {
            if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                return Some(var);
            } else if let Some(relabel) =
                nodecast!(RelabelType, T_RelabelType, args.get_ptr(0).unwrap())
            {
                return nodecast!(Var, T_Var, (*relabel).arg);
            }
        }
    }
    None
}

/// Extract any `Var` node from an arbitrary expression tree.
///
/// This is needed for `IndexedExpression` handling, where we need a `Var` to check
/// relation membership via `is_varno_valid_for_relation`. Returns the first `Var` found
/// by recursively walking the expression tree.
unsafe fn extract_any_var_from_expr(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    if node.is_null() {
        return None;
    }
    match (*node).type_ {
        pg_sys::NodeTag::T_Var => Some(node as *mut pg_sys::Var),
        pg_sys::NodeTag::T_FuncExpr => {
            let args = PgList::<pg_sys::Node>::from_pg((*(node as *mut pg_sys::FuncExpr)).args);
            let result = args
                .iter_ptr()
                .find_map(|arg| extract_any_var_from_expr(arg));
            result
        }
        pg_sys::NodeTag::T_OpExpr => {
            let args = PgList::<pg_sys::Node>::from_pg((*(node as *mut pg_sys::OpExpr)).args);
            let result = args
                .iter_ptr()
                .find_map(|arg| extract_any_var_from_expr(arg));
            result
        }
        pg_sys::NodeTag::T_RelabelType => {
            extract_any_var_from_expr((*(node as *mut pg_sys::RelabelType)).arg.cast())
        }
        pg_sys::NodeTag::T_CoerceViaIO => {
            extract_any_var_from_expr((*(node as *mut pg_sys::CoerceViaIO)).arg.cast())
        }
        pg_sys::NodeTag::T_CoerceToDomain => {
            extract_any_var_from_expr((*(node as *mut pg_sys::CoerceToDomain)).arg.cast())
        }
        _ => None,
    }
}

pub struct IndexExpressionInfo<'a> {
    pub index_expressions: &'a PgList<pg_sys::Expr>,
    pub schema: &'a SearchIndexSchema,
    pub heap_rti: pg_sys::Index,
}

/// Lazy probe of Tantivy segments used during planning to verify that a JSON sub-path's
/// fast-field leaf type matches the type that Postgres would use to evaluate an ORDER BY
/// expression
///
/// Opening a [`SearchIndexReader`] is not free, so the reader is created on first use and
/// reused across every sort key in the same query. Probes per JSON path are cached so that
/// repeated checks across multiple pathkeys do not re-walk every segment
pub struct JsonSortGate<'a> {
    index_relation: &'a PgSearchRelation,
    reader: RefCell<Option<Option<SearchIndexReader>>>,
    probed: RefCell<std::collections::HashMap<String, Option<tantivy::schema::Type>>>,
}

impl<'a> JsonSortGate<'a> {
    pub fn new(index_relation: &'a PgSearchRelation) -> Self {
        Self {
            index_relation,
            reader: RefCell::new(None),
            probed: RefCell::new(std::collections::HashMap::new()),
        }
    }

    pub fn probe(&self, path: &str) -> Option<tantivy::schema::Type> {
        if let Some(cached) = self.probed.borrow().get(path).copied() {
            return cached;
        }
        let result = {
            let mut slot = self.reader.borrow_mut();
            if slot.is_none() {
                *slot = Some(
                    SearchIndexReader::empty(self.index_relation, MvccSatisfies::Snapshot).ok(),
                );
            }
            slot.as_ref()
                .and_then(|inner| inner.as_ref())
                .and_then(|r| r.probe_json_leaf_type(path))
        };
        self.probed.borrow_mut().insert(path.to_string(), result);
        result
    }
}

/// Map a Postgres type OID to the Tantivy schema [`Type`](tantivy::schema::Type) whose stored
/// order matches Postgres' btree ordering for that type
/// Returns `None` for OIDs that have no safe Tantivy counterpart
/// LIMIT can hide cast or precision differences errors making some mappings unsafe to pushdown
fn pg_type_to_tantivy_type(pg_type: pg_sys::Oid) -> Option<tantivy::schema::Type> {
    use tantivy::schema::Type;
    if pg_type == pg_sys::TEXTOID
        || pg_type == pg_sys::VARCHAROID
        || pg_type == pg_sys::BPCHAROID
        || pg_type == pg_sys::NAMEOID
    {
        Some(Type::Str)
    } else if pg_type == pg_sys::INT8OID {
        Some(Type::I64)
    } else if pg_type == pg_sys::FLOAT8OID {
        Some(Type::F64)
    } else if pg_type == pg_sys::BOOLOID {
        Some(Type::Bool)
    } else if pg_type == pg_sys::TIMESTAMPOID || pg_type == pg_sys::TIMESTAMPTZOID {
        Some(Type::Date)
    } else {
        None
    }
}

/// Determine whether a Raw sort on a JSON field is safe to push down.
///
/// Returns `true` only when:
/// the resolved [`FieldName`] addresses a JSON sub-path (e.g. `metadata.price`)
/// the ORDER BY expression's result type maps to a known Tantivy leaf type
/// the fast-field leaf type stored in Tantivy agrees across all visible segments
/// the probed leaf type matches the expression's expected type
///
/// When any check fails we return `false` and the caller leaves the sort to Postgres, which
/// preserves the user's expected ordering semantics.
pub unsafe fn json_sub_path_sort_pushable(
    gate: &JsonSortGate,
    field_name: &FieldName,
    original_expr: *mut pg_sys::Node,
) -> bool {
    if field_name.path().is_none() {
        return false;
    }
    let expected_oid = pg_sys::exprType(original_expr);
    let Some(expected_type) = pg_type_to_tantivy_type(expected_oid) else {
        return false;
    };
    matches!(gate.probe(field_name.as_ref()), Some(t) if t == expected_type)
}

/// Analyzes an ORDER BY expression to determine its type and extract the underlying variable.
///
/// This function unifies the logic for identifying sort keys across the planner hook (validation)
/// and the custom scan planner (execution).
///
/// When `index_expressions`, `schema`, and `heap_rti` are provided, the function will also
/// attempt to match the expression against indexed expressions via `find_matching_fast_field`
/// as a fallback when the hardcoded patterns (Score, lower, Raw) don't match.
///
/// Returns:
/// - `Some((type, var, field_name))` if the expression is a supported sort key.
/// - `None` if the expression is not supported or the variable/field could not be resolved.
pub unsafe fn analyze_sort_expression(
    node: *mut pg_sys::Node,
    context: VarContext,
    index_info: Option<&IndexExpressionInfo>,
) -> Option<(SortExpressionType, *mut pg_sys::Var, Option<FieldName>)> {
    // Strip order-preserving wrappers (e.g. `id + 0`, RelabelType, CoerceToDomain)
    // so that the expression reaches the pattern-matching below in canonical form.
    let node = strip_identity_wrappers(node);

    if let Some(var) = extract_score_var(node) {
        return Some((SortExpressionType::Score, var, None));
    }

    // If this ORDER BY expression exactly matches an indexed expression, prefer the
    // schema field name from the index itself. This canonicalizes aliased indexed
    // expressions like `lower(description)::pdb.literal('alias=literal_description')`
    // so subsequent sortability checks use `literal_description` rather than the heap
    // column name `description`.
    if let Some(info) = index_info {
        if let Some(fast_field) = find_matching_fast_field(
            node,
            info.index_expressions,
            info.schema.clone(),
            info.heap_rti,
        ) {
            let field_name = FieldName::from(fast_field.name());
            if let Some(var) = extract_any_var_from_expr(node) {
                return Some((SortExpressionType::IndexedExpression, var, Some(field_name)));
            }
        }
    }

    if let Some(var) = extract_lower_var(node) {
        let (relid, attno) = context.var_relation(var);
        let field_name = fieldname_from_var(relid, var, attno);
        return Some((SortExpressionType::Lower, var, field_name));
    }

    if let Some((var, field_name)) = find_one_var_and_fieldname(context, node) {
        return Some((SortExpressionType::Raw, var, Some(field_name)));
    }

    None
}

/// Extract FuncExpr from PlaceHolderVar node
unsafe fn extract_funcexpr_from_placeholder(
    phv: *mut pg_sys::PlaceHolderVar,
) -> Option<*mut pg_sys::FuncExpr> {
    if phv.is_null() || (*phv).phexpr.is_null() {
        return None;
    }

    // The phexpr should contain our FuncExpr
    if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, (*phv).phexpr) {
        return Some(funcexpr);
    }

    None
}

/// Check if a varno is valid for the current relation (rti).
///
/// Returns true if:
/// 1. varno matches rti (exact match)
/// 2. varno is the parent of rti (partitioned/inheritance case)
unsafe fn is_varno_valid_for_relation(
    root: *mut pg_sys::PlannerInfo,
    varno: pg_sys::Index,
    current_rti: pg_sys::Index,
) -> bool {
    // 1. Exact match
    if varno == current_rti {
        return true;
    }

    // 2. Check if varno is parent of current_rti
    if !(*root).append_rel_list.is_null() {
        let append_rels = PgList::<pg_sys::AppendRelInfo>::from_pg((*root).append_rel_list);
        for appinfo in append_rels.iter_ptr() {
            if (*appinfo).parent_relid == varno && (*appinfo).child_relid == current_rti {
                return true;
            }
        }
    }

    false
}

/// Find TargetEntry by ressortgroupref
unsafe fn find_target_entry_by_ref(
    target_list: &PgList<pg_sys::TargetEntry>,
    ref_id: pg_sys::Index,
) -> Option<*mut pg_sys::TargetEntry> {
    target_list
        .iter_ptr()
        .find(|&te| (*te).ressortgroupref == ref_id)
}

/// Normalizes a collation name for case and hyphen-insensitive comparison
/// for example: "C.utf8", "C.UTF-8", "C.UTF8" all normalize to "C.UTF8".
fn normalize_collation_name(mut collation_name: String) -> String {
    collation_name.retain(|c| c != '-');
    collation_name.make_ascii_uppercase();
    collation_name
}

// This helper function tells us whether a collation is "safe", for the purposes of pushing down ORDER BY
// If a field does not have a collation (ex: integers, non-text data), it's considered safe
// Otherwise, for collatable fields, if the collation is C-like it's safe
pub fn is_collation_pushdown_safe(collation: pg_sys::Oid) -> bool {
    const NORMALIZED_SAFE_COLLATION_NAMES: &[&str] = &["C", "POSIX", "C.UTF8", "POSIX.UTF8"];

    let locale = match collation {
        pg_sys::Oid::INVALID | pg_sys::C_COLLATION_OID => return true,
        pg_sys::DEFAULT_COLLATION_OID => lookup_database_collation_locale(),
        _ => lookup_collation_locale(collation),
    };

    // If using the builtin provider, we're always safe, icu is always unsafe, and otherwise we check the name
    match locale {
        #[cfg(any(feature = "pg17", feature = "pg18"))]
        Some(CollationLocale {
            provider: CollationProvider::Builtin,
            ..
        }) => true,
        Some(CollationLocale {
            provider: CollationProvider::Libc,
            name: Some(name),
        }) => NORMALIZED_SAFE_COLLATION_NAMES.contains(&normalize_collation_name(name).as_str()),
        // ICU and anything unrecognized: never byte-ordered
        _ => false,
    }
}

/// Extract pathkeys from ORDER BY clauses using comprehensive expression handling
/// This function handles score functions, lower functions, relabel types, and regular variables
///
/// Returns PathKeyInfo indicating whether any PathKeys existed at all, and if so, whether they
/// might be usable via fast fields.
///
/// This function must be kept in sync with `validate_topk_compatibility` below.
pub unsafe fn extract_pathkey_styles_with_sortability_check<F1, F2>(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    regular_sortability_check: F1,
    lower_sortability_check: F2,
    index_expressions: Option<&PgList<pg_sys::Expr>>,
    json_sort_gate: Option<&JsonSortGate>,
) -> PathKeyInfo
where
    F1: Fn(&SearchField) -> bool,
    F2: Fn(&SearchField) -> bool,
{
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return PathKeyInfo::None;
    }

    let mut pathkey_styles = Vec::new();
    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        let mut found_valid_member = false;

        // If the collation for this pathkey isn't "safe" (C-like), then we can't pushdown as Tantivy uses byte ordering
        let collation = (*equivclass).ec_collation;
        if !is_collation_pushdown_safe(collation) {
            if pathkey_styles.is_empty() {
                return PathKeyInfo::Unusable(UnusableReason::UnsafeCollation);
            } else {
                return PathKeyInfo::UsablePrefix(pathkey_styles);
            }
        }

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            // Handle PlaceHolderVar: unwrap to check if it contains a sortable expression.
            // We support any valid sort expression (Score, Lower, Raw, IndexedExpression)
            // that might be wrapped.
            let mut expr_to_analyze = expr;
            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                if let Some(funcexpr) = extract_funcexpr_from_placeholder(phv) {
                    expr_to_analyze = funcexpr.cast();
                }
            }

            let index_info = index_expressions.map(|idx_exprs| IndexExpressionInfo {
                index_expressions: idx_exprs,
                schema,
                heap_rti: rti,
            });

            if let Some((sort_type, var, field_name_opt)) = analyze_sort_expression(
                expr_to_analyze.cast(),
                VarContext::from_planner(root),
                index_info.as_ref(),
            ) {
                // Verify the var belongs to the correct relation (either the relation itself or its parent)
                if !is_varno_valid_for_relation(root, (*var).varno as pg_sys::Index, rti) {
                    continue;
                }

                match sort_type {
                    SortExpressionType::Score => {
                        pathkey_styles.push(OrderByStyle::Score { pathkey, rti });
                        found_valid_member = true;
                        break;
                    }
                    SortExpressionType::Lower => {
                        if let Some(field_name) = field_name_opt {
                            if let Some(search_field) = schema.search_field(field_name.root()) {
                                if lower_sortability_check(&search_field) {
                                    pathkey_styles.push(OrderByStyle::Field {
                                        pathkey,
                                        name: field_name,
                                        rti,
                                    });
                                    found_valid_member = true;
                                    break;
                                }
                            }
                        }
                    }
                    SortExpressionType::Raw => {
                        if let Some(field_name) = field_name_opt {
                            if let Some(search_field) = schema.search_field(field_name.root()) {
                                if regular_sortability_check(&search_field) {
                                    // For JSON sub-paths, gate pushdown on the probed leaf
                                    // type matching the SQL expression's expected type
                                    if search_field.is_json() {
                                        let pushable = json_sort_gate.is_some_and(|gate| {
                                            json_sub_path_sort_pushable(
                                                gate,
                                                &field_name,
                                                expr_to_analyze.cast(),
                                            )
                                        });
                                        if !pushable {
                                            continue;
                                        }
                                    }
                                    pathkey_styles.push(OrderByStyle::Field {
                                        pathkey,
                                        name: field_name,
                                        rti,
                                    });
                                    found_valid_member = true;
                                    break;
                                }
                            }
                        }
                    }
                    SortExpressionType::IndexedExpression => {
                        if let Some(field_name) = field_name_opt {
                            if let Some(search_field) = schema.search_field(field_name.root()) {
                                if search_field.is_fast() {
                                    pathkey_styles.push(OrderByStyle::Field {
                                        pathkey,
                                        name: field_name,
                                        rti,
                                    });
                                    found_valid_member = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // If we couldn't find any valid member for this pathkey, then we can't handle this series
        // of pathkeys.
        if !found_valid_member {
            if pathkey_styles.is_empty() {
                return PathKeyInfo::Unusable(UnusableReason::NotSortable);
            } else {
                return PathKeyInfo::UsablePrefix(pathkey_styles);
            }
        }
    }

    PathKeyInfo::UsableAll(pathkey_styles)
}

/// Check if the query is a valid Top K query compatible with ParadeDB execution.
///
/// Ensures that:
/// 1. The query has both ORDER BY and LIMIT clauses.
/// 2. There are not too many sort columns.
/// 3. All sort columns belong to the same relation.
/// 4. That relation has a BM25 index.
/// 5. All sort columns are sortable in the index (fast fields).
///
/// This function must be kept in sync with [`extract_pathkey_styles_with_sortability_check`]
/// above to ensure that queries accepted here can be executed by the custom scan.
pub unsafe fn validate_topk_compatibility(parse: *mut pg_sys::Query) -> bool {
    if parse.is_null() || (*parse).sortClause.is_null() || (*parse).limitCount.is_null() {
        return false;
    }

    let sort_list = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
    if sort_list.len() > MAX_TOPK_FEATURES {
        return false;
    }

    let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);

    // -----------------------------------------------------------------
    // Pass 1: identify the single relation that this Top K query targets.
    //
    // The relation is determined by the first sort clause. As in the original
    // single-pass logic, the first clause is analyzed without index-expression
    // info. Resolving the relation up front lets `bm25_index`, `schema`, and
    // `index_expressions` be immutable for the rest of the function, so the
    // JsonSortGate can borrow `bm25_index` directly without any raw-pointer
    // lifetime laundering.
    // -----------------------------------------------------------------
    let Some(first_clause) = sort_list.get_ptr(0) else {
        return false;
    };
    let Some(first_te) = find_target_entry_by_ref(&target_list, (*first_clause).tleSortGroupRef)
    else {
        return false;
    };
    let first_expr = (*first_te).expr as *mut pg_sys::Node;

    let Some((_, first_var, _)) =
        analyze_sort_expression(first_expr, VarContext::from_query(parse), None)
    else {
        return false;
    };

    let target_varno = (*first_var).varno as pg_sys::Index;
    if target_varno == 0 {
        return false;
    }

    let (relid, _) = VarContext::from_query(parse).var_relation(first_var);
    if relid == pg_sys::InvalidOid {
        return false;
    }

    // Check if has BM25
    let (_, bm25_index) = match rel_get_bm25_index(relid) {
        Some(res) => res,
        None => return false,
    };

    let schema = match SearchIndexSchema::open(&bm25_index) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let index_expressions = bm25_index.index_expressions();

    // Index info shared across sort clauses. Built once; borrows the immutable
    // relation state resolved above.
    // TODO: heap_rti should use the actual varno from the target relation rather
    // than hardcoded 1. Only matters for the #3455 window function path.
    let index_info = IndexExpressionInfo {
        index_expressions: &index_expressions,
        schema: &schema,
        heap_rti: 1 as pg_sys::Index,
    };

    let mut json_sort_gate: Option<JsonSortGate> = None;

    // -----------------------------------------------------------------
    // Pass 2: validate every sort clause against the identified relation.
    // -----------------------------------------------------------------
    for (i, sort_clause) in sort_list.iter_ptr().enumerate() {
        let tle_ref = (*sort_clause).tleSortGroupRef;
        let Some(te) = find_target_entry_by_ref(&target_list, tle_ref) else {
            return false;
        };

        let expr = (*te).expr as *mut pg_sys::Node;

        // Preserve original behavior: the first clause is analyzed without index info.
        let clause_index_info = if i == 0 { None } else { Some(&index_info) };
        // If the collation for this pathkey isn't "safe" (C-like), then we can't pushdown as Tantivy uses byte ordering
        let expr_collation = pg_sys::exprCollation(expr as *const pg_sys::Node);
        if !is_collation_pushdown_safe(expr_collation) {
            return false;
        }

        // Pass index expressions if we have them (after first sort clause identifies the relation)
        let index_info = target_relation_info
            .as_ref()
            .map(|(_, _, schema, idx_exprs)| IndexExpressionInfo {
                index_expressions: idx_exprs,
                schema,
                // TODO: heap_rti should use the actual varno from target_relation_info
                // rather than hardcoded 1. Only matters for the #3455 window function path.
                heap_rti: 1 as pg_sys::Index,
            });

        // Use analyze_sort_expression to identify the sort key type and underlying variable
        let Some((sort_type, var, field_name_opt)) =
            analyze_sort_expression(expr, VarContext::from_query(parse), clause_index_info)
        else {
            return false;
        };

        // All sort columns must belong to the same relation
        let varno = (*var).varno as pg_sys::Index;
        if varno == 0 || varno != target_varno {
            return false;
        }

        match sort_type {
            SortExpressionType::Score => {
                // Score is always sortable
                continue;
            }
            SortExpressionType::Lower => {
                let Some(field_name) = field_name_opt else {
                    return false;
                };
                let Some(search_field) = schema.search_field(field_name.root()) else {
                    return false;
                };
                if !search_field.is_lower_sortable() {
                    return false;
                }
            }
            SortExpressionType::Raw => {
                let Some(field_name) = field_name_opt else {
                    return false;
                };
                let Some(search_field) = schema.search_field(field_name.root()) else {
                    return false;
                };
                if !search_field.is_raw_sortable() {
                    return false;
                }
                if search_field.is_json() {
                    // Lazily build the gate the first time we hit a JSON sub-path sort.
                    // `bm25_index` is immutable for the rest of the function, so the gate
                    // can borrow it directly.
                    let gate = json_sort_gate.get_or_insert_with(|| JsonSortGate::new(&bm25_index));
                    if !json_sub_path_sort_pushable(gate, &field_name, expr) {
                        return false;
                    }
                }
            }
            SortExpressionType::IndexedExpression => {
                let Some(field_name) = field_name_opt else {
                    return false;
                };
                let Some(search_field) = schema.search_field(field_name.root()) else {
                    return false;
                };
                if !search_field.is_fast() {
                    return false;
                }
            }
        }
    }

    true
}

/// Find a pathkey from the query that matches the index's sort_by field.
///
/// This is used to expose sorted CustomPaths when the query has an ORDER BY
/// that matches the index's sort order. The Postgres planner can then choose
/// the sorted path when downstream operations (like merge joins) need sorted input.
///
/// Returns `Some(OrderByStyle)` if a matching pathkey is found, `None` otherwise.
pub unsafe fn find_sort_by_pathkey(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    sort_by: &SortByField,
    table: &PgSearchRelation,
) -> Option<OrderByStyle> {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return None;
    }

    let sort_field_name = sort_by.field_name.as_ref();

    // Only the first pathkey can be used for sorted execution (prefix semantics).
    // If the first pathkey doesn't match sort_by, we must not declare ordering.
    let pathkey = pathkeys.get_ptr(0)?;
    let equivclass = (*pathkey).pk_eclass;
    let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

    for member in members.iter_ptr() {
        let expr = (*member).em_expr;

        // Try to extract a Var from the expression (either directly or wrapped in RelabelType)
        let var = if let Some(var) = nodecast!(Var, T_Var, expr) {
            var
        } else if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
            // RelabelType wraps a Var for type coercions
            match nodecast!(Var, T_Var, (*relabel).arg) {
                Some(var) => var,
                None => continue,
            }
        } else {
            continue;
        };

        // Check if this Var matches our sort_by field
        if let Some(style) =
            check_var_matches_sort_by(var, rti, pathkey, sort_by, sort_field_name, table)
        {
            return Some(style);
        }
    }

    None
}

/// Check if a Var matches the sort_by field and return an OrderByStyle if it does.
///
/// This helper extracts the common logic for checking whether a column reference (Var)
/// matches the index's sort_by configuration, including direction and NULLS ordering.
unsafe fn check_var_matches_sort_by(
    var: *mut pg_sys::Var,
    rti: pg_sys::Index,
    pathkey: *mut pg_sys::PathKey,
    sort_by: &SortByField,
    sort_field_name: &str,
    table: &PgSearchRelation,
) -> Option<OrderByStyle> {
    // Check if this Var is for our relation
    if (*var).varno as pg_sys::Index != rti {
        return None;
    }

    // Get the column name from the tuple descriptor
    let tupdesc = table.tuple_desc();
    let attno = (*var).varattno;
    if attno <= 0 || attno as usize > tupdesc.len() {
        return None;
    }

    let att = tupdesc.get(attno as usize - 1)?;
    let col_name = att.name();
    if col_name != sort_field_name {
        return None;
    }

    if !pathkey_matches_sort_by(pathkey, sort_by) {
        return None;
    }

    Some(OrderByStyle::Field {
        pathkey,
        name: sort_field_name.into(),
        rti,
    })
}

pub unsafe fn pathkey_matches_sort_by(
    pathkey: *mut pg_sys::PathKey,
    sort_by: &SortByField,
) -> bool {
    // Check if the sort direction matches
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    let is_desc = match (*pathkey).pk_strategy as u32 {
        pg_sys::BTLessStrategyNumber => false,
        pg_sys::BTGreaterStrategyNumber => true,
        _ => return false, // Unknown strategy
    };
    #[cfg(feature = "pg18")]
    let is_desc = match (*pathkey).pk_cmptype {
        pg_sys::CompareType::COMPARE_LT => false,
        pg_sys::CompareType::COMPARE_GT => true,
        _ => return false,
    };

    let sort_by_is_desc = matches!(sort_by.direction, SortByDirection::Desc);
    // Tantivy's sort behavior is fixed:
    //   - ASC: nulls sort first (smallest values)
    //   - DESC: nulls sort last (after largest values)
    // We cannot support other NULLS orderings (e.g., ASC NULLS LAST) because
    // Tantivy physically sorts documents this way at index time.
    let sort_by_nulls_first = !sort_by_is_desc;

    // Direction and NULLS ordering must both match for sorted path to apply.
    // If query requests incompatible NULLS ordering, we return None and
    // PostgreSQL will add a Sort node to achieve the requested ordering.

    // Note: Tantivy uses byte ordering (like C locale) - for fields with non-C collation, an incorrect order may be produced, so we must
    // check for safety using `is_collation_pushdown_safe`
    let collation = (*(*pathkey).pk_eclass).ec_collation;

    is_desc == sort_by_is_desc
        && (*pathkey).pk_nulls_first == sort_by_nulls_first
        && is_collation_pushdown_safe(collation)
}
