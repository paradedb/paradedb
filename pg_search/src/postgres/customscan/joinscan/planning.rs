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

use super::build::{JoinCSClause, JoinKeyPair, JoinSource, JoinSourceCandidate, RelNode};
use super::predicate::{find_base_info_recursive, is_column_fast_field};
use super::privdat::{OutputColumnInfo, PrivateData, SCORE_COL_NAME};

use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::score::is_score_func;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::opexpr::lookup_operator;
use crate::postgres::customscan::pullup::resolve_fast_field;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid};
use crate::postgres::customscan::score_funcoids;
use crate::postgres::customscan::CustomScan;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_vars, expr_contains_any_operator};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;

use pgrx::{pg_sys, PgList};

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
unsafe fn collect_join_sources_base_rel(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    rti: pg_sys::Index,
) -> Option<(RelNode, Vec<JoinKeyPair>)> {
    let rtable = (*(*root).parse).rtable;
    if rtable.is_null() {
        return None;
    }

    let rte = pg_sys::rt_fetch(rti, rtable);
    let relid = get_plain_relation_relid(rte)?;

    let mut side_info = JoinSourceCandidate::new(rti).with_heaprelid(relid);

    if !(*rte).eref.is_null() {
        let eref = (*rte).eref;
        if !(*eref).aliasname.is_null() {
            let alias_cstr = std::ffi::CStr::from_ptr((*eref).aliasname);
            if let Ok(alias) = alias_cstr.to_str() {
                side_info = side_info.with_alias(alias.to_string());
            }
        }
    }

    let mut extracted_subqueries = Vec::new();

    if let Some((_, bm25_index)) = rel_get_bm25_index(relid) {
        side_info = side_info.with_indexrelid(bm25_index.oid());

        // Read the sort order from the index's relation options.
        // This allows DataFusion-based execution to leverage physical sort order
        // for optimizations like SortPreservingMergeExec and sort-merge joins.
        let sort_by = bm25_index.options().sort_by();
        let sort_order = sort_by.into_iter().next();
        side_info = side_info.with_sort_order(sort_order);

        // Extract single-table predicates from baserestrictinfo.
        // These are predicates like `p.description @@@ 'wireless'` that PostgreSQL
        // has pushed down to the base relation level.
        //
        // Note: Cross-table predicates (e.g., involving multiple tables in a join)
        // are handled separately via SearchPredicateUDF through filter pushdown.
        let baserestrictinfo = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);

        if !baserestrictinfo.is_empty() {
            let context = PlannerContext::from_planner(root);

            for ri in baserestrictinfo.iter_ptr() {
                let mut state = QualExtractState::default();
                if let Some(qual) = extract_quals(
                    &context,
                    rti,
                    ri.cast(), // extract_quals expects Node, so we cast the RestrictInfo
                    anyelement_query_input_opoid(),
                    crate::postgres::customscan::builders::custom_path::RestrictInfoType::BaseRelation,
                    &bm25_index,
                    false,
                    &mut state,
                    true,
                ) {
                    let query = SearchQueryInput::from(&qual);
                    // Merge into existing query using Boolean Must, or set it if not present
                    let current_query = side_info.query.take();
                    let new_query = match current_query {
                        Some(existing) => SearchQueryInput::Boolean {
                            must: vec![existing, query],
                            should: vec![],
                            must_not: vec![],
                        },
                        None => query,
                    };

                    side_info = side_info.with_query(new_query);
                    if state.uses_our_operator {
                        side_info = side_info.with_search_predicate();
                    }
                } else if let Some((subplan, is_anti, inner_root)) = extract_subplan_from_clause(root, (*ri).clause.cast()) {
                    extracted_subqueries.push((subplan, is_anti, inner_root));
                }
            }
        }
    }

    side_info.estimate_rows();
    let source = JoinSource::try_from(side_info).ok()?;

    let mut current_node = RelNode::Scan(Box::new(source));
    let mut all_keys = Vec::new();

    // Wrap current_node in Join nodes for each extracted subquery
    for (subplan, is_anti, inner_root) in extracted_subqueries {
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

        let equi_keys = extract_equi_keys_from_subplan(subplan, &current_node, &inner_node);

        let join_node = crate::postgres::customscan::joinscan::build::JoinNode {
            join_type: if is_anti {
                crate::postgres::customscan::joinscan::build::JoinType::Anti
            } else {
                crate::postgres::customscan::joinscan::build::JoinType::Semi
            },
            left: current_node,
            right: inner_node,
            equi_keys: equi_keys.clone(),
            filter: None,
        };

        all_keys.extend(equi_keys);
        current_node = RelNode::Join(Box::new(join_node));
    }

    Some((current_node, all_keys))
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

                    if !is_column_fast_field(outer_heaprelid, outer_indexrelid, jk.outer_attno)
                        || !is_column_fast_field(inner_heaprelid, inner_indexrelid, jk.inner_attno)
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

/// Extracts equi-join keys from a subplan's testexpr for `Semi`/`Anti` joins.
unsafe fn extract_equi_keys_from_subplan(
    subplan: *mut pg_sys::SubPlan,
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

                if (*arg0).type_ == pg_sys::NodeTag::T_Var
                    && (*arg1).type_ == pg_sys::NodeTag::T_Param
                {
                    var_node = arg0 as *mut pg_sys::Var;
                } else if (*arg1).type_ == pg_sys::NodeTag::T_Var
                    && (*arg0).type_ == pg_sys::NodeTag::T_Param
                {
                    var_node = arg1 as *mut pg_sys::Var;
                }

                if !var_node.is_null() {
                    let varno = (*var_node).varno as pg_sys::Index;
                    let attno = (*var_node).varattno;

                    // Since we don't have all sources easily here, we'll map the Var to the current_node
                    let current_sources = current_node.sources();
                    let inner_sources = inner_node.sources();

                    let outer_source = find_source_for_var(&current_sources, varno, attno);

                    // To find the inner mapping, we need to look at the target list of the subquery plan
                    // For now, let's just make a dummy mapping for the inner source if outer maps.
                    // In a full implementation, we'd map the Param to the subquery's target list.
                    if let Some((outer_rti, outer_attno)) = outer_source {
                        // Hack: Assume inner_rti is the first RTI of the inner node and attno is 1
                        // Real implementation requires mapping testexpr's Param to the inner plan's targetlist
                        let inner_rti = if !inner_sources.is_empty() {
                            inner_sources[0].scan_info.heap_rti
                        } else {
                            0
                        };

                        if inner_rti > 0 {
                            let type_oid = (*var_node).vartype;
                            let (typlen, typbyval) = get_type_info(type_oid);

                            equi_keys.push(JoinKeyPair {
                                outer_rti,
                                outer_attno,
                                inner_rti,
                                inner_attno: 1, // DUMMY
                                type_oid,
                                typlen,
                                typbyval,
                            });
                        }
                    }
                }
            }
        }
    }
    equi_keys
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

        if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
            let opexpr = clause as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

            // Equi-join: should have exactly 2 args, both Var nodes, AND use equality operator
            if args.len() == 2 {
                let arg0 = args.get_ptr(0).unwrap();
                let arg1 = args.get_ptr(1).unwrap();

                // Check if operator is an equality operator
                let opno = (*opexpr).opno;
                let is_equality_op = lookup_operator(opno) == Some("=");

                if is_equality_op
                    && (*arg0).type_ == pg_sys::NodeTag::T_Var
                    && (*arg1).type_ == pg_sys::NodeTag::T_Var
                {
                    let var0 = arg0 as *mut pg_sys::Var;
                    let var1 = arg1 as *mut pg_sys::Var;

                    let varno0 = (*var0).varno as pg_sys::Index;
                    let varno1 = (*var1).varno as pg_sys::Index;
                    let attno0 = (*var0).varattno;
                    let attno1 = (*var1).varattno;

                    // Try to map vars to sources
                    let source0 = find_source_for_var(sources, varno0, attno0);
                    let source1 = find_source_for_var(sources, varno1, attno1);

                    if let (Some((rti0, att0)), Some((rti1, att1))) = (source0, source1) {
                        let type_oid = (*var0).vartype;
                        let (typlen, typbyval) = get_type_info(type_oid);

                        result.equi_keys.push(JoinKeyPair {
                            outer_rti: rti0,
                            outer_attno: att0,
                            inner_rti: rti1,
                            inner_attno: att1,
                            type_oid,
                            typlen,
                            typbyval,
                        });
                        is_equi_join = true;
                    }
                }
            }
        }

        if !is_equi_join {
            let search_op = anyelement_query_input_opoid();
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
    let join_keys = join_clause.plan.join_keys();
    let mut plan_sources = join_clause.plan.sources_mut();

    for source in &mut plan_sources {
        ensure_ctid(source);
    }

    if plan_sources.len() >= 2 {
        for jk in &join_keys {
            for source in &mut plan_sources {
                ensure_column(source, jk.outer_rti, jk.outer_attno);
                ensure_column(source, jk.inner_rti, jk.inner_attno);
            }
        }
    }

    let expr_list = PgList::<pg_sys::Node>::from_pg(custom_exprs);
    for expr_node in expr_list.iter_ptr() {
        let vars = expr_collect_vars(expr_node, true);
        for var in vars {
            if var.rti == pg_sys::INDEX_VAR as pg_sys::Index {
                let idx = (var.attno - 1) as usize;
                if let Some(info) = output_columns.get(idx) {
                    if info.original_attno > 0 {
                        for source in &mut plan_sources {
                            ensure_column(source, info.rti, info.original_attno);
                        }
                    }
                }
            } else {
                for source in &mut plan_sources {
                    ensure_column(source, var.rti, var.attno);
                }
            }
        }
    }

    for info in &join_clause.order_by {
        match &info.feature {
            OrderByFeature::Var { rti, attno, .. } => {
                for source in &mut plan_sources {
                    ensure_column(source, *rti, *attno);
                }
            }
            OrderByFeature::Field(name_wrapper) => {
                let name = name_wrapper.as_ref();
                if let Some((alias, col_name)) = name.split_once('.') {
                    let raw_col_name = col_name.trim_matches('"');
                    for source in &mut plan_sources {
                        if source.scan_info.alias.as_deref() == Some(alias) {
                            if let Some(attno) = get_attno_by_name(source, raw_col_name) {
                                ensure_field(source, attno);
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Ensures that a specific attribute from a relation is included in the output fields for a given `JoinSource`.
unsafe fn ensure_column(source: &mut JoinSource, rti: pg_sys::Index, attno: pg_sys::AttrNumber) {
    if source.contains_rti(rti) {
        ensure_field(source, attno);
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
    if side.scan_info.fields.iter().any(|f| f.attno == attno) {
        return;
    }

    let heaprel = PgSearchRelation::open(side.scan_info.heaprelid);
    let indexrel = PgSearchRelation::open(side.scan_info.indexrelid);
    let tupdesc = heaprel.tuple_desc();

    if let Some(field) = resolve_fast_field(attno as i32, &tupdesc, &indexrel) {
        side.scan_info.add_field(attno, field);
        return;
    }

    pgrx::warning!(
        "ensure_field: failed for attno {} in relation {:?}",
        attno,
        side.scan_info.alias.clone()
    );
}

/// Helper function to retrieve an attribute number given a column name from a `JoinSource`'s underlying heap relation.
unsafe fn get_attno_by_name(side: &JoinSource, name: &str) -> Option<pg_sys::AttrNumber> {
    let rel = PgSearchRelation::open(side.scan_info.heaprelid);
    let tupdesc = rel.tuple_desc();
    for (i, att) in tupdesc.iter().enumerate() {
        if att.name() == name {
            return Some((i + 1) as pg_sys::AttrNumber);
        }
    }
    None
}

/// Check if all ORDER BY columns are fast fields.
///
/// For JoinScan to be proposed, all columns used in ORDER BY must be fast fields
/// in their respective BM25 indexes (or be paradedb.score() which is handled separately).
///
/// Returns true if:
/// - No ORDER BY clause exists
/// - All ORDER BY columns are fast fields or score functions
///
/// Returns false if any ORDER BY column is not a fast field.
pub(super) unsafe fn order_by_columns_are_fast_fields(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
) -> bool {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return true;
    }

    for pathkey_ptr in pathkeys.iter_ptr() {
        let equivclass = (*pathkey_ptr).pk_eclass;
        let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                if !phv.is_null() && !(*phv).phexpr.is_null() {
                    if let Some(_funcexpr) = nodecast!(FuncExpr, T_FuncExpr, (*phv).phexpr) {
                        continue;
                    }
                }
            }

            if let Some(var) = nodecast!(Var, T_Var, expr) {
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                let mut found = false;
                for source in sources {
                    if source.contains_rti(varno) {
                        if !is_column_fast_field(
                            source.scan_info.heaprelid,
                            source.scan_info.indexrelid,
                            varattno,
                        ) {
                            return false;
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            }
        }
    }

    true
}

/// Extract ORDER BY score pathkey for the ordering side.
///
/// This checks if the query has an ORDER BY clause with paradedb.score()
/// referencing the ordering side relation. If found, returns the OrderByStyle
/// that can be used to declare pathkeys on the CustomPath, eliminating the
/// need for PostgreSQL to add a separate Sort node.
///
/// Returns None if:
/// - No ORDER BY clause exists
/// - ORDER BY doesn't use paradedb.score()
/// - Score function references a different relation
pub(super) unsafe fn extract_score_pathkey(
    root: *mut pg_sys::PlannerInfo,
    ordering_side: &JoinSource,
) -> Option<OrderByStyle> {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return None;
    }

    let pathkey_ptr = pathkeys.iter_ptr().next()?;
    let pathkey = pathkey_ptr;
    let equivclass = (*pathkey).pk_eclass;
    let members = PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

    for member in members.iter_ptr() {
        let expr = (*member).em_expr;

        if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
            if !phv.is_null() && !(*phv).phexpr.is_null() {
                if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, (*phv).phexpr) {
                    if is_score_func_recursive(funcexpr.cast(), ordering_side) {
                        return Some(OrderByStyle::Score(pathkey));
                    }
                }
            }
        } else if is_score_func_recursive(expr.cast(), ordering_side) {
            return Some(OrderByStyle::Score(pathkey));
        }
    }

    None
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

/// Extract ORDER BY information for DataFusion execution.
pub(super) unsafe fn extract_orderby(
    root: *mut pg_sys::PlannerInfo,
    sources: &[&JoinSource],
    ordering_side_index: Option<usize>,
) -> Vec<OrderByInfo> {
    let mut result = Vec::new();
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);

    if pathkeys.is_empty() || sources.is_empty() {
        return result;
    }

    for pathkey_ptr in pathkeys.iter_ptr() {
        let pathkey = pathkey_ptr;
        let equivclass = (*pathkey).pk_eclass;
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

        for member in members.iter_ptr() {
            let expr = (*member).em_expr;

            let mut check_expr = expr;
            if let Some(phv) = nodecast!(PlaceHolderVar, T_PlaceHolderVar, expr) {
                if !phv.is_null() && !(*phv).phexpr.is_null() {
                    check_expr = (*phv).phexpr;
                }
            }

            // Check if ordering by score
            let mut score_found = false;
            for (i, source) in sources.iter().enumerate() {
                if is_score_func_recursive(check_expr.cast(), source) {
                    let is_ordering_source = Some(i) == ordering_side_index;

                    if is_ordering_source {
                        result.push(OrderByInfo {
                            feature: OrderByFeature::Score,
                            direction,
                        });
                    } else {
                        let alias = source.execution_alias(i);
                        result.push(OrderByInfo {
                            feature: OrderByFeature::Field(
                                format!("{}.{}", alias, SCORE_COL_NAME).into(),
                            ),
                            direction,
                        });
                    }
                    score_found = true;
                    break;
                }
            }
            if score_found {
                break;
            }

            if let Some(var) = nodecast!(Var, T_Var, expr) {
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                for source in sources {
                    if source.contains_rti(varno) {
                        // Try to find a display name (optional)
                        let name = find_base_info_recursive(source, varno).and_then(|info| {
                            fieldname_from_var(info.heaprelid, var, varattno).map(|f| f.to_string())
                        });

                        result.push(OrderByInfo {
                            feature: OrderByFeature::Var {
                                rti: varno,
                                attno: varattno,
                                name,
                            },
                            direction,
                        });
                        break;
                    }
                }
            }
        }
    }

    result
}
