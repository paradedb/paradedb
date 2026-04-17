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

//! Planning-related functions for JoinScan.
//!
//! This module contains functions used during the planning phase to:
//! - Extract and analyze join conditions
//! - Gather information about join sides (tables, indexes, predicates)
//! - Collect required fields to ensure availability during execution
//! - Handle ORDER BY score pathkeys

use super::build::{
    self as build, FilterNode, InputVarInfo, JoinCSClause, JoinKeyPair, JoinLevelExpr, JoinNode,
    JoinSource, JoinSourceCandidate, JoinType, RelNode,
};
use super::predicate::find_base_info_recursive;
use super::privdat::{OutputColumnInfo, PrivateData};

use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{NullTestKind, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::score::is_score_func;
use crate::postgres::customscan::opexpr::lookup_operator;
use crate::postgres::customscan::pullup::{
    field_type_for_pullup, get_attno_by_name, resolve_fast_field,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::bms_iter;
use crate::postgres::customscan::score_funcoids;
use crate::postgres::customscan::CustomScan;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_vars, expr_contains_any_operator};
use crate::postgres::var::{fieldname_from_var, strip_identity_wrappers};
use crate::query::SearchQueryInput;

use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, PgList};

const PVC_RECURSE_ALL: i32 = (pg_sys::PVC_RECURSE_AGGREGATES
    | pg_sys::PVC_RECURSE_WINDOWFUNCS
    | pg_sys::PVC_RECURSE_PLACEHOLDERS) as i32;

/// Check if an expression uses paradedb.score() for any relation in the JoinSource.
pub(super) unsafe fn expr_uses_scores_from_source(
    node: *mut pg_sys::Node,
    source: &JoinSource,
) -> bool {
    // We use a walker to find score functions
    use pgrx::pg_sys::expression_tree_walker;
    use std::ptr::addr_of_mut;

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node) {
            let data = context.cast::<Data>();
            if (*data).funcoids.contains(&(*funcexpr).funcid) {
                let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
                if args.len() == 1 {
                    if let Some(var) = nodecast!(Var, T_Var, args.get_ptr(0).unwrap()) {
                        let varno = (*var).varno as pg_sys::Index;
                        if (*data).source.contains_rti(varno) {
                            (*data).found = true;
                            return true; // Abort traversal, found it
                        }
                    }
                }
            }
        }

        expression_tree_walker(node, Some(walker), context)
    }

    struct Data<'a> {
        source: &'a JoinSource,
        funcoids: [pg_sys::Oid; 2],
        found: bool,
    }

    let mut data = Data {
        source,
        funcoids: score_funcoids(),
        found: false,
    };

    walker(node, addr_of_mut!(data).cast());
    data.found
}

pub(super) struct JoinConditions {
    /// Equi-join keys with type info for composite key extraction.
    pub equi_keys: Vec<JoinKeyPair>,
    /// Other join conditions (non-equijoin) that need to be evaluated after join.
    /// These are the RestrictInfo nodes themselves.
    pub other_conditions: Vec<*mut pg_sys::RestrictInfo>,
    /// Whether any join-level condition contains our @@@ operator.
    pub has_search_predicate: bool,
}

/// Extract join conditions from the restrict list.
///
/// Analyzes the join's restrict list to identify:
/// - Equi-join conditions (e.g., `a.id = b.id`) for joining
/// - Other conditions that need post-join evaluation
/// - Whether any condition contains our @@@ search operator
pub(super) unsafe fn extract_join_conditions(
    extra: *mut pg_sys::JoinPathExtraData,
    sources: &[&JoinSource],
) -> JoinConditions {
    let result = JoinConditions {
        equi_keys: Vec::new(),
        other_conditions: Vec::new(),
        has_search_predicate: false,
    };

    if extra.is_null() || sources.len() < 2 {
        return result;
    }

    let restrictlist = (*extra).restrictlist;
    if restrictlist.is_null() {
        return result;
    }

    extract_join_conditions_from_list(restrictlist, sources)
}

/// Get type length and pass-by-value info for a given type OID.
unsafe fn get_type_info(type_oid: pg_sys::Oid) -> (i16, bool) {
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(type_oid, &mut typlen, &mut typbyval);
    (typlen, typbyval)
}

/// Main entry point for constructing a DataFusion relational query tree (`RelNode`) from
/// a PostgreSQL planner `RelOptInfo` structure.
///
/// This recursive function explores the initial query topology during `plan_custom_path` to verify
/// whether the join tree is viable for DataFusion `JoinScan` execution. It does this by:
/// 1. Locating BM25-backed relations and determining if a `@@@` full-text search predicate is present.
/// 2. Iterating through `baserestrictinfo` to natively support correlated `T_SubPlan` subqueries
///    (e.g., `IN` / `NOT IN`) by mapping them into relational `Semi` or `Anti` joins rather than
///    scalar evaluations.
/// 3. Reconstructing physical join paths (`JoinPath`) by gathering the source base relations and
///    equi-join conditions.
///
/// Returns an intermediate `RelNode` tree capturing the execution plan structure, as well as a list
/// of all extracted equi-join keys.
pub(super) unsafe fn collect_join_sources(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> Option<(RelNode, Vec<JoinKeyPair>)> {
    if rel.is_null() {
        return None;
    }

    let relids = (*rel).relids;
    if relids.is_null() {
        return None;
    }

    let num_relids = pg_sys::bms_num_members(relids);

    if num_relids == 1 {
        let mut rti_iter = bms_iter(relids);
        let rti = rti_iter.next()?;
        return collect_join_sources_base_rel(root, rel, rti);
    }

    collect_join_sources_join_rel(root, rel)
}

/// Handles the extraction of search predicates and nested subqueries from a single base relation.
/// Constructs the initial `RelNode::Scan` and wraps it in `Semi`/`Anti` joins if subqueries are present.
///
/// TODO: Currently, we only extract `T_SubPlan`s if they are at the top level of the
/// `baserestrictinfo` list (i.e. not nested inside AND/OR trees). This is sufficient for many
/// typical query patterns, but could be extended to dig deeper into the boolean expression tree.
pub(super) unsafe fn collect_join_sources_base_rel(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    rti: pg_sys::Index,
) -> Option<(RelNode, Vec<JoinKeyPair>)> {
    let (relid, alias, _bm25_opt) = build::lookup_base_rel_info(root, rti)?;

    let mut side_info = JoinSourceCandidate::new(root.into(), rti).with_heaprelid(relid);
    if let Some(alias) = alias {
        side_info = side_info.with_alias(alias);
    }

    // Subquery extraction is meaningful only when the base side has a BM25 index;
    // otherwise the Semi/Anti/LeftMark wrapping has nothing useful to wrap.
    let mut classified = ClassifiedBaseRestrictInfo::empty();

    if let Some((_, bm25_index)) = rel_get_bm25_index(relid) {
        side_info = side_info.with_indexrelid(bm25_index.oid());

        // Read the sort order from the index's relation options.
        // This allows DataFusion-based execution to leverage physical sort order
        // for optimizations like SortPreservingMergeExec and sort-merge joins.
        let sort_order = if crate::gucs::is_columnar_sort_enabled() {
            let sort_by = bm25_index.options().sort_by();
            sort_by.into_iter().next()
        } else {
            None
        };
        side_info = side_info.with_sort_order(sort_order);

        classified = classify_base_restrictinfo(root, (*rel).baserestrictinfo);

        if !classified.search_ri.is_empty() {
            let context = PlannerContext::from_planner(root);
            let mut state = QualExtractState::default();
            // Extract search-capable predicates all at once. This is required
            // for score filters, which must wrap the rest of the search query.
            if let Some(qual) = extract_quals(
                &context,
                rti,
                classified.search_ri.as_ptr().cast(),
                crate::postgres::customscan::builders::custom_path::RestrictInfoType::BaseRelation,
                &bm25_index,
                false,
                &mut state,
                true,
            ) {
                let query = SearchQueryInput::from(&qual);
                side_info = side_info.with_query(query);
                if state.uses_our_operator {
                    side_info = side_info.with_search_predicate();
                }
            } else {
                // Fail the JoinScan if any search predicate cannot be extracted.
                return None;
            }
        }
    }

    side_info.estimate_rows();
    let source = JoinSource::try_from(side_info).ok()?;

    let mut current_node = RelNode::Scan(Box::new(source));
    let mut all_keys = Vec::new();

    current_node = wrap_with_semi_anti(current_node, classified.top_level_subplans, &mut all_keys);
    current_node = wrap_with_mark_filter(current_node, classified.or_subplans, &mut all_keys);

    Some((current_node, all_keys))
}

/// Buckets that [`classify_base_restrictinfo`] sorts a relation's
/// `baserestrictinfo` clauses into.
struct ClassifiedBaseRestrictInfo {
    /// Clauses that are not subplans and should be batched into `extract_quals`
    /// for search predicate extraction.
    search_ri: PgList<pg_sys::RestrictInfo>,
    /// Top-level SubPlans (e.g. `col IN (SELECT ...)`) that should become
    /// Semi/Anti joins wrapping the base scan.
    top_level_subplans: Vec<(*mut pg_sys::SubPlan, bool, *mut pg_sys::PlannerInfo)>,
    /// SubPlans nested inside an OR (e.g. `col IS NULL OR col IN (SELECT ...)`)
    /// that should become LeftMark joins followed by a Filter.
    or_subplans: Vec<OrSubPlanExtraction>,
}

impl ClassifiedBaseRestrictInfo {
    fn empty() -> Self {
        Self {
            search_ri: PgList::<pg_sys::RestrictInfo>::new(),
            top_level_subplans: Vec::new(),
            or_subplans: Vec::new(),
        }
    }
}

/// Walk a relation's `baserestrictinfo` and split each clause into one of three
/// buckets: search-extractable predicates, top-level SubPlans, or OR-nested
/// SubPlans. Subplans are pulled out so the remaining `search_ri` can be passed
/// to `extract_quals` as a fully-search-capable batch.
unsafe fn classify_base_restrictinfo(
    root: *mut pg_sys::PlannerInfo,
    baserestrictinfo: *mut pg_sys::List,
) -> ClassifiedBaseRestrictInfo {
    let mut classified = ClassifiedBaseRestrictInfo::empty();

    // Extract single-table predicates from baserestrictinfo.
    // These are predicates like `p.description @@@ 'wireless'` that PostgreSQL
    // has pushed down to the base relation level.
    //
    // Note: Cross-table predicates (e.g., involving multiple tables in a join)
    // are handled separately via SearchPredicateUDF through filter pushdown.
    let baserestrictinfo = PgList::<pg_sys::RestrictInfo>::from_pg(baserestrictinfo);
    if baserestrictinfo.is_empty() {
        return classified;
    }

    // Separate subplans (SEMI/ANTI joins) from search-capable predicates.
    // Subplans are collected and later handled by wrapping the current
    // scan's RelNode in additional Join nodes. This separation ensures
    // `extract_quals` only receives clauses it can fully convert to a
    // Tantivy query.
    for ri in baserestrictinfo.iter_ptr() {
        if let Some((subplan, is_anti, inner_root)) =
            extract_subplan_from_clause(root, (*ri).clause.cast())
        {
            // Top-level SubPlan (e.g. `col IN (SELECT ...)`) → Semi/Anti join.
            classified
                .top_level_subplans
                .push((subplan, is_anti, inner_root));
        } else {
            // Try to extract SubPlan from inside an OR expression.
            // Handles patterns like `col IS NULL OR col IN (SELECT ...)`.
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause.cast()
            } else {
                (*ri).clause.cast()
            };
            if let Some(or_extraction) = extract_subplan_from_or_clause(root, clause) {
                classified.or_subplans.push(or_extraction);
            } else {
                // Not a SubPlan — pass to extract_quals for search predicate extraction.
                classified.search_ri.push(ri);
            }
        }
    }

    classified
}

/// Wrap a base scan node with Semi/Anti joins, one per top-level extracted
/// SubPlan. Equi-keys produced by each SubPlan are appended to `all_keys` so
/// the caller can return the full set alongside the wrapped node.
unsafe fn wrap_with_semi_anti(
    mut current_node: RelNode,
    top_level_subplans: Vec<(*mut pg_sys::SubPlan, bool, *mut pg_sys::PlannerInfo)>,
    all_keys: &mut Vec<JoinKeyPair>,
) -> RelNode {
    for (subplan, is_anti, inner_root) in top_level_subplans {
        // Find the final rel for the inner subquery
        let inner_rel = find_final_rel(inner_root);
        if inner_rel.is_null() {
            continue; // Can't resolve inner relation, maybe log or skip
        }

        let Some((inner_node, inner_keys)) = collect_join_sources(inner_root, inner_rel) else {
            continue;
        };

        // Recursively collect join sources for the inner subquery
        all_keys.extend(inner_keys);

        let equi_keys =
            extract_equi_keys_from_subplan(subplan, inner_root, &current_node, &inner_node);

        let plan_id = (*subplan).plan_id;
        let join_node = JoinNode {
            join_type: if is_anti {
                JoinType::Anti
            } else {
                JoinType::Semi
            },
            left: current_node,
            right: inner_node,
            equi_keys: equi_keys.clone(),
            filter: None,
            subplan_id: Some(plan_id),
        };

        all_keys.extend(equi_keys);
        current_node = RelNode::Join(Box::new(join_node));
    }

    current_node
}

/// Wrap a base scan node with a `LeftMark` join + Filter for each OR-extracted
/// SubPlan (`col IS NULL OR col IN (SELECT ...)` style). The LeftMark join
/// produces all left rows plus a boolean "mark" column; the Filter then keeps
/// rows where `mark = true OR col IS NULL` (or the inverted form for NOT IN).
unsafe fn wrap_with_mark_filter(
    mut current_node: RelNode,
    or_subplans: Vec<OrSubPlanExtraction>,
    all_keys: &mut Vec<JoinKeyPair>,
) -> RelNode {
    for or_ext in or_subplans {
        let inner_rel = find_final_rel(or_ext.inner_root);
        if inner_rel.is_null() {
            continue;
        }

        let Some((inner_node, inner_keys)) = collect_join_sources(or_ext.inner_root, inner_rel)
        else {
            continue;
        };

        all_keys.extend(inner_keys);

        let equi_keys = extract_equi_keys_from_subplan(
            or_ext.subplan,
            or_ext.inner_root,
            &current_node,
            &inner_node,
        );

        // Build a LeftMark join: produces all left rows + boolean "mark" column.
        // Both `IN (...) OR IS NULL` and `NOT IN (...) OR IS NULL` use LeftMark;
        // the anti vs non-anti distinction is carried through `or_ext.is_anti`
        // and applied at filter-evaluation time as a mark-check inversion.
        let plan_id = (*or_ext.subplan).plan_id;
        let join_node = JoinNode {
            join_type: JoinType::LeftMark,
            left: current_node,
            right: inner_node,
            equi_keys: equi_keys.clone(),
            filter: None,
            subplan_id: Some(plan_id),
        };

        all_keys.extend(equi_keys);
        let join_rel = RelNode::Join(Box::new(join_node));

        // Wrap the LeftMark join in a Filter node:
        //   `mark = true OR outer_col IS NULL`  (for IN)
        //   `mark = false OR outer_col IS NULL`  (for NOT IN)
        //
        // The filter is stored as a MarkOrNullFilter which is handled specially
        // during DataFusion plan building (see scan_state.rs).
        let filter_node = FilterNode {
            input: join_rel,
            predicate: JoinLevelExpr::MarkOrNull {
                is_anti: or_ext.is_anti,
                null_test_varno: or_ext.null_test_varno,
                null_test_attno: or_ext.null_test_attno,
            },
        };

        current_node = RelNode::Filter(Box::new(filter_node));
    }

    current_node
}

/// Recursively reconstructs the intermediate relational tree from standard PostgreSQL join paths.
/// Supports extracting inner equi-joins between base relations and returns the accumulated plan and join keys.
unsafe fn collect_join_sources_join_rel(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> Option<(RelNode, Vec<JoinKeyPair>)> {
    // We only inspect the cheapest path chosen by PostgreSQL.
    let path = (*rel).cheapest_total_path;
    if path.is_null() {
        return None;
    }

    if (*path).type_ == pg_sys::NodeTag::T_CustomPath {
        let custom_path = path as *mut pg_sys::CustomPath;
        let methods = (*custom_path).methods;

        // Check if this is a JoinScan
        let name_ptr = (*methods).CustomName;
        if !name_ptr.is_null() {
            let name_cstr = std::ffi::CStr::from_ptr(name_ptr);
            if name_cstr.to_bytes() == b"ParadeDB Join Scan" {
                let private_list = PgList::<pg_sys::Node>::from_pg((*custom_path).custom_private);
                if !private_list.is_empty() {
                    let private_data = PrivateData::from((*custom_path).custom_private);
                    // Return the plan from the existing JoinScan
                    let plan = private_data.join_clause.plan.clone();
                    let join_keys = plan.join_keys();
                    return Some((plan, join_keys));
                }
            }
        }
    } else if is_join_path(path) {
        // Reconstruct from standard join path
        let join_path = path as *mut pg_sys::JoinPath;
        let outer_path = (*join_path).outerjoinpath;
        let inner_path = (*join_path).innerjoinpath;
        if outer_path.is_null() || inner_path.is_null() {
            return None;
        }

        let outer_rel = (*outer_path).parent;
        let inner_rel = (*inner_path).parent;

        let (outer_node, mut keys) = collect_join_sources(root, outer_rel)?;
        let (inner_node, inner_keys) = collect_join_sources(root, inner_rel)?;
        keys.extend(inner_keys);

        let mut all_sources = outer_node.sources();
        all_sources.extend(inner_node.sources());

        // Extract keys for this level
        let join_restrict_info = (*join_path).joinrestrictinfo;
        let join_conditions = extract_join_conditions_from_list(join_restrict_info, &all_sources);

        let jointype = (*join_path).jointype;
        let parsed_jointype =
            match crate::postgres::customscan::joinscan::build::JoinType::try_from(jointype) {
                Ok(jt) => jt,
                Err(e) => {
                    crate::postgres::customscan::joinscan::JoinScan::add_planner_warning(
                        e.to_string(),
                        (),
                    );
                    return None;
                }
            };

        if join_conditions.equi_keys.is_empty() {
            return None;
        }

        // Reject if there are other conditions (filters) we can't handle yet
        if !join_conditions.other_conditions.is_empty() {
            return None;
        }

        // Validate that all join keys are fast fields.
        for jk in &join_conditions.equi_keys {
            // Find source by RTI
            let outer_source = all_sources.iter().find(|s| s.contains_rti(jk.outer_rti));
            let inner_source = all_sources.iter().find(|s| s.contains_rti(jk.inner_rti));

            match (outer_source, inner_source) {
                (Some(outer), Some(inner)) => {
                    let outer_heaprelid = outer.scan_info.heaprelid;
                    let outer_indexrelid = outer.scan_info.indexrelid;
                    let inner_heaprelid = inner.scan_info.heaprelid;
                    let inner_indexrelid = inner.scan_info.indexrelid;

                    let outer_hr = PgSearchRelation::open(outer_heaprelid);
                    let outer_ir = PgSearchRelation::open(outer_indexrelid);
                    let inner_hr = PgSearchRelation::open(inner_heaprelid);
                    let inner_ir = PgSearchRelation::open(inner_indexrelid);
                    if resolve_fast_field(jk.outer_attno as i32, &outer_hr.tuple_desc(), &outer_ir)
                        .is_none()
                        || resolve_fast_field(
                            jk.inner_attno as i32,
                            &inner_hr.tuple_desc(),
                            &inner_ir,
                        )
                        .is_none()
                    {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        let join_node = crate::postgres::customscan::joinscan::build::JoinNode {
            join_type: parsed_jointype,
            left: outer_node,
            right: inner_node,
            equi_keys: join_conditions.equi_keys.clone(),
            filter: None,
            subplan_id: None,
        };

        keys.extend(join_conditions.equi_keys);

        return Some((RelNode::Join(Box::new(join_node)), keys));
    }

    None
}

/// Determines whether the provided PostgreSQL path represents a standard physical join strategy
/// that we can intercept and execute via DataFusion.
unsafe fn is_join_path(path: *mut pg_sys::Path) -> bool {
    matches!(
        (*path).type_,
        pg_sys::NodeTag::T_NestPath | pg_sys::NodeTag::T_MergePath | pg_sys::NodeTag::T_HashPath
    )
}

/// Helper to resolve the final relation from an inner query's PlannerInfo (`root`).
/// A planned subquery has its own localized `root` with a `join_rel_list` and `simple_rel_array`.
/// This function attempts to find the "top-most" `RelOptInfo` representing the fully joined result
/// (or the single base relation if there is no join) so that we can recursively collect its sources.
unsafe fn find_final_rel(root: *mut pg_sys::PlannerInfo) -> *mut pg_sys::RelOptInfo {
    let mut final_rel = std::ptr::null_mut();

    let join_rels = pgrx::PgList::<pg_sys::RelOptInfo>::from_pg((*root).join_rel_list);
    let all_baserels = (*root).all_baserels;

    for rel in join_rels.iter_ptr() {
        if pgrx::pg_sys::bms_equal((*rel).relids, all_baserels) {
            final_rel = rel;
            break;
        }
    }

    if final_rel.is_null() && (*root).simple_rel_array_size > 1 {
        for i in 1..(*root).simple_rel_array_size {
            let rel = *(*root).simple_rel_array.add(i as usize);
            if !rel.is_null() && (*rel).reloptkind == pg_sys::RelOptKind::RELOPT_BASEREL {
                final_rel = rel;
                break;
            }
        }
    }

    final_rel
}

/// Attempts to extract a `T_SubPlan` node from a generalized expression clause, handling known wrapper node types.
/// Returns the `SubPlan`, a boolean indicating whether the subplan is logically negated (i.e. an Anti-Join),
/// and the localized inner `PlannerInfo` associated with the subquery.
unsafe fn extract_subplan_from_clause(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<(*mut pg_sys::SubPlan, bool, *mut pg_sys::PlannerInfo)> {
    if node.is_null() {
        return None;
    }

    let mut current_node = node;
    let mut is_anti = false;

    // Check for NOT (BoolExpr)
    if (*current_node).type_ == pg_sys::NodeTag::T_BoolExpr {
        let bool_expr = current_node as *mut pg_sys::BoolExpr;
        if (*bool_expr).boolop == pg_sys::BoolExprType::NOT_EXPR {
            let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
            if args.len() == 1 {
                current_node = args.get_ptr(0).unwrap();
                is_anti = true;
            }
        }
    }

    // Check for AlternativeSubPlan
    if (*current_node).type_ == pg_sys::NodeTag::T_AlternativeSubPlan {
        let alt = current_node as *mut pg_sys::AlternativeSubPlan;
        let subplans = PgList::<pg_sys::Node>::from_pg((*alt).subplans);
        if !subplans.is_empty() {
            current_node = subplans.get_ptr(0).unwrap();
        }
    }

    // Check for SubPlan
    if (*current_node).type_ == pg_sys::NodeTag::T_SubPlan {
        let subplan = current_node as *mut pg_sys::SubPlan;

        let glob = (*root).glob;
        let subroots = (*glob).subroots;
        let plan_id = (*subplan).plan_id;

        let inner_root = pgrx::pg_sys::list_nth(subroots, plan_id - 1) as *mut pg_sys::PlannerInfo;

        return Some((subplan, is_anti, inner_root));
    }

    None
}

/// Result of extracting a SubPlan from within an OR expression.
/// Contains the SubPlan info plus the outer column varno/attno
/// for which the IS NULL condition was found (used to build the
/// post-LeftMark-join filter: `mark = true OR outer_col IS NULL`).
struct OrSubPlanExtraction {
    subplan: *mut pg_sys::SubPlan,
    is_anti: bool,
    inner_root: *mut pg_sys::PlannerInfo,
    /// The outer variable's varno and varattno for the IS NULL branch.
    null_test_varno: pg_sys::Index,
    null_test_attno: pg_sys::AttrNumber,
}

/// Attempts to extract a `T_SubPlan` node from an OR expression that combines
/// an `IS NULL` test with an `IN (SubPlan)` / `NOT IN (SubPlan)` test on the
/// same column.
///
/// Recognises patterns like:
///   `col IS NULL OR col IN (SELECT ...)`
///   `col IS NULL OR NOT col IN (SELECT ...)`
///
/// Returns the SubPlan, negation flag, inner PlannerInfo, and the column
/// targeted by the IS NULL test (needed for the post-LeftMark filter).
unsafe fn extract_subplan_from_or_clause(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<OrSubPlanExtraction> {
    if node.is_null() {
        return None;
    }

    // Must be an OR BoolExpr
    if (*node).type_ != pg_sys::NodeTag::T_BoolExpr {
        return None;
    }
    let bool_expr = node as *mut pg_sys::BoolExpr;
    if (*bool_expr).boolop != pg_sys::BoolExprType::OR_EXPR {
        return None;
    }

    let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
    if args.len() != 2 {
        return None; // Only handle two-branch OR for now
    }

    let raw_arg0 = unwrap_restrict_info(args.get_ptr(0)?);
    let raw_arg1 = unwrap_restrict_info(args.get_ptr(1)?);

    // Try both orderings: (NullTest, SubPlan) and (SubPlan, NullTest)
    try_extract_null_and_subplan(root, raw_arg0, raw_arg1)
        .or_else(|| try_extract_null_and_subplan(root, raw_arg1, raw_arg0))
}

/// Unwrap a `RestrictInfo` node to its inner clause. Returns the node unchanged
/// if it is not a RestrictInfo.
unsafe fn unwrap_restrict_info(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if !node.is_null() && (*node).type_ == pg_sys::NodeTag::T_RestrictInfo {
        let ri = node as *mut pg_sys::RestrictInfo;
        (*ri).clause.cast()
    } else {
        node
    }
}

/// Helper: given a candidate null_arg (expected IS NULL) and subplan_arg (expected SubPlan),
/// try to extract the pieces.
unsafe fn try_extract_null_and_subplan(
    root: *mut pg_sys::PlannerInfo,
    null_arg: *mut pg_sys::Node,
    subplan_arg: *mut pg_sys::Node,
) -> Option<OrSubPlanExtraction> {
    // --- Validate the IS NULL side ---
    if (*null_arg).type_ != pg_sys::NodeTag::T_NullTest {
        return None;
    }
    let null_test = null_arg as *mut pg_sys::NullTest;
    if (*null_test).nulltesttype != pg_sys::NullTestType::IS_NULL {
        return None;
    }
    // The argument to IS NULL must be a Var
    let null_test_arg = (*null_test).arg as *mut pg_sys::Node;
    if null_test_arg.is_null() || (*null_test_arg).type_ != pg_sys::NodeTag::T_Var {
        return None;
    }
    let null_var = null_test_arg as *mut pg_sys::Var;
    let null_varno = (*null_var).varno as pg_sys::Index;
    let null_attno = (*null_var).varattno;

    // --- Validate the SubPlan side ---
    let (subplan, is_anti, inner_root) = extract_subplan_from_clause(root, subplan_arg)?;

    // Verify the SubPlan's testexpr references the same outer column as the IS NULL.
    // The testexpr is typically: outer_var = PARAM (or PARAM = outer_var).
    let testexpr = (*subplan).testexpr;
    if testexpr.is_null() {
        return None;
    }
    if (*testexpr).type_ != pg_sys::NodeTag::T_OpExpr {
        return None;
    }
    let opexpr = testexpr as *mut pg_sys::OpExpr;
    let te_args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    if te_args.len() != 2 {
        return None;
    }
    let te_arg0 = strip_wrappers(te_args.get_ptr(0)?);
    let te_arg1 = strip_wrappers(te_args.get_ptr(1)?);

    // Find the Var in testexpr (the outer column)
    let outer_var = if (*te_arg0).type_ == pg_sys::NodeTag::T_Var {
        te_arg0 as *mut pg_sys::Var
    } else if (*te_arg1).type_ == pg_sys::NodeTag::T_Var {
        te_arg1 as *mut pg_sys::Var
    } else {
        return None;
    };

    // The outer var in testexpr must match the IS NULL var
    if (*outer_var).varno as pg_sys::Index != null_varno || (*outer_var).varattno != null_attno {
        return None;
    }

    Some(OrSubPlanExtraction {
        subplan,
        is_anti,
        inner_root,
        null_test_varno: null_varno,
        null_test_attno: null_attno,
    })
}

/// Extracts equi-join keys from a subplan's testexpr for `Semi`/`Anti` joins.
unsafe fn extract_equi_keys_from_subplan(
    subplan: *mut pg_sys::SubPlan,
    inner_root: *mut pg_sys::PlannerInfo,
    current_node: &RelNode,
    inner_node: &RelNode,
) -> Vec<JoinKeyPair> {
    let mut equi_keys = Vec::new();
    let testexpr = (*subplan).testexpr;
    if !testexpr.is_null() && (*testexpr).type_ == pg_sys::NodeTag::T_OpExpr {
        let opexpr = testexpr as *mut pg_sys::OpExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
        if args.len() == 2 {
            let arg0 = args.get_ptr(0).unwrap();
            let arg1 = args.get_ptr(1).unwrap();

            // Check if operator is an equality operator
            let opno = (*opexpr).opno;
            let is_equality_op = lookup_operator(opno) == Some("=");

            if is_equality_op {
                let mut var_node = std::ptr::null_mut::<pg_sys::Var>();

                let stripped_arg0 = strip_wrappers(arg0);
                let stripped_arg1 = strip_wrappers(arg1);

                if (*stripped_arg0).type_ == pg_sys::NodeTag::T_Var
                    && (*stripped_arg1).type_ == pg_sys::NodeTag::T_Param
                {
                    var_node = stripped_arg0 as *mut pg_sys::Var;
                } else if (*stripped_arg1).type_ == pg_sys::NodeTag::T_Var
                    && (*stripped_arg0).type_ == pg_sys::NodeTag::T_Param
                {
                    var_node = stripped_arg1 as *mut pg_sys::Var;
                }

                if !var_node.is_null() {
                    let varno = (*var_node).varno as pg_sys::Index;
                    let attno = (*var_node).varattno;

                    let current_sources = current_node.sources();
                    let inner_sources = inner_node.sources();

                    let outer_source = find_source_for_var(&current_sources, varno, attno);

                    let inner_source = resolve_subplan_output_var(inner_root).and_then(
                        |(inner_varno, inner_attno)| {
                            find_source_for_var(&inner_sources, inner_varno, inner_attno)
                        },
                    );

                    if let (Some((outer_rti, outer_attno)), Some((inner_rti, inner_attno))) =
                        (outer_source, inner_source)
                    {
                        let type_oid = (*var_node).vartype;
                        let (typlen, typbyval) = get_type_info(type_oid);

                        equi_keys.push(JoinKeyPair {
                            outer_rti,
                            outer_attno,
                            inner_rti,
                            inner_attno,
                            type_oid,
                            typlen,
                            typbyval,
                        });
                    }
                }
            }
        }
    }
    equi_keys
}

/// Resolve the base-table Var exported by an IN/NOT IN subquery's targetlist.
unsafe fn resolve_subplan_output_var(
    inner_root: *mut pg_sys::PlannerInfo,
) -> Option<(pg_sys::Index, pg_sys::AttrNumber)> {
    if inner_root.is_null() || (*inner_root).parse.is_null() {
        return None;
    }

    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*(*inner_root).parse).targetList);
    let te = targetlist.iter_ptr().find(|te| !(*(*te)).resjunk)?;
    let expr = strip_wrappers((*te).expr.cast());
    let var = nodecast!(Var, T_Var, expr)?;
    Some(((*var).varno as pg_sys::Index, (*var).varattno))
}
/// If `clause` is a `T_OpExpr` implementing equality between two Var nodes (one
/// from each side of the join), produce the corresponding [`JoinKeyPair`].
unsafe fn equi_key_pair_from_opexpr(
    clause: *mut pg_sys::Node,
    sources: &[&JoinSource],
) -> Option<JoinKeyPair> {
    if clause.is_null() || (*clause).type_ != pg_sys::NodeTag::T_OpExpr {
        return None;
    }
    let opexpr = clause as *mut pg_sys::OpExpr;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    if args.len() != 2 {
        return None;
    }

    let opno = (*opexpr).opno;
    if lookup_operator(opno) != Some("=") {
        return None;
    }

    let arg0 = strip_wrappers(args.get_ptr(0)?);
    let arg1 = strip_wrappers(args.get_ptr(1)?);

    if (*arg0).type_ != pg_sys::NodeTag::T_Var || (*arg1).type_ != pg_sys::NodeTag::T_Var {
        return None;
    }
    let var0 = arg0 as *mut pg_sys::Var;
    let var1 = arg1 as *mut pg_sys::Var;

    let varno0 = (*var0).varno as pg_sys::Index;
    let varno1 = (*var1).varno as pg_sys::Index;
    let attno0 = (*var0).varattno;
    let attno1 = (*var1).varattno;

    let (rti0, att0) = find_source_for_var(sources, varno0, attno0)?;
    let (rti1, att1) = find_source_for_var(sources, varno1, attno1)?;

    let type_oid = (*var0).vartype;
    let (typlen, typbyval) = get_type_info(type_oid);

    Some(JoinKeyPair {
        outer_rti: rti0,
        outer_attno: att0,
        inner_rti: rti1,
        inner_attno: att1,
        type_oid,
        typlen,
        typbyval,
    })
}

/// Parses a given list of `RestrictInfo` nodes to extract equi-join conditions and other join filters.
/// Iterates over the given restrict list and groups conditions according to whether they are
/// standard join keys or general functional predicates.
unsafe fn extract_join_conditions_from_list(
    restrictlist: *mut pg_sys::List,
    sources: &[&JoinSource],
) -> JoinConditions {
    let mut result = JoinConditions {
        equi_keys: Vec::new(),
        other_conditions: Vec::new(),
        has_search_predicate: false,
    };

    let list = PgList::<pg_sys::RestrictInfo>::from_pg(restrictlist);
    for ri in list.iter_ptr() {
        let clause = (*ri).clause;
        if clause.is_null() {
            continue;
        }

        // Check if this clause contains our @@@ operator
        let search_op = anyelement_query_input_opoid();
        if expr_contains_any_operator(clause.cast(), &[search_op]) {
            result.has_search_predicate = true;
        }

        // Try to identify equi-join conditions (OpExpr with Var = Var using equality operator)
        let mut is_equi_join = false;

        if let Some(jk) = equi_key_pair_from_opexpr(clause.cast(), sources) {
            result.equi_keys.push(jk);
            is_equi_join = true;
        }

        if !is_equi_join {
            let has_search_op = expr_contains_any_operator(clause.cast(), &[search_op]);
            if !has_search_op {
                result.other_conditions.push(ri);
            }
        }
    }

    result
}

/// Attempts to map a PostgreSQL variable reference (RTI and attribute number) to its origin
/// among a list of collected base `JoinSource` candidates.
fn find_source_for_var(
    sources: &[&JoinSource],
    varno: pg_sys::Index,
    attno: pg_sys::AttrNumber,
) -> Option<(pg_sys::Index, pg_sys::AttrNumber)> {
    for source in sources {
        if let Some(mapped_attno) = source.map_var(varno, attno) {
            return Some((varno, mapped_attno));
        }
    }
    None
}

/// Collect all required fields for execution.
///
/// This iterates over various parts of the query plan to ensure that all necessary
/// columns are available during execution:
/// 1. CTID: Always required for fetching results.
/// 2. Join Keys: Required for the join condition.
/// 3. Filters: Columns used in join-level filters (custom_exprs).
/// 4. Order By: Columns used for sorting.
pub(super) unsafe fn collect_required_fields(
    join_clause: &mut JoinCSClause,
    output_columns: &[OutputColumnInfo],
    custom_exprs: *mut pg_sys::List,
) {
    // Resolve each equi-join key against the join node that owns it so we
    // bind to the correct `JoinSource` by `plan_position`. A flat
    // `(rti, attno)` scan can pick the wrong source when a SubPlan's inner
    // relation shares an RTI value with an outer relation (inner queries
    // have their own RTI numbering space), forcing us to over-project.
    let join_key_projections = join_clause.plan.join_key_projections();
    let mut plan_sources = join_clause.plan.sources_mut();

    for source in &mut plan_sources {
        ensure_ctid(source);
    }

    if plan_sources.len() >= 2 {
        for (plan_position, attno) in &join_key_projections {
            if let Some(source) = plan_sources
                .iter_mut()
                .find(|s| s.plan_position == *plan_position)
            {
                ensure_field(source, *attno);
            }
        }
    }

    let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
    for expr_node in expr_list.iter_ptr() {
        let vars = expr_collect_vars(expr_node, true);
        for var in vars {
            if var.rti == pg_sys::INDEX_VAR as pg_sys::Index {
                let idx = (var.attno - 1) as usize;
                if let Some(OutputColumnInfo::Var {
                    rti,
                    original_attno,
                    ..
                }) = output_columns.get(idx)
                {
                    if *original_attno > 0 {
                        ensure_column_in_all_sources(&mut plan_sources, *rti, *original_attno);
                    }
                }
            } else {
                ensure_column_in_all_sources(&mut plan_sources, var.rti, var.attno);
            }
        }
    }

    for info in &join_clause.order_by {
        let feature = match &info.feature {
            OrderByFeature::NullTest { inner, .. } => inner.as_ref(),
            other => other,
        };
        match feature {
            OrderByFeature::Var { rti, attno, .. } => {
                ensure_column_in_all_sources(&mut plan_sources, *rti, *attno);
            }
            OrderByFeature::Field {
                name: name_wrapper,
                rti,
            } => {
                let name = name_wrapper.as_ref();
                if let Some((alias, col_name)) = name.split_once('.') {
                    let raw_col_name = col_name.trim_matches('"');
                    for source in &mut plan_sources {
                        if source.scan_info.alias.as_deref() == Some(alias) {
                            if let Some(attno) = get_source_attno_by_name(source, raw_col_name) {
                                ensure_field(source, attno);
                            }
                            break;
                        }
                    }
                } else {
                    // Unqualified field name (e.g. from indexed expression) — use RTI
                    // to find the correct source and ensure the column is projected.
                    for source in &mut plan_sources {
                        if source.contains_rti(*rti) {
                            // Try as a regular column first, then fall back to
                            // expression-indexed field.  The column `name` may
                            // exist in the table but only be indexed via an
                            // expression (e.g. upper(name)), in which case
                            // ensure_field (via resolve_fast_field) won't find it.
                            let added = get_source_attno_by_name(source, name)
                                .and_then(|attno| try_ensure_field(source, attno))
                                .is_some();
                            if !added {
                                if let Err(e) = ensure_expression_field(source, name) {
                                    pgrx::warning!("JoinScan: failed to project expression field '{name}': {e}");
                                }
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Ensure expression input vars are included so Tantivy emits the columns
    // that DISTINCT expressions depend on (e.g., `DISTINCT upper(name)` needs `name`).
    if let Some(projections) = &join_clause.output_projection {
        for proj in projections {
            if let super::build::ChildProjection::Expression { input_vars, .. } = proj {
                for var_info in input_vars {
                    ensure_column_in_all_sources(&mut plan_sources, var_info.rti, var_info.attno);
                }
            }
        }
    }
}

/// Ensures that a specific attribute from a relation is included in the output fields for a given `JoinSource`.
unsafe fn ensure_column(source: &mut JoinSource, rti: pg_sys::Index, attno: pg_sys::AttrNumber) {
    if source.contains_rti(rti) {
        ensure_field(source, attno);
    }
}

/// Broadcast [`ensure_column`] across every source in the plan. Each source
/// only acts on the call when its `contains_rti(rti)` is true, so this is the
/// idiomatic way to "make sure this `(rti, attno)` reference can be resolved
/// regardless of which source actually owns it" in `collect_required_fields`.
unsafe fn ensure_column_in_all_sources(
    sources: &mut [&mut JoinSource],
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
) {
    for source in sources {
        ensure_column(source, rti, attno);
    }
}

/// Automatically registers the internal `ctid` (tuple identifier) column to the required fields list.
/// Used during join evaluations and late materialization to retrieve heap tuples.
unsafe fn ensure_ctid(source: &mut JoinSource) {
    source.scan_info.add_field(
        pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
        WhichFastField::Ctid,
    );
}

/// Appends a specific attribute number to the list of output fields for a `JoinSource` if not already present.
unsafe fn ensure_field(side: &mut JoinSource, attno: pg_sys::AttrNumber) {
    if try_ensure_field(side, attno).is_none() {
        pgrx::warning!(
            "JoinScan: could not resolve fast field for attno {} on relation {}",
            attno,
            side.scan_info.heaprelid
        );
    }
}

/// Like `ensure_field`, but returns `Some(())` on success or `None` on failure
/// (instead of printing a warning).
unsafe fn try_ensure_field(side: &mut JoinSource, attno: pg_sys::AttrNumber) -> Option<()> {
    if side.scan_info.fields.iter().any(|f| f.attno == attno) {
        return Some(());
    }

    let heaprel = PgSearchRelation::open(side.scan_info.heaprelid);
    let indexrel = PgSearchRelation::open(side.scan_info.indexrelid);
    let field = resolve_fast_field(attno as i32, &heaprel.tuple_desc(), &indexrel)?;
    side.scan_info.add_field(attno, field);
    Some(())
}

/// Ensures an expression-indexed fast field is projected from a `JoinSource`.
///
/// Unlike `ensure_field` (which resolves plain columns via attno), this function
/// looks up a search field by name in the BM25 index schema and adds the
/// corresponding `WhichFastField` directly.  Used for ORDER BY on indexed
/// expressions like `upper(name)`, where the Tantivy field has no matching
/// PostgreSQL column attno.
unsafe fn ensure_expression_field(source: &mut JoinSource, field_name: &str) -> Result<(), String> {
    let index_rel = PgSearchRelation::open(source.scan_info.indexrelid);
    let schema = SearchIndexSchema::open(&index_rel).map_err(|e| {
        format!(
            "Failed to open schema for index {}: {e}",
            source.scan_info.indexrelid
        )
    })?;
    let search_field = schema
        .search_field(field_name)
        .ok_or_else(|| format!("Field '{field_name}' is not part of the schema"))?;
    if !search_field.is_fast() {
        return Err(format!("Field '{field_name}' is not a fast field"));
    }
    let categorized = schema.categorized_fields();
    let (_, data) = categorized
        .iter()
        .find(|(sf, _)| sf == &search_field)
        .ok_or_else(|| format!("Field '{field_name}' not found in categorized fields"))?;
    let field_type = field_type_for_pullup(search_field.field_type(), data.is_array)
        .ok_or_else(|| format!("Field '{field_name}' has unsupported type for pullup"))?;

    let synthetic_attno = -(source.scan_info.fields.len() as pg_sys::AttrNumber + 1);
    source.scan_info.add_field(
        synthetic_attno,
        WhichFastField::Named(field_name.to_string(), field_type),
    );
    Ok(())
}

/// Retrieve an attribute number by column name from a `JoinSource`'s heap relation.
unsafe fn get_source_attno_by_name(side: &JoinSource, name: &str) -> Option<pg_sys::AttrNumber> {
    let rel = PgSearchRelation::open(side.scan_info.heaprelid);
    let tupdesc = rel.tuple_desc();
    get_attno_by_name(name, &tupdesc)
}

fn collect_source_rtis(sources: &[&JoinSource]) -> Vec<pg_sys::Index> {
    sources.iter().map(|s| s.scan_info.heap_rti).collect()
}

/// Count query_pathkeys that reference at least one source relation (i.e. are
/// not outer-only). This is the number of pathkeys JoinScan is responsible for.
pub(super) unsafe fn count_relevant_pathkeys(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
) -> usize {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    let source_rtis = collect_source_rtis(sources);
    pathkeys
        .iter_ptr()
        .filter(|pk| !pathkey_is_outer_only((**pk).pk_eclass, &source_rtis))
        .count()
}

/// Returns true if no equivalence class member for this pathkey references any
/// relation in `source_rtis`. Such pathkeys are "outer-only" w.r.t. this join
/// subtree — the parent plan owns sorting on those keys.
unsafe fn pathkey_is_outer_only(
    equivclass: *mut pg_sys::EquivalenceClass,
    source_rtis: &[pg_sys::Index],
) -> bool {
    let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);
    for member in members.iter_ptr() {
        let check_expr = strip_wrappers((*member).em_expr.cast());

        if let Some(var) = nodecast!(Var, T_Var, check_expr) {
            if source_rtis.contains(&((*var).varno as pg_sys::Index)) {
                return false;
            }
        } else {
            let var_list = pg_sys::pull_var_clause(check_expr, PVC_RECURSE_ALL);
            let vars = PgList::<pg_sys::Var>::from_pg(var_list);
            for var_ptr in vars.iter_ptr() {
                if source_rtis.contains(&((*var_ptr).varno as pg_sys::Index)) {
                    return false;
                }
            }
        }
    }
    true
}

/// Check if all ORDER BY columns are fast fields.
///
/// For JoinScan to be proposed, all columns used in ORDER BY must be fast fields
/// in their respective BM25 indexes (or be paradedb.score() which is handled separately).
///
/// Pathkeys that reference only relations outside this join subtree ("outer-only")
/// are skipped — the parent plan is responsible for sorting on those keys.
///
/// Returns true if:
/// - No ORDER BY clause exists
/// - All relevant ORDER BY columns are fast fields or score functions
///
/// Returns false if any relevant ORDER BY column is not a fast field.
pub(super) unsafe fn order_by_columns_are_fast_fields(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
    has_distinct: bool,
) -> bool {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return true;
    }

    let source_rtis = collect_source_rtis(sources);

    'pathkey: for pathkey_ptr in pathkeys.iter_ptr() {
        let equivclass = (*pathkey_ptr).pk_eclass;

        if pathkey_is_outer_only(equivclass, &source_rtis) {
            continue;
        }

        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;
            // Chain strip_wrappers (for PlaceHolderVar/RelabelType) then
            // strip_identity_wrappers (for identity OpExpr like `id + 0`).
            let mut expr = strip_identity_wrappers(strip_wrappers(expr.cast()));

            // Unwrap NullTest to inspect the inner expression
            if let Some(nt) = nodecast!(NullTest, T_NullTest, expr) {
                expr = strip_identity_wrappers(strip_wrappers((*nt).arg.cast()));
            }

            if sources
                .iter()
                .any(|s| is_score_func_recursive(expr.cast(), s))
            {
                continue 'pathkey;
            }

            if let Some(var) = nodecast!(Var, T_Var, expr) {
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                if !source_rtis.contains(&varno) {
                    continue;
                }

                for source in sources {
                    if source.contains_rti(varno) {
                        let hr = PgSearchRelation::open(source.scan_info.heaprelid);
                        let ir = PgSearchRelation::open(source.scan_info.indexrelid);
                        if resolve_fast_field(varattno as i32, &hr.tuple_desc(), &ir).is_some() {
                            continue 'pathkey;
                        }
                        break;
                    }
                }
            } else {
                let mut found = false;
                for source in sources {
                    let index_rel = PgSearchRelation::open(source.scan_info.indexrelid);
                    let Ok(schema) = SearchIndexSchema::open(&index_rel) else {
                        continue;
                    };
                    if find_matching_fast_field(
                        expr,
                        &index_rel.index_expressions(),
                        schema,
                        source.scan_info.heap_rti,
                    )
                    .is_some()
                    {
                        found = true;
                        break;
                    }
                }

                if !found && has_distinct && expression_vars_all_fast(expr.cast(), sources) {
                    found = true;
                }

                if found {
                    continue 'pathkey;
                }
            }
        }

        return false;
    }

    true
}

/// Check if all Var dependencies of an expression are fast fields in any source.
/// Returns true if ALL Var deps are fast fields, false if any is missing or no vars found.
/// Also rejects aggregates and window functions which ExecEvalExpr cannot evaluate.
unsafe fn expression_vars_all_fast(expr: *mut pg_sys::Node, sources: &[&JoinSource]) -> bool {
    if pg_sys::contain_agg_clause(expr) || pg_sys::contain_window_function(expr) {
        return false;
    }

    let var_list = pg_sys::pull_var_clause(expr, PVC_RECURSE_ALL);
    let vars = PgList::<pg_sys::Var>::from_pg(var_list);
    if vars.is_empty() {
        return false;
    }
    for var_ptr in vars.iter_ptr() {
        let vno = (*var_ptr).varno as pg_sys::Index;
        let vattno = (*var_ptr).varattno;
        let found = sources.iter().any(|s| {
            if !s.contains_rti(vno) {
                return false;
            }
            let hr = PgSearchRelation::open(s.scan_info.heaprelid);
            let ir = PgSearchRelation::open(s.scan_info.indexrelid);
            let td = hr.tuple_desc();
            resolve_fast_field(vattno as i32, &td, &ir).is_some()
        });
        if !found {
            return false;
        }
    }
    true
}

/// Represents a parsed DISTINCT target list entry.
pub(super) enum ResolvedExpr {
    /// Simple column reference (existing behavior)
    Column {
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
    },
    /// Score function (existing behavior)
    Score { rti: pg_sys::Index },
    /// An indexed expression that matched via find_matching_fast_field.
    /// Handled by existing machinery — does NOT need the UDF path.
    IndexedExpression { rti: pg_sys::Index },
    /// Arbitrary expression with its Var dependencies and resolved type info
    Expression {
        expr_node: *mut pg_sys::Expr,
        input_vars: Vec<InputVarInfo>,
        result_type: pg_sys::Oid,
    },
}

// ---------------------------------------------------------------------------
// Diagnostic helpers for DISTINCT expression decline messages
// ---------------------------------------------------------------------------

/// Human-readable PG type name (e.g., "jsonb", "integer").
unsafe fn format_type_name(type_oid: pg_sys::Oid) -> String {
    let c_str = pg_sys::format_type_be(type_oid);
    if c_str.is_null() {
        return format!("OID {}", type_oid);
    }
    std::ffi::CStr::from_ptr(c_str)
        .to_string_lossy()
        .into_owned()
}

/// Get table name from a source's heap relation OID.
unsafe fn source_table_name(source: &JoinSource) -> String {
    let relname = pg_sys::get_rel_name(source.scan_info.heaprelid);
    if !relname.is_null() {
        std::ffi::CStr::from_ptr(relname)
            .to_string_lossy()
            .into_owned()
    } else {
        format!("rti {}", source.scan_info.heap_rti)
    }
}

/// Get a "table.column" name for a Var reference, for diagnostic messages.
unsafe fn column_name_for_var(
    sources: &[&JoinSource],
    varno: pg_sys::Index,
    varattno: pg_sys::AttrNumber,
) -> String {
    for source in sources {
        if source.contains_rti(varno) {
            let hr = PgSearchRelation::open(source.scan_info.heaprelid);
            let td = hr.tuple_desc();
            if varattno > 0 && (varattno as usize) <= td.len() {
                if let Some(attr) = td.get((varattno - 1) as usize) {
                    let tbl = source_table_name(source);
                    return format!("{}.{}", tbl, attr.name());
                }
            }
        }
    }
    format!("rti {}, attno {}", varno, varattno)
}

/// Check if all DISTINCT columns are fast fields in their respective BM25 indexes.
///
/// DISTINCT requires all target columns to be available as fast fields so that
/// deduplication can happen within DataFusion without heap access.
/// Walks `parse->distinctClause` (a list of SortGroupClause), resolves each to
/// its TargetEntry, and checks the referenced Var against source fast fields.
///
/// Returns `Some(entries)` if all DISTINCT columns are fast fields, `None` otherwise.
/// When there is no DISTINCT clause, returns `Some(vec![])`.
pub(super) unsafe fn distinct_columns_are_fast_fields(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
) -> Option<Vec<ResolvedExpr>> {
    let parse = (*root).parse;
    if (*parse).distinctClause.is_null() {
        return Some(vec![]);
    }

    // Build table names once for diagnostic messages in Case 4 decline points.
    let tables_str = sources
        .iter()
        .map(|s| source_table_name(s))
        .collect::<Vec<_>>()
        .join(", ");

    let distinct_list = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).distinctClause);
    let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);

    let mut entries = Vec::new();

    for clause_ptr in distinct_list.iter_ptr() {
        let tle_ref = (*clause_ptr).tleSortGroupRef;
        let te = target_list
            .iter_ptr()
            .find(|te| (**te).ressortgroupref == tle_ref);

        let te = te?;

        let expr = (*te).expr as *mut pg_sys::Node;

        // Case 1: Plain column reference (Var node)
        if let Some(var) = nodecast!(Var, T_Var, expr) {
            let varno = (*var).varno as pg_sys::Index;
            let is_fast = sources.iter().any(|source| {
                if !source.contains_rti(varno) {
                    return false;
                }
                let hr = PgSearchRelation::open(source.scan_info.heaprelid);
                let ir = PgSearchRelation::open(source.scan_info.indexrelid);
                let td = hr.tuple_desc();
                resolve_fast_field((*var).varattno as i32, &td, &ir).is_some()
            });
            if !is_fast {
                return None;
            }
            entries.push(ResolvedExpr::Column {
                rti: varno,
                attno: (*var).varattno,
            });
            continue;
        }

        // Case 2: Score function
        if let Some(rti) = get_score_func_rti(expr.cast()) {
            entries.push(ResolvedExpr::Score { rti });
            continue;
        }

        // Case 3: Check if expression matches an indexed expression (existing behavior)
        let matched_source = sources.iter().find(|source| {
            let index_rel = PgSearchRelation::open(source.scan_info.indexrelid);
            let Ok(schema) = SearchIndexSchema::open(&index_rel) else {
                return false;
            };
            find_matching_fast_field(
                expr,
                &index_rel.index_expressions(),
                schema,
                source.scan_info.heap_rti,
            )
            .is_some()
        });
        if let Some(source) = matched_source {
            // Indexed expressions are handled by existing fast field machinery.
            // They don't need the UDF path. The attno=0 convention for indexed
            // expressions is already handled by build_projection_expr.
            entries.push(ResolvedExpr::IndexedExpression {
                rti: source.scan_info.heap_rti,
            });
            continue;
        }

        // Case 4: Expression with Var dependencies — walk the expression tree
        // to find all referenced Var nodes and verify each is a fast field.

        if pg_sys::contain_agg_clause(expr) {
            pgrx::debug1!(
                "JoinScan declined: DISTINCT expression contains an aggregate \
                 function (tables: {})",
                tables_str
            );
            return None;
        }
        if pg_sys::contain_window_function(expr) {
            pgrx::debug1!(
                "JoinScan declined: DISTINCT expression contains a window \
                 function (tables: {})",
                tables_str
            );
            return None;
        }

        let var_list = pg_sys::pull_var_clause(expr, PVC_RECURSE_ALL);
        let vars = PgList::<pg_sys::Var>::from_pg(var_list);

        if vars.is_empty() {
            pgrx::debug1!(
                "JoinScan declined: DISTINCT expression is a constant with \
                 no column dependencies (tables: {})",
                tables_str
            );
            return None;
        }

        let mut input_vars = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for var_ptr in vars.iter_ptr() {
            let varno = (*var_ptr).varno as pg_sys::Index;
            let varattno = (*var_ptr).varattno;

            if !seen.insert((varno, varattno)) {
                continue;
            }

            let source = sources.iter().find(|s| s.contains_rti(varno));
            match source {
                Some(source) => {
                    let hr = PgSearchRelation::open(source.scan_info.heaprelid);
                    let ir = PgSearchRelation::open(source.scan_info.indexrelid);
                    let td = hr.tuple_desc();
                    if resolve_fast_field(varattno as i32, &td, &ir).is_none() {
                        let col = column_name_for_var(sources, varno, varattno);
                        pgrx::debug1!(
                            "JoinScan declined: DISTINCT expression depends on '{}' \
                             which is not a fast field (rti={}, attno={}, heaprelid={}) \
                             (tables: {})",
                            col,
                            varno,
                            varattno,
                            source.scan_info.heaprelid,
                            tables_str
                        );
                        return None;
                    }
                    input_vars.push(InputVarInfo {
                        rti: varno,
                        attno: varattno,
                        type_oid: (*var_ptr).vartype,
                        typmod: (*var_ptr).vartypmod,
                        collation: (*var_ptr).varcollid,
                    });
                }
                None => {
                    pgrx::debug1!(
                        "JoinScan declined: DISTINCT expression depends on column \
                         (rti={}, attno={}) not found in any source (available: {:?}) \
                         (tables: {})",
                        varno,
                        varattno,
                        sources
                            .iter()
                            .map(|s| s.scan_info.heap_rti)
                            .collect::<Vec<_>>(),
                        tables_str
                    );
                    return None;
                }
            }
        }

        let result_type = pg_sys::exprType(expr);

        if !crate::postgres::types_arrow::is_arrow_convertible(result_type) {
            let type_name = format_type_name(result_type);
            pgrx::debug1!(
                "JoinScan declined: DISTINCT expression returns type '{}' \
                 (OID {}) which is not supported for Arrow conversion \
                 (tables: {})",
                type_name,
                result_type,
                tables_str
            );
            return None;
        }

        entries.push(ResolvedExpr::Expression {
            expr_node: expr.cast(),
            input_vars,
            result_type,
        });
    }

    Some(entries)
}

/// Check if any pathkey (ORDER BY clause) uses paradedb.score() referencing a specific relation.
pub(super) unsafe fn pathkey_uses_scores_from_source(
    root: *mut pg_sys::PlannerInfo,
    source: &JoinSource,
) -> bool {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return false;
    }

    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;
            let check_expr = strip_wrappers(expr.cast()).cast::<pg_sys::Expr>();

            if is_score_func_recursive(check_expr, source) {
                return true;
            }
        }
    }

    false
}

/// Recursively peels `RelabelType` and `PlaceHolderVar` wrappers to get the underlying node.
unsafe fn strip_wrappers(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    loop {
        if node.is_null() {
            return node;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_RelabelType => {
                node = (*(node as *mut pg_sys::RelabelType)).arg.cast();
            }
            pg_sys::NodeTag::T_PlaceHolderVar => {
                node = (*(node as *mut pg_sys::PlaceHolderVar)).phexpr.cast();
            }
            _ => break,
        }
    }
    node
}

/// Extracts the RTI of the variable passed to a `paradedb.score(var)` function call.
/// Handles implicit casts and placeholder wrappers.
pub(super) unsafe fn get_score_func_rti(expr: *mut pg_sys::Expr) -> Option<pg_sys::Index> {
    if expr.is_null() {
        return None;
    }
    let stripped_expr = strip_wrappers(expr.cast());
    if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, stripped_expr) {
        let args = PgList::<pg_sys::Node>::from_pg((*func).args);
        if !args.is_empty() {
            if let Some(arg) = args.get_ptr(0) {
                let stripped_arg = strip_wrappers(arg);
                if let Some(var) = nodecast!(Var, T_Var, stripped_arg) {
                    let varno = (*var).varno as pg_sys::Index;
                    if is_score_func(stripped_expr.cast(), varno) {
                        return Some(varno);
                    }
                }
            }
        }
    }
    None
}

/// Sets `score_needed` on the ordering base relation.
/// Returns the RTI of the ordering base relation if found.
pub(super) fn ensure_score_bubbling(source: &mut JoinSource) -> Option<pg_sys::Index> {
    source.scan_info.score_needed = true;
    source.scan_info.add_field(0, WhichFastField::Score);
    Some(source.scan_info.heap_rti)
}

/// Check if an expression is a `paradedb.score()` call referencing a relation in the given source.
unsafe fn is_score_func_recursive(expr: *mut pg_sys::Expr, source: &JoinSource) -> bool {
    if expr.is_null() {
        return false;
    }
    if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, expr) {
        let args = PgList::<pg_sys::Node>::from_pg((*func).args);
        if !args.is_empty() {
            if let Some(arg) = args.get_ptr(0) {
                if let Some(var) = nodecast!(Var, T_Var, arg) {
                    let varno = (*var).varno as pg_sys::Index;
                    if source.contains_rti(varno) {
                        return is_score_func(expr.cast(), varno);
                    }
                }
            }
        }
    }
    false
}

/// Single-expression classification for join sort keys (score / heap Var / indexed `Field`).
///
/// `pathkey_equivalence_member`: when true, a `Var` outside `output_rtis` means "try the next
/// equivalence member". When false (sortClause-only extraction), that situation fails the whole
/// extraction.
enum JoinSortExprKind {
    Resolved(OrderByInfo),
    /// Pathkeys only: this EC member is not usable.
    SkipMember,
    /// No encoding as Score, Var, or indexed Field.
    NoMatch,
}

impl JoinSortExprKind {
    unsafe fn classify(
        check_expr: *mut pg_sys::Expr,
        direction: SortDirection,
        sources: &[&JoinSource],
        output_rtis: &[pg_sys::Index],
        pathkey_equivalence_member: bool,
    ) -> Self {
        // Strip order-preserving wrappers (e.g. `id + 0`, RelabelType, CoerceToDomain)
        // so that the expression reaches the pattern-matching below in canonical form.
        let check_expr =
            strip_identity_wrappers(check_expr as *mut pg_sys::Node) as *mut pg_sys::Expr;

        if let Some(nt) = nodecast!(NullTest, T_NullTest, check_expr) {
            let inner_expr = strip_wrappers((*nt).arg.cast()).cast::<pg_sys::Expr>();
            let nulltesttype = if (*nt).nulltesttype == pg_sys::NullTestType::IS_NULL {
                NullTestKind::IsNull
            } else {
                NullTestKind::IsNotNull
            };
            return match Self::classify(
                inner_expr,
                direction,
                sources,
                output_rtis,
                pathkey_equivalence_member,
            ) {
                Self::Resolved(inner_info) => Self::Resolved(OrderByInfo {
                    feature: OrderByFeature::NullTest {
                        inner: Box::new(inner_info.feature),
                        nulltesttype,
                    },
                    direction,
                }),
                other => other,
            };
        }

        for source in sources.iter() {
            if is_score_func_recursive(check_expr.cast(), source) {
                if !output_rtis.contains(&source.scan_info.heap_rti) {
                    continue;
                }
                return Self::Resolved(OrderByInfo {
                    feature: OrderByFeature::Score {
                        rti: source.scan_info.heap_rti,
                    },
                    direction,
                });
            }
        }

        if let Some(var) = nodecast!(Var, T_Var, check_expr) {
            let varno = (*var).varno as pg_sys::Index;
            let varattno = (*var).varattno;

            if !output_rtis.contains(&varno) {
                return if pathkey_equivalence_member {
                    Self::SkipMember
                } else {
                    Self::NoMatch
                };
            }

            for source in sources {
                if source.contains_rti(varno) {
                    let name = find_base_info_recursive(source, varno).and_then(|info| {
                        fieldname_from_var(info.heaprelid, var, varattno).map(|f| f.to_string())
                    });
                    return Self::Resolved(OrderByInfo {
                        feature: OrderByFeature::Var {
                            rti: varno,
                            attno: varattno,
                            name,
                        },
                        direction,
                    });
                }
            }

            // At this point output_rtis.contains(&varno) is guaranteed — we already
            // returned SkipMember/NoMatch above when !output_rtis.contains(&varno).
            debug_assert!(output_rtis.contains(&varno));
            if !sources.iter().any(|s| s.contains_rti(varno)) {
                return Self::Resolved(OrderByInfo {
                    feature: OrderByFeature::Var {
                        rti: varno,
                        attno: varattno,
                        name: None,
                    },
                    direction,
                });
            }

            return Self::NoMatch;
        }

        for source in sources {
            if !output_rtis.contains(&source.scan_info.heap_rti) {
                continue;
            }
            let index_rel = PgSearchRelation::open(source.scan_info.indexrelid);
            let Ok(schema) = SearchIndexSchema::open(&index_rel) else {
                continue;
            };
            if let Some(search_field) = find_matching_fast_field(
                check_expr as *mut pg_sys::Node,
                &index_rel.index_expressions(),
                schema,
                source.scan_info.heap_rti,
            ) {
                return Self::Resolved(OrderByInfo {
                    feature: OrderByFeature::Field {
                        name: search_field.name().into(),
                        rti: source.scan_info.heap_rti,
                    },
                    direction,
                });
            }
        }

        Self::NoMatch
    }
}

/// ORDER BY from `parse->sortClause` only. Used when JoinScan defers DISTINCT to a parent node
/// and `query_pathkeys` still list keys for the full DISTINCT row.
pub(super) unsafe fn extract_orderby_from_parse_sort_clause(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
    output_rtis: &[pg_sys::Index],
) -> Option<Vec<OrderByInfo>> {
    let parse = (*root).parse;
    if (*parse).sortClause.is_null() {
        return Some(Vec::new());
    }

    let sort_list = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
    let mut result = Vec::new();

    for sort_clause_ptr in sort_list.iter_ptr() {
        let direction =
            SortDirection::from_sort_op((*sort_clause_ptr).sortop, (*sort_clause_ptr).nulls_first)?;
        let sort_expr = pg_sys::get_sortgroupclause_expr(sort_clause_ptr, (*parse).targetList);
        let check_expr = strip_wrappers(sort_expr.cast()).cast::<pg_sys::Expr>();

        match JoinSortExprKind::classify(check_expr, direction, sources, output_rtis, false) {
            JoinSortExprKind::Resolved(info) => result.push(info),
            JoinSortExprKind::SkipMember => unreachable!("sortClause entry is not an EC member"),
            JoinSortExprKind::NoMatch => return None,
        }
    }

    Some(result)
}

/// Extract `ORDER BY` information from the Postgres query planner to pass down to the
/// DataFusion execution plan.
///
/// This function translates PostgreSQL's `PathKey`s (which represent requested sort orders)
/// into a format (`OrderByInfo`) that `JoinScan` and DataFusion can consume to construct a
/// physical `Sort` node.
///
/// # Equivalence Classes
/// In PostgreSQL, the planner bundles logically equivalent expressions into "Equivalence Classes"
/// (`ec_members`). For example, if a query includes the equi-join condition `a.id = b.id`
/// and orders by `b.id`, the planner considers sorting by `a.id` equally valid. Both variables
/// will be present in the `ec_members` list for that `PathKey`.
///
/// # Interaction with Pruned Relations (e.g., `SEMI JOIN`)
/// Certain join types, such as `LeftSemi` or `LeftAnti` joins, discard columns from one side
/// of the join. Continuing the above example, if the relation `b` is on the right side of a
/// Semi-Join, `b.id` will *not* be available in the output schema of the join operation.
/// If DataFusion attempts to sort on `b.id`, it will panic with a `SchemaError(FieldNotFound)`.
///
/// To prevent this, this function accepts `output_rtis`, a list of the Range Table Identifiers
/// (RTIs) that actually survive the entire relational tree defined in `JoinCSClause`.
/// When inspecting an Equivalence Class, the function searches for *any* member that belongs
/// to an RTI in `output_rtis`.
///
/// # Returns
/// - `Some(Vec<OrderByInfo>)`: The translated sort instructions containing valid, available columns.
/// - `None`: If the function encounters an `ORDER BY` pathkey where *none* of its Equivalence
///   Class members belong to the `output_rtis` list. This can happen in edge cases or complex
///   projections where Postgres asks for a sort on a variable not present in the local execution
///   context. Returning `None` signals the planner to abandon `JoinScan` and fall back to native
///   PostgreSQL execution.
pub(super) unsafe fn extract_orderby(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
    output_rtis: &[pg_sys::Index],
    has_distinct: bool,
) -> Option<Vec<OrderByInfo>> {
    let mut result = Vec::new();
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);

    if pathkeys.is_empty() || sources.is_empty() {
        return Some(result);
    }

    let source_rtis = collect_source_rtis(sources);

    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;

        if pathkey_is_outer_only(equivclass, &source_rtis) {
            continue;
        }
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        let nulls_first = (*pathkey).pk_nulls_first;
        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        let is_asc = match (*pathkey).pk_strategy as u32 {
            pg_sys::BTLessStrategyNumber => true,
            pg_sys::BTGreaterStrategyNumber => false,
            _ => true,
        };
        #[cfg(feature = "pg18")]
        let is_asc = match (*pathkey).pk_cmptype {
            pg_sys::CompareType::COMPARE_LT => true,
            pg_sys::CompareType::COMPARE_GT => false,
            _ => true,
        };

        let direction = match (is_asc, nulls_first) {
            (true, true) => SortDirection::AscNullsFirst,
            (true, false) => SortDirection::AscNullsLast,
            (false, true) => SortDirection::DescNullsFirst,
            (false, false) => SortDirection::DescNullsLast,
        };

        let mut pathkey_resolved = false;

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            let check_expr = strip_wrappers(expr.cast()).cast::<pg_sys::Expr>();

            match JoinSortExprKind::classify(check_expr, direction, sources, output_rtis, true) {
                JoinSortExprKind::Resolved(info) => {
                    // For DISTINCT queries, NullTest pathkeys come from the
                    // DISTINCT target list — they are handled by the GROUP BY,
                    // not the sort. Acknowledge the pathkey but don't add it to
                    // the ORDER BY list.
                    if has_distinct && matches!(info.feature, OrderByFeature::NullTest { .. }) {
                        pathkey_resolved = true;
                    } else {
                        result.push(info);
                        pathkey_resolved = true;
                    }
                }
                JoinSortExprKind::SkipMember => continue,
                JoinSortExprKind::NoMatch => {}
            }

            // DISTINCT adds non-Var expression pathkeys; skip when deps are fast fields.
            if !pathkey_resolved
                && has_distinct
                && nodecast!(Var, T_Var, check_expr).is_none()
                && expression_vars_all_fast(check_expr.cast(), sources)
            {
                pathkey_resolved = true;
            }
            if pathkey_resolved {
                break;
            }
        }

        if !pathkey_resolved {
            return None;
        }
    }

    Some(result)
}
