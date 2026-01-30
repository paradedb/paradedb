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

use super::build::{ChildProjection, JoinCSClause, JoinKeyPair, JoinSource, ScanInfo};
use super::predicate::find_base_info_recursive;
use super::privdat::{OutputColumnInfo, PrivateData, INNER_SCORE_ALIAS, OUTER_SCORE_ALIAS};
use crate::api::operator::anyelement_query_input_opoid;
use crate::api::{HashMap, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::nodecast;
use crate::postgres::customscan::pullup::resolve_fast_field;
use crate::postgres::customscan::score_funcoids;

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

use crate::postgres::customscan::basescan::projections::score::is_score_func;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::opexpr::{
    initialize_equality_operator_lookup, OperatorAccepts, PostgresOperatorOid, TantivyOperator,
};
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::customscan::range_table::{bms_iter, get_plain_relation_relid};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{expr_collect_vars, expr_contains_any_operator};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
use pgrx::{pg_sys, PgList};
use std::sync::OnceLock;

/// Cache for operator OID lookups.
static OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();
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
    sources: &[JoinSource],
) -> JoinConditions {
    let mut result = JoinConditions {
        equi_keys: Vec::new(),
        other_conditions: Vec::new(),
        has_search_predicate: false,
    };

    if extra.is_null() || sources.len() < 2 {
        return result;
    }

    let outer_side = &sources[0];
    let inner_side = &sources[1];

    let restrictlist = (*extra).restrictlist;
    if restrictlist.is_null() {
        return result;
    }

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

                    // Try to map vars to positions in outer/inner sides
                    if let (Some(outer_attno), Some(inner_attno)) = (
                        outer_side.map_var(varno0, attno0),
                        inner_side.map_var(varno1, attno1),
                    ) {
                        let type_oid = (*var0).vartype;
                        let (typlen, typbyval) = get_type_info(type_oid);
                        result.equi_keys.push(JoinKeyPair {
                            outer_attno,
                            inner_attno,
                            type_oid,
                            typlen,
                            typbyval,
                        });
                        is_equi_join = true;
                    } else if let (Some(outer_attno), Some(inner_attno)) = (
                        outer_side.map_var(varno1, attno1),
                        inner_side.map_var(varno0, attno0),
                    ) {
                        let type_oid = (*var1).vartype;
                        let (typlen, typbyval) = get_type_info(type_oid);
                        result.equi_keys.push(JoinKeyPair {
                            outer_attno,
                            inner_attno,
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

/// Lookup the Tantivy operator string for a given PostgreSQL operator OID.
///
/// Returns `Some("=")` for equality operators, or `None` if the operator is not supported.
fn lookup_operator(opno: pg_sys::Oid) -> Option<&'static str> {
    let lookup = OPERATOR_LOOKUP
        .get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::All) });
    lookup.get(&opno).copied()
}

/// Get type length and pass-by-value info for a given type OID.
pub(super) unsafe fn get_type_info(type_oid: pg_sys::Oid) -> (i16, bool) {
    let mut typlen: i16 = 0;
    let mut typbyval: bool = false;
    pg_sys::get_typlenbyval(type_oid, &mut typlen, &mut typbyval);
    (typlen, typbyval)
}

/// Try to extract join source information from a RelOptInfo.
/// Returns JoinSource if we find a base relation (possibly with a BM25 index) or a supported child join.
pub(super) unsafe fn extract_join_source(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> Option<JoinSource> {
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

        let rtable = (*(*root).parse).rtable;
        if rtable.is_null() {
            return None;
        }

        let rte = pg_sys::rt_fetch(rti, rtable);
        let relid = get_plain_relation_relid(rte)?;

        let mut side_info = ScanInfo::new().with_heap_rti(rti).with_heaprelid(relid);

        if !(*rte).eref.is_null() {
            let eref = (*rte).eref;
            if !(*eref).aliasname.is_null() {
                let alias_cstr = std::ffi::CStr::from_ptr((*eref).aliasname);
                if let Ok(alias) = alias_cstr.to_str() {
                    side_info = side_info.with_alias(alias.to_string());
                }
            }
        }

        if let Some((_, bm25_index)) = rel_get_bm25_index(relid) {
            side_info = side_info.with_indexrelid(bm25_index.oid());

            let baserestrictinfo = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
            if !baserestrictinfo.is_empty() {
                let context = PlannerContext::from_planner(root);
                let mut state = QualExtractState::default();

                if let Some(qual) = extract_quals(
                    &context,
                    rti,
                    baserestrictinfo.as_ptr().cast(),
                    anyelement_query_input_opoid(),
                    crate::postgres::customscan::builders::custom_path::RestrictInfoType::BaseRelation,
                    &bm25_index,
                    false,
                    &mut state,
                    true,
                ) {
                    if state.uses_our_operator {
                        let query = SearchQueryInput::from(&qual);
                        side_info = side_info.with_query(query);
                    }
                }
            }
        }

        return Some(JoinSource::Base(side_info));
    }

    // Case 2: Join Relation (multiple relids)
    let pathlist = PgList::<pg_sys::Path>::from_pg((*rel).pathlist);

    for path in pathlist.iter_ptr() {
        if !path.is_null() && (*path).type_ == pg_sys::NodeTag::T_CustomPath {
            let custom_path = path as *mut pg_sys::CustomPath;
            let methods = (*custom_path).methods;

            // Check if this is a JoinScan
            let name_ptr = (*methods).CustomName;
            if !name_ptr.is_null() {
                let name_cstr = std::ffi::CStr::from_ptr(name_ptr);
                if name_cstr.to_bytes() == b"ParadeDB Join Scan" {
                    // Deserialize PrivateData to get the child clause
                    let private_list =
                        PgList::<pg_sys::Node>::from_pg((*custom_path).custom_private);
                    if !private_list.is_empty() {
                        let private_data = PrivateData::from((*custom_path).custom_private);

                        // Extract projection from path target
                        let pathtarget = (*path).pathtarget;
                        let exprs = PgList::<pg_sys::Node>::from_pg((*pathtarget).exprs);
                        let mut projection = Vec::with_capacity(exprs.len());

                        for expr in exprs.iter_ptr() {
                            if let Some(var) = nodecast!(Var, T_Var, expr) {
                                projection.push(ChildProjection {
                                    rti: (*var).varno as pg_sys::Index,
                                    attno: (*var).varattno,
                                    is_score: false,
                                });
                            } else if let Some(rti) = get_score_func_rti(expr.cast()) {
                                projection.push(ChildProjection {
                                    rti,
                                    attno: 0,
                                    is_score: true,
                                });
                            } else {
                                projection.push(ChildProjection {
                                    rti: 0,
                                    attno: 0,
                                    is_score: false,
                                });
                            }
                        }
                        // Clone clause and add projection
                        let mut child_clause = private_data.join_clause.clone();
                        child_clause.output_projection = Some(projection.clone());

                        return Some(JoinSource::Join(child_clause, projection, None));
                    }
                }
            }
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
///
/// It recursively propagates these requirements down to the base relations.
pub(super) unsafe fn collect_required_fields(
    join_clause: &mut JoinCSClause,
    output_columns: &[OutputColumnInfo],
    custom_exprs: *mut pg_sys::List,
) {
    for source in &mut join_clause.sources {
        ensure_ctid_recursive(source);
    }

    if join_clause.sources.len() >= 2 {
        for jk in &join_clause.join_keys {
            ensure_field_recursive(&mut join_clause.sources[0], jk.outer_attno, None);
            ensure_field_recursive(&mut join_clause.sources[1], jk.inner_attno, None);
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
                        for source in &mut join_clause.sources {
                            if source.contains_rti(info.rti) {
                                ensure_field_recursive(source, info.original_attno, Some(info.rti));
                                break;
                            }
                        }
                    }
                }
            } else {
                for source in &mut join_clause.sources {
                    if source.contains_rti(var.rti) {
                        ensure_column_recursive(source, var.rti, var.attno);
                        break;
                    }
                }
            }
        }
    }

    for info in &join_clause.order_by {
        match &info.feature {
            OrderByFeature::Var { rti, attno, .. } => {
                for source in &mut join_clause.sources {
                    if source.contains_rti(*rti) {
                        ensure_column_recursive(source, *rti, *attno);
                        break;
                    }
                }
            }
            OrderByFeature::Field(name_wrapper) => {
                let name = name_wrapper.as_ref();
                if let Some((alias, col_name)) = name.split_once('.') {
                    let raw_col_name = col_name.trim_matches('"');
                    for source in &mut join_clause.sources {
                        if source.alias().as_deref() == Some(alias) {
                            if let Some(attno) = get_attno_by_name_recursive(source, raw_col_name) {
                                ensure_field_recursive(source, attno, None);
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Recursively collect internal requirements (join keys) for nested joins
    for source in &mut join_clause.sources {
        collect_internal_fields(source);
    }
}

/// Recursively ensure that a specific column is available from a join source.
///
/// If the source is a base relation, it adds the field to the scan requirements.
/// If the source is a nested join, it adds the field to the projection and recurses.
unsafe fn ensure_column_recursive(
    source: &mut JoinSource,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
) {
    match source {
        JoinSource::Base(info) => {
            if info.heap_rti == Some(rti) {
                ensure_field(info, attno);
            }
        }
        JoinSource::Join(clause, projection, _) => {
            // Find if this base column is already projected
            let found = projection.iter().any(|p| p.rti == rti && p.attno == attno);
            if !found {
                // Add to projection
                let proj = ChildProjection {
                    rti,
                    attno,
                    is_score: false,
                };
                projection.push(proj.clone());
                if let Some(op) = &mut clause.output_projection {
                    op.push(proj);
                }
            }

            // Recurse to ensure it is available from child
            for s in &mut clause.sources {
                if s.contains_rti(rti) {
                    ensure_column_recursive(s, rti, attno);
                    break;
                }
            }
        }
    }
}

/// Recursively collect fields required internally by the join structure itself.
///
/// This mainly ensures that:
/// 1. Join keys are available for all join levels.
/// 2. CTID is available for all base relations (always needed).
unsafe fn collect_internal_fields(source: &mut JoinSource) {
    if let JoinSource::Join(clause, _, _) = source {
        // Recurse first
        for s in &mut clause.sources {
            collect_internal_fields(s);
            ensure_ctid_recursive(s);
        }

        // Ensure Join Keys
        if clause.sources.len() >= 2 {
            for jk in &clause.join_keys {
                ensure_field_recursive(&mut clause.sources[0], jk.outer_attno, None);
                ensure_field_recursive(&mut clause.sources[1], jk.inner_attno, None);
            }
        }
    }
}

/// Recursively ensure that CTID is fetched for all base relations in the source tree.
unsafe fn ensure_ctid_recursive(source: &mut JoinSource) {
    match source {
        JoinSource::Base(info) => {
            info.add_field(
                pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber,
                WhichFastField::Ctid,
            );
        }
        JoinSource::Join(clause, _, _) => {
            for s in &mut clause.sources {
                ensure_ctid_recursive(s);
            }
        }
    }
}

/// Recursively ensure a specific field from a base relation is available.
///
/// This traverses down the join tree to find the base relation with the matching RTI,
/// ensures the field is added to its scan requirements, and ensures it is projected
/// up through any intermediate joins.
unsafe fn ensure_field_recursive(
    source: &mut JoinSource,
    attno: pg_sys::AttrNumber,
    rti: Option<pg_sys::Index>,
) {
    match source {
        JoinSource::Base(info) => {
            if let Some(req_rti) = rti {
                if info.heap_rti != Some(req_rti) {
                    return;
                }
            }
            ensure_field(info, attno);
        }
        JoinSource::Join(clause, projection, _) => {
            if attno > 0 && (attno as usize) <= projection.len() {
                let proj = &projection[(attno - 1) as usize];
                if proj.rti > 0 && proj.attno != 0 {
                    for s in &mut clause.sources {
                        if s.contains_rti(proj.rti) {
                            ensure_field_recursive(s, proj.attno, Some(proj.rti));
                            break;
                        }
                    }
                }
            }
        }
    }
}

unsafe fn ensure_field(side: &mut ScanInfo, attno: pg_sys::AttrNumber) {
    if side.fields.iter().any(|f| f.attno == attno) {
        return;
    }

    if let Some(heaprelid) = side.heaprelid {
        if let Some(indexrelid) = side.indexrelid {
            let heaprel = PgSearchRelation::open(heaprelid);
            let indexrel = PgSearchRelation::open(indexrelid);
            let tupdesc = heaprel.tuple_desc();

            if let Some(field) = resolve_fast_field(attno as i32, &tupdesc, &indexrel) {
                side.add_field(attno, field);
                return;
            }
        }
    }

    pgrx::warning!(
        "ensure_field: failed for attno {} in relation {:?}",
        attno,
        side.alias
    );
}

unsafe fn get_attno_by_name(side: &ScanInfo, name: &str) -> Option<pg_sys::AttrNumber> {
    let heaprelid = side.heaprelid?;
    let rel = PgSearchRelation::open(heaprelid);
    let tupdesc = rel.tuple_desc();
    for (i, att) in tupdesc.iter().enumerate() {
        if att.name() == name {
            return Some((i + 1) as pg_sys::AttrNumber);
        }
    }
    None
}

unsafe fn get_attno_by_name_recursive(
    source: &JoinSource,
    name: &str,
) -> Option<pg_sys::AttrNumber> {
    match source {
        JoinSource::Base(info) => get_attno_by_name(info, name),
        JoinSource::Join(clause, _, _) => {
            for s in &clause.sources {
                if let Some(attno) = get_attno_by_name_recursive(s, name) {
                    return Some(attno);
                }
            }
            None
        }
    }
}

pub(super) unsafe fn is_source_column_fast_field(
    source: &JoinSource,
    attno: pg_sys::AttrNumber,
) -> bool {
    use super::predicate::is_column_fast_field;
    match source {
        JoinSource::Base(info) => is_column_fast_field(info, attno),
        JoinSource::Join(clause, projection, _) => {
            if attno > 0 && (attno as usize) <= projection.len() {
                let proj = &projection[(attno - 1) as usize];
                if proj.rti > 0 && proj.attno != 0 {
                    for s in &clause.sources {
                        if s.contains_rti(proj.rti) {
                            return is_source_column_fast_field(s, proj.attno);
                        }
                    }
                }
            }
            false
        }
    }
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
pub(super) unsafe fn order_by_columns_are_fast_fields_recursive(
    root: *mut pg_sys::PlannerInfo,
    outer_side: &JoinSource,
    inner_side: &JoinSource,
) -> bool {
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);
    if pathkeys.is_empty() {
        return true;
    }

    let sources = [outer_side, inner_side];

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
                        if !is_column_fast_field_recursive(source, varno, varattno) {
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

unsafe fn is_column_fast_field_recursive(
    source: &JoinSource,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
) -> bool {
    use super::predicate::is_column_fast_field;
    match source {
        JoinSource::Base(info) => {
            if info.heap_rti == Some(rti) {
                is_column_fast_field(info, attno)
            } else {
                false
            }
        }
        JoinSource::Join(clause, _, _) => {
            for s in &clause.sources {
                if s.contains_rti(rti) {
                    return is_column_fast_field_recursive(s, rti, attno);
                }
            }
            false
        }
    }
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

/// Recursively sets `score_needed` on the ordering base relation and updates intermediate join projections
/// to ensure the score column bubbles up to the top level.
/// Returns the RTI of the ordering base relation if found.
pub(super) fn ensure_score_bubbling(source: &mut JoinSource) -> Option<pg_sys::Index> {
    match source {
        JoinSource::Base(info) => {
            info.score_needed = true;
            info.add_field(0, WhichFastField::Score);
            info.heap_rti
        }
        JoinSource::Join(clause, projection, _) => {
            let rti = if clause.ordering_side_is_outer() {
                ensure_score_bubbling(&mut clause.sources[0])
            } else {
                ensure_score_bubbling(&mut clause.sources[1])
            }?;

            let found = projection.iter().any(|p| p.rti == rti && p.is_score);
            if !found {
                let proj = ChildProjection {
                    rti,
                    attno: 0,
                    is_score: true,
                };
                projection.push(proj.clone());
                if let Some(op) = &mut clause.output_projection {
                    op.push(proj);
                }
            }
            Some(rti)
        }
    }
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
    sources: &[JoinSource],
    ordering_side_is_outer: bool,
) -> Vec<OrderByInfo> {
    let mut result = Vec::new();
    let pathkeys = PgList::<pg_sys::PathKey>::from_pg((*root).query_pathkeys);

    if pathkeys.is_empty() || sources.is_empty() {
        return result;
    }

    let outer_side = &sources[0];
    let inner_side = if sources.len() > 1 {
        Some(&sources[1])
    } else {
        None
    };

    let outer_alias = outer_side.alias();
    let inner_alias = inner_side.and_then(|s| s.alias());

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

            if is_score_func_recursive(check_expr.cast(), outer_side) {
                if ordering_side_is_outer {
                    result.push(OrderByInfo {
                        feature: OrderByFeature::Score,
                        direction,
                    });
                } else {
                    let alias = outer_alias.as_deref().unwrap_or("outer");
                    result.push(OrderByInfo {
                        feature: OrderByFeature::Field(
                            format!("{}.{}", alias, OUTER_SCORE_ALIAS).into(),
                        ),
                        direction,
                    });
                }
                break;
            }
            if let Some(inner) = inner_side {
                if is_score_func_recursive(check_expr.cast(), inner) {
                    if !ordering_side_is_outer {
                        result.push(OrderByInfo {
                            feature: OrderByFeature::Score,
                            direction,
                        });
                    } else {
                        let alias = inner_alias.as_deref().unwrap_or("inner");
                        result.push(OrderByInfo {
                            feature: OrderByFeature::Field(
                                format!("{}.{}", alias, INNER_SCORE_ALIAS).into(),
                            ),
                            direction,
                        });
                    }
                    break;
                }
            }

            if let Some(var) = nodecast!(Var, T_Var, expr) {
                let varno = (*var).varno as pg_sys::Index;
                let varattno = (*var).varattno;

                for source in sources {
                    if source.contains_rti(varno) {
                        // Try to find a display name (optional)
                        let name = find_base_info_recursive(source, varno).and_then(|info| {
                            info.heaprelid.and_then(|relid| {
                                fieldname_from_var(relid, var, varattno).map(|f| f.to_string())
                            })
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
