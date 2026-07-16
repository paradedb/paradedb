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
use crate::index::reader::index::MAX_TOPK_FEATURES;
use crate::nodecast;
use crate::postgres::catalog::{
    lookup_collation_locale, lookup_database_collation_locale, CollationLocale, CollationProvider,
};
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::{
    fieldname_from_var, find_one_var_and_fieldname, strip_identity_wrappers, VarContext,
};
use crate::schema::{SearchField, SearchFieldType, SearchIndexSchema};
use crate::vector::metric::VectorMetric;
use pgrx::{direct_function_call, pg_sys, FromDatum, IntoDatum, PgList};

/// The type of sort expression found in an ORDER BY clause.
#[derive(Debug, Clone, PartialEq)]
pub enum SortExpressionType {
    /// Sorting by search score: `ORDER BY pdb.score(...)`
    Score,
    /// Sorting by a lowercased field: `ORDER BY lower(col)`
    Lower,
    /// Sorting by a raw field: `ORDER BY col`
    Raw,
    /// Sorting by an expression that matched an indexed expression in `pg_index.indexprs`.
    IndexedExpression,
    /// Sorting by vector distance: `ORDER BY col <-> '[...]'`.
    /// Carries the resolved query vector (if a `Const`), or the
    /// `Param` ID to resolve at execution time (if a parameterized
    /// generic plan), plus the metric implied by the operator
    /// (`<->` → L2, `<=>` → Cosine, `<#>` → InnerProduct).
    /// Query vectors are passed through to tantivy raw — the storage
    /// layer owns unit-norm policy for the doc side, and the cosine
    /// scoring kernel handles non-unit queries via `inv_norm_q`.
    VectorDistance {
        query_vector: Vec<f32>,
        query_vector_param_id: Option<i32>,
        metric: VectorMetric,
    },
    /// Sorting by a vector distance operator whose implied metric
    /// disagrees with the index attribute's opclass — e.g. `<=>`
    /// (cosine) on a column built with `vector_l2_ops`. We refuse to
    /// silently rank by the index's metric instead of the operator
    /// the user asked for; the planner falls back to a non-index
    /// sort. Only carried for diagnostics in the planner warning.
    VectorMetricMismatch {
        field_metric: VectorMetric,
        op_metric: VectorMetric,
    },
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
    /// ORDER BY uses a vector distance operator whose implied metric
    /// disagrees with the index attribute's opclass (e.g. `<=>` on a
    /// column built with `vector_l2_ops`).
    VectorMetricMismatch {
        field_metric: VectorMetric,
        op_metric: VectorMetric,
    },
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

    if let Some(info) = index_info {
        if let Some((var, vector_sort)) = extract_vector_distance(node, context, info.schema) {
            let (relid, attno) = context.var_relation(var);
            let field_name = fieldname_from_var(relid, var, attno);
            return Some((vector_sort, var, field_name));
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

/// Map a pgvector distance operator OID to its implied `VectorMetric`.
/// Returns `None` if `opoid` is not one of `<->`, `<=>`, or `<#>`.
pub(crate) fn metric_for_opoid(opoid: pg_sys::Oid) -> Option<VectorMetric> {
    use std::sync::OnceLock;
    static OP_METRICS: OnceLock<[(pg_sys::Oid, VectorMetric); 3]> = OnceLock::new();
    let cached = OP_METRICS.get_or_init(|| unsafe {
        let lookup = |sig: &std::ffi::CStr| -> pg_sys::Oid {
            direct_function_call::<pg_sys::Oid>(pg_sys::regoperatorin, &[sig.into_datum()])
                .unwrap_or(pg_sys::Oid::INVALID)
        };
        [
            (lookup(c"<->(vector,vector)"), VectorMetric::L2),
            (lookup(c"<=>(vector,vector)"), VectorMetric::Cosine),
            (lookup(c"<#>(vector,vector)"), VectorMetric::InnerProduct),
        ]
    });
    cached
        .iter()
        .find(|(oid, _)| *oid != pg_sys::Oid::INVALID && *oid == opoid)
        .map(|(_, metric)| *metric)
}

/// Try to interpret `node` as a reference to an indexed vector
/// column. Accepts only a direct `T_Var` whose schema entry resolves
/// to `SearchFieldType::Vector` — pgvector convention. The metric
/// travels via the index attribute's opclass (see
/// `VectorMetric::from_index_attr`); the column type itself
/// is just plain `vector`, so there is no cast wrapper to unwrap.
unsafe fn resolve_vector_expr(
    node: *mut pg_sys::Node,
    context: VarContext,
    schema: &SearchIndexSchema,
) -> Option<(*mut pg_sys::Var, VectorMetric)> {
    if node.is_null() || (*node).type_ != pg_sys::NodeTag::T_Var {
        return None;
    }
    let var = node as *mut pg_sys::Var;
    let (relid, attno) = context.var_relation(var);
    let field_name = fieldname_from_var(relid, var, attno)?;
    let field_type = schema.get_field_type(field_name.root())?;
    if let SearchFieldType::Vector(_, _, metric) = field_type {
        return Some((var, metric));
    }
    None
}

/// Detect `col <-> '[...]'` vector distance expression.
/// Returns (column Var, sort expression descriptor) on match.
///
/// The operator's implied metric (`<->` → L2, `<=>` → Cosine,
/// `<#>` → InnerProduct) must equal the indexed field's metric;
/// a mismatch (e.g. `<=>` against an L2-metric field) bails so
/// Postgres can pick a non-index plan instead of silently returning
/// results ranked by the wrong distance.
unsafe fn extract_vector_distance(
    node: *mut pg_sys::Node,
    context: VarContext,
    schema: &SearchIndexSchema,
) -> Option<(*mut pg_sys::Var, SortExpressionType)> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let op_metric = metric_for_opoid((*opexpr).opno)?;

    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    if args.len() != 2 {
        return None;
    }

    let left = args.get_ptr(0)?;
    let right = args.get_ptr(1)?;

    // One side must be a direct Var on the indexed vector column; the
    // other carries the query vector. The metric travels via the index
    // attribute's opclass (pgvector convention), not via a cast on the
    // column, so there's no wrapper expression to unwrap here.
    let (var_node, field_metric, value_node) =
        if let Some((var, metric)) = resolve_vector_expr(left, context, schema) {
            (var, metric, right)
        } else if let Some((var, metric)) = resolve_vector_expr(right, context, schema) {
            (var, metric, left)
        } else {
            return None;
        };

    if field_metric != op_metric {
        return Some((
            var_node,
            SortExpressionType::VectorMetricMismatch {
                field_metric,
                op_metric,
            },
        ));
    }

    if let Some(const_node) = nodecast!(Const, T_Const, value_node) {
        if (*const_node).constisnull {
            return None;
        }
        let datum = (*const_node).constvalue;
        let query_vector = unsafe { crate::vector::PgVector::from_datum(datum, false) }
            .expect("vector ORDER BY constant should not be NULL")
            .0;
        return Some((
            var_node,
            SortExpressionType::VectorDistance {
                query_vector,
                query_vector_param_id: None,
                metric: op_metric,
            },
        ));
    }

    if let Some(param) = nodecast!(Param, T_Param, value_node) {
        // Only PARAM_EXTERN values (prepared-statement bindings) live in
        // EState.es_param_list_info. PARAM_EXEC (subplan params) require
        // a different resolution path that we don't yet support — leave
        // those queries to fall back to a regular sort.
        if (*param).paramkind != pg_sys::ParamKind::PARAM_EXTERN {
            return None;
        }
        return Some((
            var_node,
            SortExpressionType::VectorDistance {
                query_vector: Vec::new(),
                query_vector_param_id: Some((*param).paramid),
                metric: op_metric,
            },
        ));
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
pub(crate) unsafe fn is_varno_valid_for_relation(
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

/// Cheap (no index open) detection of whether the query's `ORDER BY` includes a
/// vector-distance operator (`<->`/`<=>`/`<#>`) applied to a `Var` on relation
/// `rti`. Used to decide, before opening the bm25 index, whether a bare
/// `ORDER BY <vector>` with no `@@@` predicate should still justify a custom
/// scan. Conservative: a false negative just falls back to a regular sort, and
/// the authoritative opclass/indexed-field validation still happens in
/// `pullup_topk_pathkeys` after the index is opened.
pub(crate) unsafe fn query_orders_by_vector_distance(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
) -> bool {
    if root.is_null() || (*root).query_pathkeys.is_null() {
        return false;
    }
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    for pathkey in pathkeys.iter_ptr() {
        let equivclass = (*pathkey).pk_eclass;
        if equivclass.is_null() {
            continue;
        }
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);
        for member in members.iter_ptr() {
            let mut expr = (*member).em_expr.cast::<pg_sys::Node>();
            // Vector-distance sorts arrive as a bare OpExpr; unwrap a PlaceHolderVar
            // wrapper (as `extract_pathkey_styles` does) to reach it.
            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                expr = (*phv).phexpr.cast();
            }
            let Some(opexpr) = nodecast!(OpExpr, T_OpExpr, expr) else {
                continue;
            };
            if metric_for_opoid((*opexpr).opno).is_none() {
                continue;
            }
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
            for arg in args.iter_ptr() {
                if let Some(var) = nodecast!(Var, T_Var, arg) {
                    if is_varno_valid_for_relation(root, (*var).varno as pg_sys::Index, rti) {
                        return true;
                    }
                }
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
                    SortExpressionType::VectorDistance {
                        ref query_vector,
                        query_vector_param_id,
                        metric,
                    } => {
                        if let Some(field_name) = field_name_opt {
                            pathkey_styles.push(OrderByStyle::VectorDistance {
                                pathkey,
                                name: field_name,
                                rti,
                                query_vector: query_vector.clone(),
                                query_vector_param_id,
                                metric,
                            });
                            found_valid_member = true;
                            break;
                        }
                    }
                    SortExpressionType::VectorMetricMismatch {
                        field_metric,
                        op_metric,
                    } => {
                        // Don't keep iterating other equivalence-class
                        // members — the user clearly intended this
                        // metric and we want the warning to surface
                        // that specific reason.
                        return PathKeyInfo::Unusable(UnusableReason::VectorMetricMismatch {
                            field_metric,
                            op_metric,
                        });
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

    // We need to identify the single relation that this Top K query targets
    // Tuple: (varno, relid, schema, index_expressions)
    let mut target_relation_info: Option<(
        pg_sys::Index,
        pg_sys::Oid,
        SearchIndexSchema,
        PgList<pg_sys::Expr>,
    )> = None;

    for sort_clause in sort_list.iter_ptr() {
        let tle_ref = (*sort_clause).tleSortGroupRef;
        let Some(te) = find_target_entry_by_ref(&target_list, tle_ref) else {
            return false;
        };

        let expr = (*te).expr as *mut pg_sys::Node;

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
            analyze_sort_expression(expr, VarContext::from_query(parse), index_info.as_ref())
        else {
            return false;
        };

        // Identify relation
        let varno = (*var).varno as pg_sys::Index;
        if varno == 0 {
            return false;
        }

        if let Some((expected_varno, _, _, _)) = &target_relation_info {
            if varno != *expected_varno {
                // Sorting by different relations
                return false;
            }
        } else {
            // Initialize target relation info
            let (relid, _) = VarContext::from_query(parse).var_relation(var);
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

            target_relation_info = Some((varno, relid, schema, index_expressions));
        }

        // Validate sortability
        let (_, _, schema, _) = target_relation_info.as_ref().unwrap();

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
            SortExpressionType::VectorDistance { .. } => {
                // Vector distance is always valid if the field exists in the index
                continue;
            }
            SortExpressionType::VectorMetricMismatch { .. } => {
                // Operator/opclass disagreement; not pushable.
                return false;
            }
        }
    }

    target_relation_info.is_some()
}
