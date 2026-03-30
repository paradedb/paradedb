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

//! Join tree extraction from the Postgres parse tree for AggregateScan.
//!
//! At the `UPPERREL_GROUP_AGG` stage the planner hook receives an `input_rel`
//! that is a `RELOPT_JOINREL`, but the join structure (equi-keys, join type) is
//! not directly available as it was in the `join_pathlist` hook. Instead we walk
//! the parse tree (`root->parse->jointree`) which carries the original `FromExpr` /
//! `JoinExpr` nodes, and reconstruct a [`RelNode`] tree that downstream code can
//! lower into a DataFusion plan.

use crate::index::fast_fields_helper::WhichFastField;
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::joinscan::build::{
    JoinKeyPair, JoinNode, JoinSource, JoinSourceCandidate, JoinType, PlannerRootId, RelNode,
};
use crate::postgres::customscan::pullup::resolve_fast_field;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid, get_rte};
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

/// Build a [`RelNode`] tree using two complementary sources of information:
///
/// 1. **Parse tree** (`root->parse->jointree`): provides the join **structure** —
///    which tables participate, whether they use explicit `JOIN` syntax or
///    comma-separated `FROM`, and the join type (INNER, LEFT, etc.). This is
///    walked via [`build_relnode_from_fromexpr`] to produce the `RelNode` skeleton.
///
/// 2. **Cheapest path** (`input_rel.cheapest_total_path`): provides the equi-join
///    **keys** (e.g., `a.id = b.id`). By the time we reach `UPPERREL_GROUP_AGG`,
///    the planner has absorbed WHERE-clause quals into `RestrictInfo` lists on
///    the planned `JoinPath` nodes — so `(*from).quals` can be null even for
///    `SELECT ... FROM a, b WHERE a.id = b.id`. We recursively walk the path
///    tree via [`extract_equi_keys_from_path`], inspecting each `JoinPath`'s
///    `joinrestrictinfo` for `OpExpr` nodes with merge-joinable (equality)
///    operators whose two sides reference different base relations.
///
/// The parse tree gives the skeleton, the path gives the keys, and
/// [`inject_equi_keys`] attaches the keys to the topmost `JoinNode`.
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
        extract_equi_keys_from_quals((*from).quals, sources, &mut result)?;
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
                        RestrictInfoType::BaseRelation,
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

    // Support INNER and LEFT/RIGHT JOINs
    match join_type {
        JoinType::Inner | JoinType::Left | JoinType::Right => {}
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
        extract_equi_keys_from_expr(join.quals, sources)?
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
                keys.extend(extract_equi_keys_from_expr(arg, sources)?);
            }
        }
    } else if tag == pg_sys::NodeTag::T_List {
        // Postgres may wrap ON clause quals in a List node
        let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
        for item in list.iter_ptr() {
            keys.extend(extract_equi_keys_from_expr(item, sources)?);
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

    // Both tables must be in our sources (early-return None if not)
    sources.iter().find(|s| s.rti == left_rti)?;
    sources.iter().find(|s| s.rti == right_rti)?;

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
    quals: *mut pg_sys::Node,
    sources: &[JoinAggSource],
    plan: &mut RelNode,
) -> Result<(), String> {
    let keys = extract_equi_keys_from_expr(quals, sources)?;
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

/// Extract equi-join keys from the `input_rel`'s cheapest planned path.
///
/// At `UPPERREL_GROUP_AGG`, `input_rel` is a `RELOPT_JOINREL` whose cheapest
/// path is a tree of `JoinPath` nodes (NestPath / MergePath / HashPath). Each
/// `JoinPath` carries a `joinrestrictinfo` list of `RestrictInfo` nodes — these
/// are the join conditions the planner collected from WHERE and ON clauses.
///
/// We recursively walk this path tree and, for each `RestrictInfo`, check if its
/// clause is an `OpExpr` with a merge-joinable (equality) operator whose two
/// `Var` arguments reference different base relations in our `sources`. If so,
/// we extract a `JoinKeyPair` with the (rti, attno) from each side plus type info.
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

/// Check whether the join path has non-equi-join quals that our DataFusion
/// backend can't execute (e.g., cross-table OR conditions, inequality filters).
///
/// Uses the same path-walking strategy as [`extract_equi_keys_from_path`] but
/// counts total `RestrictInfo` entries vs. those that are equi-join keys. If
/// `total > equi`, the remaining entries are post-join filters we'd silently
/// drop, producing wrong results — so the caller rejects the DataFusion path.
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
    match plan {
        RelNode::Join(ref mut join_node) => {
            join_node.equi_keys.extend(keys);
        }
        RelNode::Filter(ref mut filter) => {
            inject_equi_keys(&mut filter.input, keys);
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
    let join_keys = plan.join_keys();
    let mut sources = plan.sources_mut();

    // Open relations once per source and reuse throughout
    let source_rels: Vec<_> = sources
        .iter()
        .map(|s| {
            let heaprel = PgSearchRelation::open(s.scan_info.heaprelid);
            let indexrel = PgSearchRelation::open(s.scan_info.indexrelid);
            (heaprel, indexrel)
        })
        .collect();

    for (idx, source) in sources.iter_mut().enumerate() {
        let (heaprel, indexrel) = &source_rels[idx];
        let tupdesc = heaprel.tuple_desc();

        // Always add Ctid so the provider schema is never empty (needed for COUNT(*))
        source.scan_info.add_field(
            pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
            WhichFastField::Ctid,
        );

        // Add fields referenced in GROUP BY — must be fast fields so
        // PgSearchTableProvider can expose them as Arrow columns.
        for gc in &targetlist.group_columns {
            if source.contains_rti(gc.rti) {
                match resolve_fast_field(gc.attno as i32, &tupdesc, indexrel) {
                    Some(field) => source.scan_info.add_field(gc.attno, field),
                    None => {
                        return Err(format!(
                            "GROUP BY column (attno={}) is not a fast field on table {}",
                            gc.attno,
                            source.scan_info.heaprelid.to_u32()
                        ));
                    }
                }
            }
        }

        // Add fields referenced in aggregate arguments — same requirement:
        // DataFusion reads these from BM25 fast fields.
        for agg in &targetlist.aggregates {
            if let Some((rti, attno, _)) = &agg.field_ref {
                if source.contains_rti(*rti) {
                    match resolve_fast_field(*attno as i32, &tupdesc, indexrel) {
                        Some(field) => source.scan_info.add_field(*attno, field),
                        None => {
                            return Err(format!(
                                "aggregate argument (attno={}) is not a fast field on table {}",
                                attno,
                                source.scan_info.heaprelid.to_u32()
                            ));
                        }
                    }
                }
            }
        }

        // Add join key fields — these MUST be resolvable as fast fields because
        // DataFusion reads them from the BM25 index. If a join key can't be
        // resolved, the PgSearchTableProvider would have no data columns, producing
        // empty RecordBatches that crash execution.
        for jk in &join_keys {
            for &(rti, attno) in &[
                (jk.outer_rti, jk.outer_attno),
                (jk.inner_rti, jk.inner_attno),
            ] {
                if source.contains_rti(rti) {
                    match resolve_fast_field(attno as i32, &tupdesc, indexrel) {
                        Some(field) => source.scan_info.add_field(attno, field),
                        None => {
                            return Err(format!(
                                "join key (attno={}) is not a fast field on table {}",
                                attno,
                                source.scan_info.heaprelid.to_u32()
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
