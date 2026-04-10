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

use super::privdat::{CompareOp, FilterExpr};
use crate::api::operator::anyelement_query_input_opoid;
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::joinscan::build::{
    lookup_base_rel_info, try_extract_equi_key, FilterNode, JoinKeyPair, JoinLevelExpr,
    JoinLevelSearchPredicate, JoinNode, JoinSource, JoinSourceCandidate, JoinType,
    MultiTablePredicateInfo, PlannerRootId, RelNode,
};
use crate::postgres::customscan::pullup::{
    get_attno_by_name, resolve_fast_field, resolve_fast_field_by_name,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::bms_iter;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{expr_collect_rtis, expr_collect_vars, expr_contains_any_operator};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, PgList};

/// Result type for `extract_join_tree_from_parse`: the plan tree, search
/// predicates, multi-table predicate info, and raw PG Expr clause pointers.
type JoinTreeResult = (
    RelNode,
    Vec<JoinLevelSearchPredicate>,
    Vec<MultiTablePredicateInfo>,
    Vec<*mut pg_sys::Expr>,
);

/// Result type for `build_search_filter`: the filter expression, search
/// predicates, multi-table predicate info, and raw PG Expr clause pointers.
type SearchFilterResult = (
    JoinLevelExpr,
    Vec<JoinLevelSearchPredicate>,
    Vec<MultiTablePredicateInfo>,
    Vec<*mut pg_sys::Expr>,
);

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
///
/// Delegates per-relation metadata lookup to the shared [`lookup_base_rel_info`]
/// in `joinscan/build.rs`.
pub unsafe fn collect_join_agg_sources(
    root: *mut pg_sys::PlannerInfo,
    input_rel: &pg_sys::RelOptInfo,
) -> Vec<JoinAggSource> {
    let mut sources = Vec::new();
    let rtis: Vec<pg_sys::Index> = bms_iter(input_rel.relids).collect();

    for rti in rtis {
        let Some((relid, alias, bm25_index)) = lookup_base_rel_info(root, rti) else {
            continue;
        };

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
/// [`RelNode::inject_equi_keys`] attaches the keys to the correct join levels.
pub unsafe fn extract_join_tree_from_parse(
    root: *mut pg_sys::PlannerInfo,
    sources: &[JoinAggSource],
    input_rel: &pg_sys::RelOptInfo,
) -> Result<JoinTreeResult, String> {
    let parse = (*root).parse;
    if parse.is_null() {
        return Err("parse tree is null".into());
    }

    let jointree = (*parse).jointree;
    if jointree.is_null() {
        return Err("jointree is null".into());
    }

    let mut plan = build_relnode_from_fromexpr(root, jointree, sources)?;

    // Walk the cheapest path's joinrestrictinfo once to extract both equi-join
    // keys and cross-table @@@ search predicates (mirrors JoinScan's approach).
    let path_info = analyze_join_path_restrictinfo(input_rel, sources);

    if !path_info.equi_keys.is_empty() {
        plan.inject_equi_keys(path_info.equi_keys);
    }

    // Fix plan_positions (they default to 0 from JoinSourceCandidate)
    for (position, source) in plan.sources_mut().into_iter().enumerate() {
        source.plan_position = position;
    }

    // Build a FilterNode from cross-table predicates using JoinScan's
    // transform_to_search_expr for the actual transformation.
    // Handles both @@@ predicates and non-@@@ cross-table predicates
    // (like `b.id > 5`) that reference fast fields.
    //
    // Try path-based extraction first, then fall back to the parse tree's
    // FromExpr.quals. The path walk can miss predicates when the cheapest
    // path isn't a standard join type (e.g. with certain GUC combinations),
    // while the parse tree is always available.
    let mut join_level_predicates = Vec::new();
    let mut multi_table_predicates = Vec::new();
    let mut multi_table_clauses: Vec<*mut pg_sys::Expr> = Vec::new();

    // 1. Path-based extraction
    if !path_info.search_clauses.is_empty() {
        if let Some((filter_expr, predicates, mt_predicates, mt_clauses)) =
            build_search_filter(root, &path_info.search_clauses, sources, &plan)
        {
            join_level_predicates = predicates;
            multi_table_predicates = mt_predicates;
            multi_table_clauses = mt_clauses;
            plan = RelNode::Filter(Box::new(FilterNode {
                input: plan,
                predicate: filter_expr,
            }));
        }
    }

    // 2. Fallback: parse tree quals
    if join_level_predicates.is_empty()
        && multi_table_predicates.is_empty()
        && !(*jointree).quals.is_null()
    {
        let mut parse_clauses = Vec::new();
        collect_cross_table_search_quals((*jointree).quals, &mut parse_clauses);
        if !parse_clauses.is_empty() {
            if let Some((filter_expr, predicates, mt_predicates, mt_clauses)) =
                build_search_filter(root, &parse_clauses, sources, &plan)
            {
                join_level_predicates = predicates;
                multi_table_predicates = mt_predicates;
                multi_table_clauses = mt_clauses;
                plan = RelNode::Filter(Box::new(FilterNode {
                    input: plan,
                    predicate: filter_expr,
                }));
            }
        }
    }

    Ok((
        plan,
        join_level_predicates,
        multi_table_predicates,
        multi_table_clauses,
    ))
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
        // Postgres may wrap ON clause quals in a List node.
        // On PG18 this is the common path even for single-condition ON clauses.
        let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
        for item in list.iter_ptr() {
            keys.extend(extract_equi_keys_from_expr(item, sources)?);
        }
    }

    Ok(keys)
}

/// Try to extract a single equi-join key from an `OpExpr`.
///
/// Delegates to the shared [`try_extract_equi_key`] in `joinscan/build.rs`,
/// scoping the valid RTIs to the tables participating in this aggregate join.
unsafe fn try_extract_one_equi_key(
    op: *mut pg_sys::OpExpr,
    sources: &[JoinAggSource],
) -> Option<JoinKeyPair> {
    let valid_rtis: Vec<pg_sys::Index> = sources.iter().map(|s| s.rti).collect();
    try_extract_equi_key(op, &valid_rtis)
}

/// Walk the WHERE clause quals and attach equi-join keys to the appropriate join nodes.
///
/// For implicit joins (comma-separated FROM), the equi-keys live in the WHERE clause
/// rather than in an ON clause. Each key is distributed to the correct join level
/// using [`RelNode::inject_equi_keys`] so that 3+ table joins work correctly.
unsafe fn extract_equi_keys_from_quals(
    quals: *mut pg_sys::Node,
    sources: &[JoinAggSource],
    plan: &mut RelNode,
) -> Result<(), String> {
    let keys = extract_equi_keys_from_expr(quals, sources)?;
    if keys.is_empty() {
        return Ok(());
    }

    plan.inject_equi_keys(keys);

    Ok(())
}

/// Result of a single walk over the cheapest path's `joinrestrictinfo`.
struct PathRestrictInfo {
    /// Equi-join keys (`a.id = b.id`).
    equi_keys: Vec<JoinKeyPair>,
    /// Cross-table @@@ clause pointers (will be transformed via
    /// `build_search_filter` after plan_positions are assigned).
    search_clauses: Vec<*mut pg_sys::Node>,
    /// Number of RestrictInfo entries we couldn't classify as equi-keys or
    /// @@@ predicates. Non-zero means unhandled quals that would be silently
    /// dropped — the caller should reject the DataFusion path.
    unhandled: usize,
}

/// Walk `input_rel.cheapest_total_path` once, classifying every
/// `joinrestrictinfo` entry as an equi-join key, a cross-table
/// predicate (@@@ or non-@@@), or unhandled.
///
/// For LEFT/RIGHT JOINs, ON-clause predicates (`is_pushed_down=false`)
/// affect matching and NULL-extension semantics — they cannot be
/// correctly applied as post-join filters. These are counted as
/// "unhandled", causing `has_non_equi_join_quals` to reject the
/// DataFusion path for such queries.
unsafe fn analyze_join_path_restrictinfo(
    input_rel: &pg_sys::RelOptInfo,
    sources: &[JoinAggSource],
) -> PathRestrictInfo {
    let mut info = PathRestrictInfo {
        equi_keys: Vec::new(),
        search_clauses: Vec::new(),
        unhandled: 0,
    };
    let path = input_rel.cheapest_total_path;
    if !path.is_null() {
        let search_op = anyelement_query_input_opoid();
        walk_path_restrictinfo(path, sources, search_op, &mut info);
    }
    info
}

unsafe fn walk_path_restrictinfo(
    path: *mut pg_sys::Path,
    sources: &[JoinAggSource],
    search_op: pg_sys::Oid,
    info: &mut PathRestrictInfo,
) {
    if path.is_null() {
        return;
    }
    let tag = (*path).type_;
    let is_join_path = matches!(
        tag,
        pg_sys::NodeTag::T_NestPath | pg_sys::NodeTag::T_MergePath | pg_sys::NodeTag::T_HashPath
    );
    if !is_join_path {
        return;
    }

    let join_path = path as *mut pg_sys::JoinPath;
    let restrict_list = PgList::<pg_sys::RestrictInfo>::from_pg((*join_path).joinrestrictinfo);

    // For outer joins (LEFT/RIGHT), ON-clause predicates affect matching
    // and NULL-extension — they must NOT be applied as post-join filters.
    // PostgreSQL marks ON-clause restrictions with is_pushed_down=false
    // and a non-null outer_relids. We only accept cross-table predicates
    // that are pushed down (WHERE-clause) for non-inner joins.
    let is_inner = (*join_path).jointype == pg_sys::JoinType::JOIN_INNER;

    for ri in restrict_list.iter_ptr() {
        let clause = (*ri).clause as *mut pg_sys::Node;
        if clause.is_null() {
            continue;
        }

        // 1. Equi-join key?
        if (*clause).type_ == pg_sys::NodeTag::T_OpExpr {
            if let Some(key) = try_extract_one_equi_key(clause as *mut pg_sys::OpExpr, sources) {
                let dup = info.equi_keys.iter().any(|k| {
                    (k.outer_rti == key.outer_rti
                        && k.outer_attno == key.outer_attno
                        && k.inner_rti == key.inner_rti
                        && k.inner_attno == key.inner_attno)
                        || (k.outer_rti == key.inner_rti
                            && k.outer_attno == key.inner_attno
                            && k.inner_rti == key.outer_rti
                            && k.inner_attno == key.outer_attno)
                });
                if !dup {
                    info.equi_keys.push(key);
                }
                continue;
            }
        }

        // 2. Cross-table predicate?
        // Covers both @@@ predicates and non-@@@ cross-table predicates
        // (like `b.id > 5`) that reference fast fields and can be translated.
        //
        // For outer joins, reject ON-clause predicates (is_pushed_down=false)
        // since they affect matching semantics, not post-join filtering.
        //
        // Single-table @@@ predicates (rtis.len() == 1) are already handled
        // via baserestrictinfo in build_scan_node — they don't appear here
        // under normal planning. If one does, it's counted as unhandled
        // (conservative: reject the path rather than risk double-applying).
        if !is_inner && !(*ri).is_pushed_down {
            // ON-clause predicate for an outer join — can't apply as post-join
            // filter without changing NULL-extension semantics. Count as unhandled
            // so has_non_equi_join_quals rejects the DataFusion path.
            info.unhandled += 1;
            continue;
        }

        let rtis = expr_collect_rtis(clause);
        if rtis.len() > 1 {
            let has_search = expr_contains_any_operator(clause, &[search_op]);
            let acceptable = if has_search {
                true // build_search_filter will validate the full tree
            } else {
                all_vars_are_fast_fields_for_agg(clause, sources)
            };
            if acceptable {
                if !info.search_clauses.iter().any(|&c| std::ptr::eq(c, clause)) {
                    info.search_clauses.push(clause);
                }
                continue;
            }
        }

        // 3. Unhandled
        info.unhandled += 1;
    }

    walk_path_restrictinfo((*join_path).outerjoinpath, sources, search_op, info);
    walk_path_restrictinfo((*join_path).innerjoinpath, sources, search_op, info);
}

/// Check whether the join path has quals we can't handle.
pub unsafe fn has_non_equi_join_quals(
    input_rel: &pg_sys::RelOptInfo,
    sources: &[JoinAggSource],
) -> bool {
    analyze_join_path_restrictinfo(input_rel, sources).unhandled > 0
}

/// Context for the **build phase** — translating Postgres expression trees
/// into a serializable [`FilterExpr`] IR.
///
/// HAVING provides `targetlist` for resolving `T_Aggref` → `AggRef` and
/// `T_Var` → `GroupRef`. FILTER provides `sources` for resolving
/// `T_Var` → `ColumnRef`.
///
/// This is distinct from the exec-phase context in `datafusion_exec.rs`,
/// which carries a `RelNode` tree instead of raw planner sources.
pub struct FilterExprBuildContext<'a> {
    pub targetlist: Option<&'a super::join_targetlist::JoinAggregateTargetList>,
    pub sources: Option<&'a [JoinAggSource]>,
}

impl FilterExpr {
    /// Translate a Postgres expression node tree into a serializable [`FilterExpr`].
    ///
    /// Used for both HAVING quals (pass `targetlist`) and per-aggregate FILTER
    /// clauses (pass `sources`). The context determines how leaf nodes are resolved.
    pub unsafe fn from_pg_node(
        node: *mut pg_sys::Node,
        ctx: &FilterExprBuildContext<'_>,
    ) -> Option<Self> {
        if node.is_null() {
            return None;
        }

        let tag = (*node).type_;

        match tag {
            pg_sys::NodeTag::T_List => {
                let list = PgList::<pg_sys::Node>::from_pg(node as *mut pg_sys::List);
                let mut children = Vec::new();
                for item in list.iter_ptr() {
                    children.push(Self::from_pg_node(item, ctx)?);
                }
                if children.len() == 1 {
                    return children.into_iter().next();
                }
                Some(Self::And(children))
            }
            pg_sys::NodeTag::T_Aggref => {
                let targetlist = ctx.targetlist?;
                let aggref = node as *mut pg_sys::Aggref;
                for (idx, agg) in targetlist.aggregates.iter().enumerate() {
                    if (*aggref).aggfnoid.to_u32() == agg.func_oid
                        && ((*aggref).aggstar
                            == matches!(agg.agg_kind, super::join_targetlist::AggKind::CountStar))
                    {
                        if (*aggref).aggstar {
                            return Some(Self::AggRef(idx));
                        }
                        if let Some((_, _, ref _field_name)) = agg.field_refs.first() {
                            let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
                            if let Some(first_arg) = args.get_ptr(0) {
                                if let Some(var) = crate::postgres::var::find_one_var(
                                    (*first_arg).expr as *mut pg_sys::Node,
                                ) {
                                    let rti = (*var).varno as pg_sys::Index;
                                    let attno = (*var).varattno;
                                    if let Some((agg_rti, agg_attno, _)) = agg.field_refs.first() {
                                        if rti == *agg_rti && attno == *agg_attno {
                                            return Some(Self::AggRef(idx));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            pg_sys::NodeTag::T_Var => {
                let var = node as *mut pg_sys::Var;
                let rti = (*var).varno as pg_sys::Index;
                let attno = (*var).varattno;

                // FILTER context: resolve to ColumnRef via sources
                if let Some(sources) = ctx.sources {
                    let source = sources.iter().find(|s| s.rti == rti)?;
                    let field_name = fieldname_from_var(source.relid, var, attno)?.into_inner();
                    return Some(Self::ColumnRef { rti, field_name });
                }

                // HAVING context: resolve to GroupRef via targetlist
                if let Some(targetlist) = ctx.targetlist {
                    for gc in &targetlist.group_columns {
                        if gc.rti == rti && gc.attno == attno {
                            return Some(Self::GroupRef(gc.field_name.clone()));
                        }
                    }
                }
                None
            }
            pg_sys::NodeTag::T_Const => {
                let c = node as *mut pg_sys::Const;
                if (*c).constisnull {
                    return None;
                }
                let typoid = (*c).consttype;
                let datum = (*c).constvalue;
                match typoid {
                    pg_sys::INT2OID => {
                        let i: Option<i16> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitInt(i? as i64))
                    }
                    pg_sys::INT4OID => {
                        let i: Option<i32> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitInt(i? as i64))
                    }
                    pg_sys::INT8OID => {
                        let i: Option<i64> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitInt(i?))
                    }
                    pg_sys::FLOAT4OID => {
                        let f: Option<f32> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitFloat(f? as f64))
                    }
                    pg_sys::FLOAT8OID | pg_sys::NUMERICOID => {
                        let f: Option<f64> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitFloat(f?))
                    }
                    pg_sys::BOOLOID => {
                        let b: Option<bool> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitBool(b?))
                    }
                    pg_sys::TEXTOID | pg_sys::VARCHAROID => {
                        let s: Option<String> = pgrx::FromDatum::from_datum(datum, false);
                        Some(Self::LitString(s?))
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
                let left = Self::from_pg_node(args.get_ptr(0)?, ctx)?;
                let right = Self::from_pg_node(args.get_ptr(1)?, ctx)?;

                let opname_ptr = pg_sys::get_opname((*op).opno);
                if opname_ptr.is_null() {
                    return None;
                }
                let opname = std::ffi::CStr::from_ptr(opname_ptr).to_str().ok()?;
                let having_op = match opname {
                    "=" => CompareOp::Eq,
                    "<>" | "!=" => CompareOp::NotEq,
                    "<" => CompareOp::Lt,
                    "<=" => CompareOp::LtEq,
                    ">" => CompareOp::Gt,
                    ">=" => CompareOp::GtEq,
                    _ => return None,
                };

                Some(Self::BinOp {
                    left: Box::new(left),
                    op: having_op,
                    right: Box::new(right),
                })
            }
            pg_sys::NodeTag::T_NullTest => {
                let nt = node as *mut pg_sys::NullTest;
                let arg = Self::from_pg_node((*nt).arg as *mut pg_sys::Node, ctx)?;
                if (*nt).nulltesttype == pg_sys::NullTestType::IS_NULL {
                    Some(Self::IsNull(Box::new(arg)))
                } else {
                    Some(Self::IsNotNull(Box::new(arg)))
                }
            }
            pg_sys::NodeTag::T_BoolExpr => {
                let bexpr = node as *mut pg_sys::BoolExpr;
                let args = PgList::<pg_sys::Node>::from_pg((*bexpr).args);
                let mut children = Vec::new();
                for arg in args.iter_ptr() {
                    children.push(Self::from_pg_node(arg, ctx)?);
                }
                match (*bexpr).boolop {
                    pg_sys::BoolExprType::AND_EXPR => Some(Self::And(children)),
                    pg_sys::BoolExprType::OR_EXPR => Some(Self::Or(children)),
                    pg_sys::BoolExprType::NOT_EXPR => {
                        if children.len() == 1 {
                            Some(Self::Not(Box::new(children.into_iter().next()?)))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
            pg_sys::NodeTag::T_RelabelType => {
                let relabel = node as *mut pg_sys::RelabelType;
                Self::from_pg_node((*relabel).arg as *mut pg_sys::Node, ctx)
            }
            _ => None,
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
    multi_table_clauses: &[*mut pg_sys::Expr],
) -> Result<(), String> {
    let join_keys = plan.join_keys();
    let mut sources = plan.sources_mut();

    // Collect Var references from multi-table predicate clauses so their
    // columns are registered in the PgSearchTableProvider schema.
    let multi_table_vars: Vec<crate::postgres::utils::VarRef> = multi_table_clauses
        .iter()
        .flat_map(|&clause| expr_collect_vars(clause.cast(), false))
        .collect();

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
                if let Some(field) = resolve_fast_field(gc.attno as i32, &tupdesc, indexrel) {
                    source.scan_info.add_field(gc.attno, field);
                } else if let Some(field) = resolve_fast_field_by_name(&gc.field_name, indexrel) {
                    // JSON sub-field (e.g., metadata.category from metadata->>'category').
                    // The attno maps to the parent JSON column, but Tantivy stores
                    // sub-fields as separate fast fields with dotted names.
                    source.scan_info.add_field_by_name(gc.attno, field);
                } else {
                    return Err(format!(
                        "GROUP BY column '{}' (attno={}) is not a fast field on table {}",
                        gc.field_name,
                        gc.attno,
                        source.scan_info.heaprelid.to_u32()
                    ));
                }
            }
        }

        // Add fields referenced in aggregate arguments — same requirement:
        // DataFusion reads these from BM25 fast fields.
        for agg in &targetlist.aggregates {
            for (rti, attno, _) in &agg.field_refs {
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

        // Add fields referenced in aggregate ORDER BY clauses (e.g.,
        // STRING_AGG(col, ',' ORDER BY col2) needs col2 as a fast field).
        for agg in &targetlist.aggregates {
            for ob in &agg.order_by {
                if source.contains_rti(ob.rti) {
                    match resolve_fast_field(ob.attno as i32, &tupdesc, indexrel) {
                        Some(field) => source.scan_info.add_field(ob.attno, field),
                        None => {
                            return Err(format!(
                                "aggregate ORDER BY column '{}' is not a fast field on table {}",
                                ob.field_name,
                                source.scan_info.heaprelid.to_u32()
                            ));
                        }
                    }
                }
            }
        }

        // Add fields referenced in aggregate FILTER clauses.
        for agg in &targetlist.aggregates {
            if let Some(ref filter) = agg.filter {
                for (rti, field_name) in collect_filter_column_refs(filter) {
                    if source.contains_rti(rti) {
                        if let Some(attno) = get_attno_by_name(field_name, &tupdesc) {
                            match resolve_fast_field(attno as i32, &tupdesc, indexrel) {
                                Some(field) => source.scan_info.add_field(attno, field),
                                None => {
                                    return Err(format!(
                                        "FILTER column '{}' is not a fast field on table {}",
                                        field_name,
                                        source.scan_info.heaprelid.to_u32()
                                    ));
                                }
                            }
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

        // Add fields referenced in multi-table predicate clauses — these
        // are cross-table expressions like `b.id > 5` that DataFusion
        // evaluates at join time. Their columns must be in the schema.
        for var_ref in &multi_table_vars {
            if source.contains_rti(var_ref.rti) {
                match resolve_fast_field(var_ref.attno as i32, &tupdesc, indexrel) {
                    Some(field) => source.scan_info.add_field(var_ref.attno, field),
                    None => {
                        return Err(format!(
                            "multi-table predicate column (attno={}) is not a fast field on table {}",
                            var_ref.attno,
                            source.scan_info.heaprelid.to_u32()
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Cross-table @@@ predicate extraction for AggregateScan
// ---------------------------------------------------------------------------

/// Check if all Var references in an expression are fast fields, using
/// `JoinAggSource` metadata (aggregate scan variant of the JoinScan
/// `all_vars_are_fast_fields_recursive`).
unsafe fn all_vars_are_fast_fields_for_agg(
    node: *mut pg_sys::Node,
    sources: &[JoinAggSource],
) -> bool {
    let vars = expr_collect_vars(node, false);

    for var_ref in vars {
        let mut source_found = false;
        for source in sources {
            if source.rti == var_ref.rti {
                let bm25_index = match &source.bm25_index {
                    Some(idx) => idx,
                    None => return false,
                };
                let heaprel = PgSearchRelation::open(source.relid);
                if resolve_fast_field(var_ref.attno as i32, &heaprel.tuple_desc(), bm25_index)
                    .is_none()
                {
                    return false;
                }
                source_found = true;
                break;
            }
        }
        if !source_found {
            return false;
        }
    }

    true
}

/// Transform collected cross-table clause pointers into a `JoinLevelExpr`
/// tree by delegating to JoinScan's [`transform_to_search_expr`] via a
/// temporary [`JoinCSClause`]. After plan_positions have been assigned,
/// `plan.sources()` returns `&[&JoinSource]` — the same type JoinScan uses —
/// so the shared function works directly.
///
/// Returns `(filter_expr, search_predicates, multi_table_predicates, multi_table_clauses)`.
unsafe fn build_search_filter(
    root: *mut pg_sys::PlannerInfo,
    clauses: &[*mut pg_sys::Node],
    _sources: &[JoinAggSource],
    plan: &RelNode,
) -> Option<SearchFilterResult> {
    use crate::postgres::customscan::joinscan::build::JoinCSClause;
    use crate::postgres::customscan::joinscan::predicate::transform_to_search_expr;

    let sources = plan.sources();
    let mut temp_clause = JoinCSClause::new(plan.clone());
    let mut multi_table_clauses: Vec<*mut pg_sys::Expr> = Vec::new();
    let mut expr_trees: Vec<JoinLevelExpr> = Vec::new();

    for &clause in clauses {
        match transform_to_search_expr(
            root,
            clause,
            &sources,
            &mut temp_clause,
            &mut multi_table_clauses,
        ) {
            Some(expr) => expr_trees.push(expr),
            // If any clause can't be fully transformed, bail out.
            // Returning None leaves the clause as "unhandled", which causes
            // has_non_equi_join_quals to reject the DataFusion path.
            None => return None,
        }
    }

    if expr_trees.is_empty() {
        return None;
    }

    let final_expr = if expr_trees.len() == 1 {
        expr_trees.pop().unwrap()
    } else {
        JoinLevelExpr::And(expr_trees)
    };

    Some((
        final_expr,
        temp_clause.join_level_predicates,
        temp_clause.multi_table_predicates,
        multi_table_clauses,
    ))
}

/// Walk a parse-tree expression (typically `FromExpr.quals`) and collect
/// cross-table clause pointers. Flattens top-level AND conjuncts and
/// selects those that reference >1 relation (either @@@ or non-@@@ with fast fields).
unsafe fn collect_cross_table_search_quals(
    node: *mut pg_sys::Node,
    clauses: &mut Vec<*mut pg_sys::Node>,
) {
    if node.is_null() {
        return;
    }
    // Flatten top-level ANDs
    if (*node).type_ == pg_sys::NodeTag::T_BoolExpr {
        let boolexpr = node as *mut pg_sys::BoolExpr;
        if (*boolexpr).boolop == pg_sys::BoolExprType::AND_EXPR {
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            for arg in args.iter_ptr() {
                collect_cross_table_search_quals(arg, clauses);
            }
            return;
        }
    }
    // Keep cross-table conjuncts (both @@@ and non-@@@)
    let rtis = expr_collect_rtis(node);
    if rtis.len() > 1 {
        clauses.push(node);
    }
}

/// Collect all `(rti, field_name)` column references from an [`FilterExpr`] tree.
fn collect_filter_column_refs(expr: &FilterExpr) -> Vec<(pg_sys::Index, &str)> {
    let mut refs = Vec::new();
    match expr {
        FilterExpr::ColumnRef { rti, field_name } => {
            refs.push((*rti, field_name.as_str()));
        }
        FilterExpr::BinOp { left, right, .. } => {
            refs.extend(collect_filter_column_refs(left));
            refs.extend(collect_filter_column_refs(right));
        }
        FilterExpr::And(children) | FilterExpr::Or(children) => {
            for c in children {
                refs.extend(collect_filter_column_refs(c));
            }
        }
        FilterExpr::Not(inner) | FilterExpr::IsNull(inner) | FilterExpr::IsNotNull(inner) => {
            refs.extend(collect_filter_column_refs(inner));
        }
        FilterExpr::AggRef(_)
        | FilterExpr::GroupRef(_)
        | FilterExpr::LitInt(_)
        | FilterExpr::LitFloat(_)
        | FilterExpr::LitBool(_)
        | FilterExpr::LitString(_) => {}
    }
    refs
}
