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

// These functions are not yet called — they will be used by the planner
// integration in #4485. Suppress dead_code until then.
#![allow(dead_code)]

//! Join tree extraction from the Postgres parse tree for AggregateScan.
//!
//! At the `UPPERREL_GROUP_AGG` stage the planner hook receives an `input_rel`
//! that is a `RELOPT_JOINREL`, but the join structure (equi-keys, join type) is
//! not directly available as it was in the `join_pathlist` hook. Instead we walk
//! the parse tree (`root->parse->jointree`) which carries the original `FromExpr` /
//! `JoinExpr` nodes, and reconstruct a [`RelNode`] tree that downstream code can
//! lower into a DataFusion plan.

use crate::nodecast;
use crate::postgres::customscan::joinscan::build::{
    JoinKeyPair, JoinLevelSearchPredicate, JoinNode, JoinSource, JoinSourceCandidate, JoinType,
    PlannerRootId, RelNode,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid, get_rte};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, PgList};

/// Metadata about a table participating in the join, collected during parse-tree walk.
#[derive(Debug)]
pub struct JoinAggSource {
    pub rti: pg_sys::Index,
    pub relid: pg_sys::Oid,
    pub alias: Option<String>,
    pub bm25_index: Option<PgSearchRelation>,
}

/// Collected search predicates from WHERE clause walking.
pub struct CollectedPredicates {
    pub predicates: Vec<JoinLevelSearchPredicate>,
    pub has_any_search_predicate: bool,
}

/// Extract all tables participating in the join from `input_rel.relids` and look up
/// their RTE / BM25 index information.
pub unsafe fn collect_join_agg_sources(
    root: *mut pg_sys::PlannerInfo,
    input_rel: &pg_sys::RelOptInfo,
) -> Vec<JoinAggSource> {
    let mut sources = Vec::new();
    let rtis: Vec<pg_sys::Index> = bms_iter(input_rel.relids).collect();

    for rti in rtis {
        let rte = get_rte(
            (*root).simple_rel_array_size as usize,
            (*root).simple_rte_array,
            rti,
        );
        let Some(rte) = rte else { continue };

        let Some(relid) = get_plain_relation_relid(rte) else {
            continue;
        };

        let alias = if !(*rte).eref.is_null() && !(*(*rte).eref).aliasname.is_null() {
            std::ffi::CStr::from_ptr((*(*rte).eref).aliasname)
                .to_str()
                .ok()
                .map(|s| s.to_string())
        } else {
            None
        };

        let bm25_index = rel_get_bm25_index(relid).map(|(_, idx)| idx);

        sources.push(JoinAggSource {
            rti,
            relid,
            alias,
            bm25_index,
        });
    }

    sources
}

/// Build a [`RelNode`] tree by walking the Postgres parse tree's `jointree`
/// and extracting equi-join keys from the `input_rel`'s cheapest path.
///
/// We use the parse tree for the join structure (FROM items, explicit JOINs)
/// and the planner's path for equi-join keys (since the parser moves quals
/// into restrictinfo lists during planning).
pub unsafe fn extract_join_tree_from_parse(
    root: *mut pg_sys::PlannerInfo,
    sources: &[JoinAggSource],
    input_rel: &pg_sys::RelOptInfo,
) -> Result<RelNode, String> {
    let parse = (*root).parse;
    if parse.is_null() {
        return Err("parse tree is null".into());
    }

    let jointree = (*parse).jointree;
    if jointree.is_null() {
        return Err("jointree is null".into());
    }

    let mut plan = build_relnode_from_fromexpr(root, jointree, sources)?;

    // Extract equi-join keys from the input_rel's cheapest path
    let equi_keys = extract_equi_keys_from_path(input_rel, sources);
    if !equi_keys.is_empty() {
        inject_equi_keys(&mut plan, equi_keys);
    }

    // Fix plan_positions (they default to 0 from JoinSourceCandidate)
    for (position, source) in plan.sources_mut().into_iter().enumerate() {
        source.plan_position = position;
    }

    Ok(plan)
}

/// Walk a `FromExpr` and produce a `RelNode` tree.
///
/// A `FromExpr` contains a `fromlist` (list of tables/joins) and `quals` (WHERE).
/// The WHERE quals are extracted separately — here we only build the join structure.
unsafe fn build_relnode_from_fromexpr(
    root: *mut pg_sys::PlannerInfo,
    from: *mut pg_sys::FromExpr,
    sources: &[JoinAggSource],
) -> Result<RelNode, String> {
    let from_list = PgList::<pg_sys::Node>::from_pg((*from).fromlist);

    if from_list.is_empty() {
        return Err("empty FROM list".into());
    }

    // Build RelNode for first item
    let first_node = from_list
        .get_ptr(0)
        .ok_or_else(|| "failed to get first FROM item".to_string())?;
    let mut result = build_relnode_from_node(root, first_node, sources)?;

    // Additional items are implicit cross/inner joins
    for i in 1..from_list.len() {
        let node = from_list
            .get_ptr(i)
            .ok_or_else(|| format!("failed to get FROM item at index {}", i))?;
        let right = build_relnode_from_node(root, node, sources)?;

        // Implicit join — equi-keys will come from WHERE clause quals
        result = RelNode::Join(Box::new(JoinNode {
            join_type: JoinType::Inner,
            left: result,
            right,
            equi_keys: Vec::new(),
            filter: None,
        }));
    }

    // Extract equi-join keys from WHERE quals and attach to join nodes
    if !(*from).quals.is_null() {
        extract_equi_keys_from_quals(root, (*from).quals, sources, &mut result)?;
    }

    Ok(result)
}

/// Dispatch on a parse-tree node to build the appropriate `RelNode`.
unsafe fn build_relnode_from_node(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
    sources: &[JoinAggSource],
) -> Result<RelNode, String> {
    if node.is_null() {
        return Err("null node in FROM clause".into());
    }

    let tag = (*node).type_;

    if tag == pg_sys::NodeTag::T_RangeTblRef {
        let rtref = node as *mut pg_sys::RangeTblRef;
        let rti = (*rtref).rtindex as pg_sys::Index;
        build_scan_node(root, rti, sources)
    } else if tag == pg_sys::NodeTag::T_JoinExpr {
        let join_expr = node as *mut pg_sys::JoinExpr;
        build_join_node(root, join_expr, sources)
    } else {
        Err(format!("unexpected node type {:?} in join tree", tag))
    }
}

/// Build a `RelNode::Scan` for a single base relation.
unsafe fn build_scan_node(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    sources: &[JoinAggSource],
) -> Result<RelNode, String> {
    let source = sources
        .iter()
        .find(|s| s.rti == rti)
        .ok_or_else(|| format!("RTI {} not found in join sources", rti))?;

    let bm25_index = source.bm25_index.as_ref().ok_or_else(|| {
        format!(
            "table at RTI {} ({}) has no BM25 index",
            rti,
            source.alias.as_deref().unwrap_or("unknown")
        )
    })?;

    let sort_order = if crate::gucs::is_columnar_sort_enabled() {
        bm25_index.options().sort_by().into_iter().next()
    } else {
        None
    };

    // Build a JoinSourceCandidate progressively
    let mut candidate = JoinSourceCandidate::new(PlannerRootId::from(root), rti)
        .with_heaprelid(source.relid)
        .with_indexrelid(bm25_index.oid())
        .with_sort_order(sort_order);

    if let Some(ref alias) = source.alias {
        candidate = candidate.with_alias(alias.clone());
    }

    // Extract search predicates from baserestrictinfo if the planner has them
    let rel_array = (*root).simple_rel_array;
    if !rel_array.is_null() && (rti as isize) < (*root).simple_rel_array_size as isize {
        let rel = *rel_array.offset(rti as isize);
        if !rel.is_null() {
            let baserestrictinfo = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);

            if !baserestrictinfo.is_empty() {
                let context = PlannerContext::from_planner(root);
                for ri in baserestrictinfo.iter_ptr() {
                    let mut state = QualExtractState::default();
                    if let Some(qual) = extract_quals(
                        &context,
                        rti,
                        ri.cast(),
                        crate::postgres::customscan::builders::custom_path::RestrictInfoType::BaseRelation,
                        bm25_index,
                        false,
                        &mut state,
                        true,
                    ) {
                        let query = SearchQueryInput::from(&qual);
                        let current_query = candidate.query.take();
                        let new_query = match current_query {
                            Some(existing) => SearchQueryInput::Boolean {
                                must: vec![existing, query],
                                should: vec![],
                                must_not: vec![],
                            },
                            None => query,
                        };
                        candidate = candidate.with_query(new_query);
                        if state.uses_our_operator {
                            candidate = candidate.with_search_predicate();
                        }
                    }
                }
            }
        }
    }

    candidate.estimate_rows();

    let join_source = JoinSource::try_from(candidate).map_err(|e| e.to_string())?;
    Ok(RelNode::Scan(Box::new(join_source)))
}

/// Build a `RelNode::Join` from a `JoinExpr` parse node.
unsafe fn build_join_node(
    root: *mut pg_sys::PlannerInfo,
    join_expr: *mut pg_sys::JoinExpr,
    sources: &[JoinAggSource],
) -> Result<RelNode, String> {
    let join = &*join_expr;

    let join_type = JoinType::try_from(join.jointype).map_err(|e| e.to_string())?;

    // Support INNER, LEFT/RIGHT, and FULL OUTER JOINs
    match join_type {
        JoinType::Inner | JoinType::Left | JoinType::Right | JoinType::Full => {}
        _ => {
            return Err(format!(
                "aggregate-on-join does not support {} JOIN",
                join_type
            ));
        }
    }

    let left = build_relnode_from_node(root, join.larg, sources)?;
    let right = build_relnode_from_node(root, join.rarg, sources)?;

    // Extract equi-join keys from ON clause (join.quals)
    let equi_keys = if !join.quals.is_null() {
        extract_equi_keys_from_expr(root, join.quals, sources)?
    } else {
        Vec::new()
    };

    Ok(RelNode::Join(Box::new(JoinNode {
        join_type,
        left,
        right,
        equi_keys,
        filter: None,
    })))
}

/// Extract equi-join keys from an expression tree (ON clause or WHERE clause).
///
/// Looks for `OpExpr` nodes where the operator is `=` and the arguments are `Var`
/// nodes referencing different tables that have BM25 indexes.
unsafe fn extract_equi_keys_from_expr(
    _root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
    sources: &[JoinAggSource],
) -> Result<Vec<JoinKeyPair>, String> {
    let mut keys = Vec::new();

    if node.is_null() {
        return Ok(keys);
    }

    let tag = (*node).type_;

    if tag == pg_sys::NodeTag::T_OpExpr {
        if let Some(key) = try_extract_one_equi_key(node as *mut pg_sys::OpExpr, sources) {
            keys.push(key);
        }
    } else if tag == pg_sys::NodeTag::T_BoolExpr {
        let bool_expr = node as *mut pg_sys::BoolExpr;
        // Only recurse into AND expressions — OR'd equi-keys aren't usable
        if (*bool_expr).boolop == pg_sys::BoolExprType::AND_EXPR {
            let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);
            for arg in args.iter_ptr() {
                keys.extend(extract_equi_keys_from_expr(_root, arg, sources)?);
            }
        }
    } else if tag == pg_sys::NodeTag::T_List {
        // Postgres may wrap ON clause quals in a List node
        let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
        for item in list.iter_ptr() {
            keys.extend(extract_equi_keys_from_expr(_root, item, sources)?);
        }
    }

    Ok(keys)
}

/// Try to extract a single equi-join key from an `OpExpr`.
///
/// Returns `Some(JoinKeyPair)` if the expression is `var1 = var2` where
/// `var1` and `var2` reference different tables.
unsafe fn try_extract_one_equi_key(
    op: *mut pg_sys::OpExpr,
    sources: &[JoinAggSource],
) -> Option<JoinKeyPair> {
    let opno = (*op).opno;

    // Check if operator is an equality operator
    // Check if operator is merge-joinable (i.e., an equality operator).
    // Pass InvalidOid to check any input type.
    if !pg_sys::op_mergejoinable(opno, pg_sys::Oid::INVALID) {
        return None;
    }

    let args = PgList::<pg_sys::Node>::from_pg((*op).args);
    if args.len() != 2 {
        return None;
    }

    let left_node = args.get_ptr(0)?;
    let right_node = args.get_ptr(1)?;

    // Both sides must be Var nodes
    let left_var = nodecast!(Var, T_Var, left_node)?;
    let right_var = nodecast!(Var, T_Var, right_node)?;

    let left_rti = (*left_var).varno as pg_sys::Index;
    let right_rti = (*right_var).varno as pg_sys::Index;

    // Must reference different tables
    if left_rti == right_rti {
        return None;
    }

    // Both tables must be in our sources
    let _left_source = sources.iter().find(|s| s.rti == left_rti)?;
    let _right_source = sources.iter().find(|s| s.rti == right_rti)?;

    let left_attno = (*left_var).varattno;
    let right_attno = (*right_var).varattno;

    // Get type info
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(
        (*left_var).vartype,
        &mut typlen as *mut _,
        &mut typbyval as *mut _,
    );

    Some(JoinKeyPair {
        outer_rti: left_rti,
        outer_attno: left_attno,
        inner_rti: right_rti,
        inner_attno: right_attno,
        type_oid: (*left_var).vartype,
        typlen,
        typbyval,
    })
}

/// Walk the WHERE clause quals and attach equi-join keys to the appropriate join nodes.
///
/// For implicit joins (comma-separated FROM), the equi-keys live in the WHERE clause
/// rather than in an ON clause. This function extracts them and pushes them into the
/// `equi_keys` of the topmost `JoinNode`.
unsafe fn extract_equi_keys_from_quals(
    root: *mut pg_sys::PlannerInfo,
    quals: *mut pg_sys::Node,
    sources: &[JoinAggSource],
    plan: &mut RelNode,
) -> Result<(), String> {
    let keys = extract_equi_keys_from_expr(root, quals, sources)?;
    if keys.is_empty() {
        return Ok(());
    }

    // Push equi-keys into the topmost join node
    match plan {
        RelNode::Join(ref mut join_node) => {
            join_node.equi_keys.extend(keys);
        }
        _ => {
            // Single scan — keys from WHERE don't apply to a non-join
        }
    }

    Ok(())
}

/// Extract search predicates (`@@@` operator) from the WHERE clause for each
/// table that has a BM25 index. Returns a list of predicates that can be
/// stored in `JoinCSClause.join_level_predicates`.
pub unsafe fn extract_search_predicates(
    root: *mut pg_sys::PlannerInfo,
    sources: &[JoinAggSource],
) -> CollectedPredicates {
    let mut predicates = Vec::new();
    let mut has_any = false;

    for source in sources {
        let Some(ref bm25_index) = source.bm25_index else {
            continue;
        };

        // Check baserestrictinfo for search predicates
        let rel_array = (*root).simple_rel_array;
        if rel_array.is_null() {
            continue;
        }
        if (source.rti as isize) >= (*root).simple_rel_array_size as isize {
            continue;
        }

        let rel = *rel_array.offset(source.rti as isize);
        if rel.is_null() {
            continue;
        }

        let baserestrictinfo = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
        if baserestrictinfo.is_empty() {
            continue;
        }

        let context = PlannerContext::from_planner(root);
        let mut merged_query: Option<SearchQueryInput> = None;
        let mut source_has_search = false;

        for ri in baserestrictinfo.iter_ptr() {
            let mut state = QualExtractState::default();
            if let Some(qual) = extract_quals(
                &context,
                source.rti,
                ri.cast(),
                crate::postgres::customscan::builders::custom_path::RestrictInfoType::BaseRelation,
                bm25_index,
                false,
                &mut state,
                true,
            ) {
                if state.uses_our_operator {
                    source_has_search = true;
                    has_any = true;
                }

                let query = SearchQueryInput::from(&qual);
                merged_query = Some(match merged_query.take() {
                    Some(existing) => SearchQueryInput::Boolean {
                        must: vec![existing, query],
                        should: vec![],
                        must_not: vec![],
                    },
                    None => query,
                });
            }
        }

        if let Some(query) = merged_query {
            // Deparse the first qualifying RestrictInfo for EXPLAIN display
            let display_string = baserestrictinfo
                .iter_ptr()
                .next()
                .map(|ri| {
                    let expr = (*ri).clause as *mut pg_sys::Node;
                    if !expr.is_null() {
                        let context = PlannerContext::from_planner(root);
                        let heaprel = PgSearchRelation::open(source.relid);
                        deparse_expr(Some(&context), &heaprel, expr)
                    } else {
                        String::new()
                    }
                })
                .unwrap_or_default();

            predicates.push(JoinLevelSearchPredicate {
                rti: source.rti,
                indexrelid: bm25_index.oid(),
                heaprelid: source.relid,
                query,
                display_string,
            });
        }

        // We still track predicates even if not @@@ — they're relevant for the scan
        let _ = source_has_search;
    }

    CollectedPredicates {
        predicates,
        has_any_search_predicate: has_any,
    }
}

/// Extract equi-join keys from the `input_rel`'s cheapest path.
///
/// At `UPPERREL_GROUP_AGG`, `input_rel` is a `RELOPT_JOINREL` whose `pathlist`
/// contains planned join paths. We extract equi-keys from the cheapest path's
/// `joinrestrictinfo` — specifically looking for `OpExpr` clauses with equality
/// operators referencing two different base relations.
unsafe fn extract_equi_keys_from_path(
    input_rel: &pg_sys::RelOptInfo,
    sources: &[JoinAggSource],
) -> Vec<JoinKeyPair> {
    let mut keys = Vec::new();

    // Walk the cheapest_total_path chain looking for JoinPath nodes
    let path = input_rel.cheapest_total_path;
    if path.is_null() {
        return keys;
    }

    extract_keys_from_path_recursive(path, sources, &mut keys);
    keys
}

/// Check the parse tree's WHERE clause for any OR predicates.
/// OR predicates in join aggregate queries are problematic because our
/// per-table scan pushdown can't split OR branches correctly across
/// different PG versions. Reject and let Postgres handle them natively.
pub unsafe fn has_or_in_quals(
    root: *mut pg_sys::PlannerInfo,
    input_rel: &pg_sys::RelOptInfo,
) -> bool {
    let parse = (*root).parse;
    if parse.is_null() {
        return false;
    }
    let jointree = (*parse).jointree;
    if jointree.is_null() {
        return false;
    }
    // Check parse tree WHERE clause (FromExpr.quals)
    if !(*jointree).quals.is_null() && contains_or_expr((*jointree).quals) {
        return true;
    }
    // Check JoinExpr quals in the fromlist (for explicit JOIN ... ON ... WHERE)
    let from_list = PgList::<pg_sys::Node>::from_pg((*jointree).fromlist);
    for node in from_list.iter_ptr() {
        if has_or_in_join_tree(node) {
            return true;
        }
    }
    // Check joinrestrictinfo on cheapest path (PG may move OR there)
    let path = input_rel.cheapest_total_path;
    if !path.is_null() && has_or_in_path_recursive(path) {
        return true;
    }
    false
}

/// Recursively walk a parse tree node (JoinExpr/RangeTblRef) for OR predicates.
unsafe fn has_or_in_join_tree(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }
    if (*node).type_ == pg_sys::NodeTag::T_JoinExpr {
        let join = node as *mut pg_sys::JoinExpr;
        if !(*join).quals.is_null() && contains_or_expr((*join).quals) {
            return true;
        }
        if has_or_in_join_tree((*join).larg) || has_or_in_join_tree((*join).rarg) {
            return true;
        }
    }
    false
}

unsafe fn has_or_in_path_recursive(path: *mut pg_sys::Path) -> bool {
    if path.is_null() {
        return false;
    }
    let tag = (*path).type_;
    if matches!(
        tag,
        pg_sys::NodeTag::T_NestPath | pg_sys::NodeTag::T_MergePath | pg_sys::NodeTag::T_HashPath
    ) {
        let join_path = path as *mut pg_sys::JoinPath;
        let restrict_list = PgList::<pg_sys::RestrictInfo>::from_pg((*join_path).joinrestrictinfo);
        for ri in restrict_list.iter_ptr() {
            if contains_or_expr((*ri).clause as *mut pg_sys::Node) {
                return true;
            }
        }
        if has_or_in_path_recursive((*join_path).outerjoinpath)
            || has_or_in_path_recursive((*join_path).innerjoinpath)
        {
            return true;
        }
    }
    false
}

unsafe fn contains_or_expr(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }
    let tag = (*node).type_;
    match tag {
        pg_sys::NodeTag::T_BoolExpr => {
            let bexpr = node as *mut pg_sys::BoolExpr;
            if (*bexpr).boolop == pg_sys::BoolExprType::OR_EXPR {
                return true;
            }
            let args = PgList::<pg_sys::Node>::from_pg((*bexpr).args);
            for arg in args.iter_ptr() {
                if contains_or_expr(arg) {
                    return true;
                }
            }
            false
        }
        pg_sys::NodeTag::T_List => {
            let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
            for item in list.iter_ptr() {
                if contains_or_expr(item) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Translate a HAVING qual into a serializable `HavingExpr`.
///
/// When the HAVING clause references aggregates not in the SELECT list,
/// they are automatically added to `targetlist.having_aggregates` as hidden
/// aggregates (computed by DataFusion but not projected to Postgres output).
pub unsafe fn translate_having_qual(
    node: *mut pg_sys::Node,
    targetlist: &mut crate::postgres::customscan::aggregatescan::join_targetlist::JoinAggregateTargetList,
    sources: &[JoinAggSource],
) -> Option<crate::postgres::customscan::aggregatescan::privdat::HavingExpr> {
    use crate::postgres::customscan::aggregatescan::join_targetlist::{
        classify_aggregate_by_name, classify_aggregate_oid, JoinAggregateEntry,
    };
    use crate::postgres::customscan::aggregatescan::privdat::{FilterOp, HavingExpr};
    use crate::postgres::var::fieldname_from_var;

    if node.is_null() {
        return None;
    }
    let tag = (*node).type_;
    match tag {
        // Postgres wraps HAVING quals in a List node (even for a single qual).
        // Treat it as an implicit AND of the list elements.
        pg_sys::NodeTag::T_List => {
            let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
            // All children must translate — if any fails, the entire HAVING is rejected
            let children: Option<Vec<_>> = list
                .iter_ptr()
                .map(|child| translate_having_qual(child, targetlist, sources))
                .collect();
            let children = children?;
            if children.is_empty() {
                None
            } else if children.len() == 1 {
                Some(children.into_iter().next().unwrap())
            } else {
                Some(HavingExpr::And(children))
            }
        }
        pg_sys::NodeTag::T_Aggref => {
            let aggref = node as *mut pg_sys::Aggref;
            let aggfnoid = (*aggref).aggfnoid.to_u32();
            let aggstar = (*aggref).aggstar;
            let has_distinct = !(*aggref).aggdistinct.is_null();

            // Extract Var info from the Aggref args for precise matching.
            // This distinguishes SUM(a.x) from SUM(b.x) which have the same func_oid.
            let having_var_info: Option<(pg_sys::Index, pg_sys::AttrNumber)> = if !aggstar {
                let args = PgList::<pg_sys::Node>::from_pg((*aggref).args);
                args.get_ptr(0).and_then(|first_arg| {
                    if (*first_arg).type_ == pg_sys::NodeTag::T_TargetEntry {
                        let te = first_arg as *mut pg_sys::TargetEntry;
                        let expr = (*te).expr as *mut pg_sys::Node;
                        if (*expr).type_ == pg_sys::NodeTag::T_Var {
                            let var = expr as *mut pg_sys::Var;
                            Some(((*var).varno as pg_sys::Index, (*var).varattno))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            // Match helper: checks OID, aggstar, and field reference (RTI + attno)
            let matches_agg = |agg: &JoinAggregateEntry| -> bool {
                if agg.func_oid != aggfnoid {
                    return false;
                }
                let is_count_star = agg.agg_kind
                    == crate::postgres::customscan::aggregatescan::join_targetlist::AggKind::CountStar;
                if aggstar != is_count_star {
                    return false;
                }
                // For non-star aggregates, also compare the field reference
                if !aggstar {
                    match (&agg.field_ref, &having_var_info) {
                        (Some((rti, attno, _)), Some((h_rti, h_attno))) => {
                            *rti == *h_rti && *attno == *h_attno
                        }
                        _ => false,
                    }
                } else {
                    true
                }
            };

            // First check if this aggregate is already in the SELECT list
            for (i, agg) in targetlist.aggregates.iter().enumerate() {
                if matches_agg(agg) {
                    return Some(HavingExpr::AggRef(i));
                }
            }

            // Check if already in having_aggregates
            for (i, agg) in targetlist.having_aggregates.iter().enumerate() {
                if matches_agg(agg) {
                    return Some(HavingExpr::HavingAggRef(i));
                }
            }

            // Not found — classify and add as a hidden HAVING aggregate
            let agg_kind = classify_aggregate_oid(aggfnoid, aggstar, has_distinct)
                .or_else(|| classify_aggregate_by_name(aggfnoid))?;

            // Extract field reference for non-star aggregates
            let field_ref = if !aggstar {
                let args = PgList::<pg_sys::Node>::from_pg((*aggref).args);
                if let Some(first_arg) = args.get_ptr(0) {
                    if (*first_arg).type_ == pg_sys::NodeTag::T_TargetEntry {
                        let te = first_arg as *mut pg_sys::TargetEntry;
                        let expr = (*te).expr as *mut pg_sys::Node;
                        if (*expr).type_ == pg_sys::NodeTag::T_Var {
                            let var = expr as *mut pg_sys::Var;
                            let rti = (*var).varno as pg_sys::Index;
                            let attno = (*var).varattno;
                            // Resolve field name via the source's heaprelid
                            let source = sources.iter().find(|s| s.rti == rti);
                            source.and_then(|s| {
                                fieldname_from_var(s.relid, var, attno)
                                    .map(|name| (rti, attno, name.to_string()))
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // For non-star aggregates, field_ref is required
            if !aggstar && field_ref.is_none() {
                return None;
            }

            let idx = targetlist.having_aggregates.len();
            targetlist.having_aggregates.push(JoinAggregateEntry {
                func_oid: aggfnoid,
                agg_kind,
                field_ref,
                output_index: usize::MAX, // Not projected to output
                result_type_oid: (*aggref).aggtype,
            });

            Some(HavingExpr::HavingAggRef(idx))
        }
        pg_sys::NodeTag::T_Const => {
            let c = node as *mut pg_sys::Const;
            if (*c).constisnull {
                return Some(HavingExpr::LitNull);
            }
            let datum = (*c).constvalue;
            match (*c).consttype {
                pg_sys::INT2OID => Some(HavingExpr::LitInt(
                    pgrx::FromDatum::from_datum(datum, false).unwrap_or(0) as i64,
                )),
                pg_sys::INT4OID => Some(HavingExpr::LitInt(
                    pgrx::FromDatum::from_datum(datum, false).unwrap_or(0i32) as i64,
                )),
                pg_sys::INT8OID => Some(HavingExpr::LitInt(
                    pgrx::FromDatum::from_datum(datum, false).unwrap_or(0i64),
                )),
                pg_sys::FLOAT4OID => Some(HavingExpr::LitFloat(
                    pgrx::FromDatum::from_datum(datum, false).unwrap_or(0.0f32) as f64,
                )),
                pg_sys::FLOAT8OID => Some(HavingExpr::LitFloat(
                    pgrx::FromDatum::from_datum(datum, false).unwrap_or(0.0f64),
                )),
                _ => None,
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            let op = node as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*op).args);
            if args.len() != 2 {
                return None;
            }
            let left = translate_having_qual(args.get_ptr(0)?, targetlist, sources)?;
            let right = translate_having_qual(args.get_ptr(1)?, targetlist, sources)?;
            let opname_ptr = pg_sys::get_opname((*op).opno);
            if opname_ptr.is_null() {
                return None;
            }
            let opname = std::ffi::CStr::from_ptr(opname_ptr).to_str().ok()?;
            let filter_op = match opname {
                "=" => FilterOp::Eq,
                "<>" | "!=" => FilterOp::NotEq,
                "<" => FilterOp::Lt,
                "<=" => FilterOp::LtEq,
                ">" => FilterOp::Gt,
                ">=" => FilterOp::GtEq,
                _ => return None,
            };
            Some(HavingExpr::BinOp {
                left: Box::new(left),
                op: filter_op,
                right: Box::new(right),
            })
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let bexpr = node as *mut pg_sys::BoolExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*bexpr).args);
            // All children must translate — partial translation would weaken the filter
            let children: Option<Vec<_>> = args
                .iter_ptr()
                .map(|a| translate_having_qual(a, targetlist, sources))
                .collect();
            let children = children?;
            if children.is_empty() {
                return None;
            }
            match (*bexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(HavingExpr::And(children)),
                pg_sys::BoolExprType::OR_EXPR => Some(HavingExpr::Or(children)),
                pg_sys::BoolExprType::NOT_EXPR => {
                    if children.len() == 1 {
                        Some(HavingExpr::Not(Box::new(
                            children.into_iter().next().unwrap(),
                        )))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Check whether the join path has any non-equi-join quals (OR across tables,
/// cross-table filters) that our DataFusion backend can't execute.
///
/// Walks the cheapest path's joinrestrictinfo and counts entries that aren't
/// equi-join keys. If any remain, the query has post-join filters we'd lose.
/// Extract non-equi join quals from the cheapest path's joinrestrictinfo.
/// Returns `PostJoinFilter` entries for clauses that aren't equi-join keys.
pub unsafe fn extract_non_equi_join_quals(
    input_rel: &pg_sys::RelOptInfo,
    sources: &[JoinAggSource],
) -> Vec<crate::postgres::customscan::aggregatescan::privdat::PostJoinFilter> {
    let path = input_rel.cheapest_total_path;
    if path.is_null() {
        return Vec::new();
    }
    let mut filters = Vec::new();
    collect_non_equi_quals_recursive(path, sources, &mut filters);
    filters
}

unsafe fn collect_non_equi_quals_recursive(
    path: *mut pg_sys::Path,
    sources: &[JoinAggSource],
    filters: &mut Vec<crate::postgres::customscan::aggregatescan::privdat::PostJoinFilter>,
) {
    if path.is_null() {
        return;
    }
    let tag = (*path).type_;
    let is_join_path = matches!(
        tag,
        pg_sys::NodeTag::T_NestPath | pg_sys::NodeTag::T_MergePath | pg_sys::NodeTag::T_HashPath
    );
    if is_join_path {
        let join_path = path as *mut pg_sys::JoinPath;
        let restrict_list = PgList::<pg_sys::RestrictInfo>::from_pg((*join_path).joinrestrictinfo);
        for ri in restrict_list.iter_ptr() {
            let clause = (*ri).clause as *mut pg_sys::Node;
            if clause.is_null() {
                continue;
            }
            // Skip equi-join keys — they're already handled
            if (*clause).type_ == pg_sys::NodeTag::T_OpExpr
                && try_extract_one_equi_key(clause as *mut pg_sys::OpExpr, sources).is_some()
            {
                continue;
            }
            // Translate the clause to a serializable FilterExpr
            if let Some(expr) = translate_node_to_filter_expr(clause, sources) {
                filters.push(
                    crate::postgres::customscan::aggregatescan::privdat::PostJoinFilter { expr },
                );
            } else {
                // Can't translate — reject the whole path by pushing an untranslatable marker
                filters.push(
                    crate::postgres::customscan::aggregatescan::privdat::PostJoinFilter {
                        expr: crate::postgres::customscan::aggregatescan::privdat::FilterExpr::LitNull,
                    },
                );
            }
        }
        collect_non_equi_quals_recursive((*join_path).outerjoinpath, sources, filters);
        collect_non_equi_quals_recursive((*join_path).innerjoinpath, sources, filters);
    }
}

/// Translate a Postgres expression node into a serializable `FilterExpr`.
/// Returns `None` for expression types we can't translate.
unsafe fn translate_node_to_filter_expr(
    node: *mut pg_sys::Node,
    sources: &[JoinAggSource],
) -> Option<crate::postgres::customscan::aggregatescan::privdat::FilterExpr> {
    use crate::postgres::customscan::aggregatescan::privdat::{FilterExpr, FilterOp};

    if node.is_null() {
        return None;
    }

    let tag = (*node).type_;

    match tag {
        pg_sys::NodeTag::T_Var => {
            let var = node as *mut pg_sys::Var;
            let rti = (*var).varno as pg_sys::Index;
            let attno = (*var).varattno;
            let (idx, source) = sources.iter().enumerate().find(|(_, s)| s.rti == rti)?;
            let name_ptr = pg_sys::get_attname(source.relid, attno, false);
            if name_ptr.is_null() {
                return None;
            }
            let name = std::ffi::CStr::from_ptr(name_ptr).to_str().ok()?.to_owned();
            Some(FilterExpr::Column(idx, name))
        }
        pg_sys::NodeTag::T_Const => {
            let c = node as *mut pg_sys::Const;
            if (*c).constisnull {
                return Some(FilterExpr::LitNull);
            }
            let typoid = (*c).consttype;
            let datum = (*c).constvalue;
            match typoid {
                pg_sys::INT2OID => {
                    let i: Option<i16> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitInt(i? as i64))
                }
                pg_sys::INT4OID => {
                    let i: Option<i32> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitInt(i? as i64))
                }
                pg_sys::INT8OID => {
                    let i: Option<i64> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitInt(i?))
                }
                pg_sys::FLOAT4OID => {
                    let f: Option<f32> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitFloat(f? as f64))
                }
                pg_sys::FLOAT8OID => {
                    let f: Option<f64> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitFloat(f?))
                }
                pg_sys::BOOLOID => {
                    let b: Option<bool> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitBool(b?))
                }
                pg_sys::TEXTOID | pg_sys::VARCHAROID => {
                    let s: Option<String> = pgrx::FromDatum::from_datum(datum, false);
                    Some(FilterExpr::LitString(s?))
                }
                _ => None,
            }
        }
        pg_sys::NodeTag::T_OpExpr => {
            let op = node as *mut pg_sys::OpExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*op).args);
            if args.len() != 2 {
                return None;
            }
            let left = translate_node_to_filter_expr(args.get_ptr(0)?, sources)?;
            let right = translate_node_to_filter_expr(args.get_ptr(1)?, sources)?;

            // Map Postgres operator to FilterOp
            let opname_ptr = pg_sys::get_opname((*op).opno);
            if opname_ptr.is_null() {
                return None;
            }
            let opname = std::ffi::CStr::from_ptr(opname_ptr).to_str().ok()?;
            let filter_op = match opname {
                "=" => FilterOp::Eq,
                "<>" | "!=" => FilterOp::NotEq,
                "<" => FilterOp::Lt,
                "<=" => FilterOp::LtEq,
                ">" => FilterOp::Gt,
                ">=" => FilterOp::GtEq,
                _ => return None,
            };

            Some(FilterExpr::BinOp {
                left: Box::new(left),
                op: filter_op,
                right: Box::new(right),
            })
        }
        pg_sys::NodeTag::T_BoolExpr => {
            let bexpr = node as *mut pg_sys::BoolExpr;
            let args = PgList::<pg_sys::Node>::from_pg((*bexpr).args);
            let mut children = Vec::new();
            for arg in args.iter_ptr() {
                children.push(translate_node_to_filter_expr(arg, sources)?);
            }
            match (*bexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(FilterExpr::And(children)),
                pg_sys::BoolExprType::OR_EXPR => Some(FilterExpr::Or(children)),
                pg_sys::BoolExprType::NOT_EXPR => {
                    if children.len() == 1 {
                        Some(FilterExpr::Not(Box::new(children.into_iter().next()?)))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub unsafe fn has_non_equi_join_quals(
    input_rel: &pg_sys::RelOptInfo,
    sources: &[JoinAggSource],
) -> bool {
    let path = input_rel.cheapest_total_path;
    if path.is_null() {
        return false;
    }
    let mut total_restrict = 0usize;
    let mut equi_keys = 0usize;
    count_restrict_entries_recursive(path, sources, &mut total_restrict, &mut equi_keys);
    total_restrict > equi_keys
}

unsafe fn count_restrict_entries_recursive(
    path: *mut pg_sys::Path,
    sources: &[JoinAggSource],
    total: &mut usize,
    equi: &mut usize,
) {
    if path.is_null() {
        return;
    }
    let tag = (*path).type_;
    let is_join_path = matches!(
        tag,
        pg_sys::NodeTag::T_NestPath | pg_sys::NodeTag::T_MergePath | pg_sys::NodeTag::T_HashPath
    );
    if is_join_path {
        let join_path = path as *mut pg_sys::JoinPath;
        let restrict_list = PgList::<pg_sys::RestrictInfo>::from_pg((*join_path).joinrestrictinfo);
        *total += restrict_list.len();
        for ri in restrict_list.iter_ptr() {
            let clause = (*ri).clause as *mut pg_sys::Node;
            if !clause.is_null()
                && (*clause).type_ == pg_sys::NodeTag::T_OpExpr
                && try_extract_one_equi_key(clause as *mut pg_sys::OpExpr, sources).is_some()
            {
                *equi += 1;
            }
        }
        count_restrict_entries_recursive((*join_path).outerjoinpath, sources, total, equi);
        count_restrict_entries_recursive((*join_path).innerjoinpath, sources, total, equi);
    }
}

/// Recursively walk a path tree extracting equi-join keys from JoinPath nodes.
unsafe fn extract_keys_from_path_recursive(
    path: *mut pg_sys::Path,
    sources: &[JoinAggSource],
    keys: &mut Vec<JoinKeyPair>,
) {
    if path.is_null() {
        return;
    }

    let tag = (*path).type_;

    // Check if this is a join-type path
    let is_join_path = matches!(
        tag,
        pg_sys::NodeTag::T_NestPath | pg_sys::NodeTag::T_MergePath | pg_sys::NodeTag::T_HashPath
    );

    if is_join_path {
        let join_path = path as *mut pg_sys::JoinPath;
        let restrict_list = PgList::<pg_sys::RestrictInfo>::from_pg((*join_path).joinrestrictinfo);

        for ri in restrict_list.iter_ptr() {
            let clause = (*ri).clause as *mut pg_sys::Node;
            if clause.is_null() {
                continue;
            }
            // Look for OpExpr equality clauses
            if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
                if let Some(key) = try_extract_one_equi_key(clause as *mut pg_sys::OpExpr, sources)
                {
                    // Avoid duplicates
                    if !keys.iter().any(|k| {
                        (k.outer_rti == key.outer_rti
                            && k.outer_attno == key.outer_attno
                            && k.inner_rti == key.inner_rti
                            && k.inner_attno == key.inner_attno)
                            || (k.outer_rti == key.inner_rti
                                && k.outer_attno == key.inner_attno
                                && k.inner_rti == key.outer_rti
                                && k.inner_attno == key.outer_attno)
                    }) {
                        keys.push(key);
                    }
                }
            }
        }

        // Recurse into subpaths
        extract_keys_from_path_recursive((*join_path).outerjoinpath, sources, keys);
        extract_keys_from_path_recursive((*join_path).innerjoinpath, sources, keys);
    }

    // If it's a non-join path (e.g., CustomPath wrapping our BaseScan), stop recursing
}

/// Inject extracted equi-join keys into the topmost JoinNode of the plan.
fn inject_equi_keys(plan: &mut RelNode, keys: Vec<JoinKeyPair>) {
    for key in keys {
        inject_single_equi_key(plan, key);
    }
}

/// Inject a single equi-join key into the correct join node in the tree.
///
/// For 3+ table joins, a key pair (outer_rti, inner_rti) must be injected into
/// the join node whose left subtree contains one RTI and right subtree contains
/// the other. Injecting all keys at the top level would cause `resolve_against`
/// to fail because some RTIs are nested inside inner join subtrees.
fn inject_single_equi_key(plan: &mut RelNode, key: JoinKeyPair) {
    match plan {
        RelNode::Join(ref mut join_node) => {
            // Check if this join node can resolve this key
            let left_has_outer = join_node
                .left
                .source_for_rti_in_subtree(key.outer_rti)
                .is_some();
            let right_has_inner = join_node
                .right
                .source_for_rti_in_subtree(key.inner_rti)
                .is_some();
            let left_has_inner = join_node
                .left
                .source_for_rti_in_subtree(key.inner_rti)
                .is_some();
            let right_has_outer = join_node
                .right
                .source_for_rti_in_subtree(key.outer_rti)
                .is_some();

            if (left_has_outer && right_has_inner) || (left_has_inner && right_has_outer) {
                // This key belongs at this join level — check for duplicates
                let already_has = join_node.equi_keys.iter().any(|k| {
                    (k.outer_rti == key.outer_rti
                        && k.inner_rti == key.inner_rti
                        && k.outer_attno == key.outer_attno
                        && k.inner_attno == key.inner_attno)
                        || (k.outer_rti == key.inner_rti
                            && k.inner_rti == key.outer_rti
                            && k.outer_attno == key.inner_attno
                            && k.inner_attno == key.outer_attno)
                });
                if !already_has {
                    join_node.equi_keys.push(key);
                }
            } else {
                // Try injecting into child join nodes
                inject_single_equi_key(&mut join_node.left, key.clone());
                inject_single_equi_key(&mut join_node.right, key);
            }
        }
        RelNode::Filter(ref mut filter) => {
            inject_single_equi_key(&mut filter.input, key);
        }
        RelNode::Scan(_) => {
            // Single scan — no join to inject into
        }
    }
}

/// Validate that at least one table in the join has a BM25 index.
pub fn has_any_bm25_index(sources: &[JoinAggSource]) -> bool {
    sources.iter().any(|s| s.bm25_index.is_some())
}

/// Validate that all tables in the join have a BM25 index.
/// Required because DataFusion needs to scan all tables via `PgSearchTableProvider`.
pub fn all_have_bm25_index(sources: &[JoinAggSource]) -> bool {
    sources.iter().all(|s| s.bm25_index.is_some())
}

/// Populate the `fields` on each `JoinSource` in the `RelNode` tree based on
/// columns referenced in the target list (GROUP BY + aggregate arguments) and
/// join keys. Without this, `PgSearchTableProvider` exposes an empty schema.
pub unsafe fn populate_required_fields(
    plan: &mut RelNode,
    targetlist: &super::join_targetlist::JoinAggregateTargetList,
) -> Result<(), String> {
    use crate::postgres::customscan::pullup::resolve_fast_field;

    let mut sources = plan.sources_mut();

    for source in &mut sources {
        let heaprel = PgSearchRelation::open(source.scan_info.heaprelid);
        let indexrel = PgSearchRelation::open(source.scan_info.indexrelid);
        let tupdesc = heaprel.tuple_desc();

        // Always add Ctid so the provider schema is never empty (needed for COUNT(*))
        use crate::index::fast_fields_helper::WhichFastField;
        source.scan_info.add_field(
            pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
            WhichFastField::Ctid,
        );

        // Add fields referenced in GROUP BY.
        // If any GROUP BY column can't be resolved as a fast field, the
        // DataFusion path cannot execute — return an error to fall back.
        for gc in &targetlist.group_columns {
            if source.contains_rti(gc.rti) {
                match resolve_fast_field(gc.attno as i32, &tupdesc, &indexrel) {
                    Some(field) => source.scan_info.add_field(gc.attno, field),
                    None => {
                        return Err(format!(
                            "GROUP BY column '{}' is not a fast field",
                            gc.field_name
                        ));
                    }
                }
            }
        }

        // Add fields referenced in aggregate arguments.
        // If any aggregate argument can't be resolved as a fast field, the
        // DataFusion path cannot execute — return an error to fall back.
        for agg in &targetlist.aggregates {
            if let Some((rti, attno, ref field_name)) = agg.field_ref {
                if source.contains_rti(rti) {
                    match resolve_fast_field(attno as i32, &tupdesc, &indexrel) {
                        Some(field) => source.scan_info.add_field(attno, field),
                        None => {
                            return Err(format!(
                                "aggregate argument '{}' is not a fast field",
                                field_name
                            ));
                        }
                    }
                }
            }
        }
    }

    // Also add fields for join keys — these MUST be resolvable as fast fields
    // because DataFusion reads them from the BM25 index. If a join key can't be
    // resolved, the PgSearchTableProvider would have no data columns, producing
    // empty RecordBatches that crash execution.
    let join_keys = plan.join_keys();
    let mut sources = plan.sources_mut();
    for jk in &join_keys {
        for source in &mut sources {
            if source.contains_rti(jk.outer_rti) {
                let heaprel = PgSearchRelation::open(source.scan_info.heaprelid);
                let indexrel = PgSearchRelation::open(source.scan_info.indexrelid);
                let tupdesc = heaprel.tuple_desc();
                match resolve_fast_field(jk.outer_attno as i32, &tupdesc, &indexrel) {
                    Some(field) => source.scan_info.add_field(jk.outer_attno, field),
                    None => {
                        return Err(format!(
                            "join key (attno={}) is not a fast field on table {}",
                            jk.outer_attno,
                            source.scan_info.heaprelid.to_u32()
                        ));
                    }
                }
            }
            if source.contains_rti(jk.inner_rti) {
                let heaprel = PgSearchRelation::open(source.scan_info.heaprelid);
                let indexrel = PgSearchRelation::open(source.scan_info.indexrelid);
                let tupdesc = heaprel.tuple_desc();
                match resolve_fast_field(jk.inner_attno as i32, &tupdesc, &indexrel) {
                    Some(field) => source.scan_info.add_field(jk.inner_attno, field),
                    None => {
                        return Err(format!(
                            "join key (attno={}) is not a fast field on table {}",
                            jk.inner_attno,
                            source.scan_info.heaprelid.to_u32()
                        ));
                    }
                }
            }
        }
    }

    // Safety fallback: ensure every source has at least one Named fast field.
    // Without a Named field, PgSearchTableProvider produces empty RecordBatches.
    // This is critical for 3+ table joins where a table might not be directly
    // referenced in GROUP BY, aggregate args, or join keys.
    use crate::index::fast_fields_helper::WhichFastField;
    let mut sources = plan.sources_mut();
    for source in &mut sources {
        let has_named = source.scan_info.fields.iter().any(|f| {
            matches!(
                f.field,
                WhichFastField::Named(_, _) | WhichFastField::Deferred(_, _)
            )
        });
        if !has_named {
            // Find the first fast field from the BM25 index
            let heaprel = PgSearchRelation::open(source.scan_info.heaprelid);
            let indexrel = PgSearchRelation::open(source.scan_info.indexrelid);
            let tupdesc = heaprel.tuple_desc();
            let natts = tupdesc.len();
            for attno in 1..=natts as pg_sys::AttrNumber {
                if let Some(field) = resolve_fast_field(attno as i32, &tupdesc, &indexrel) {
                    if matches!(field, WhichFastField::Named(_, _)) {
                        source.scan_info.add_field(attno, field);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
