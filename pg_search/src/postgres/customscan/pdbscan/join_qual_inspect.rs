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

//! Join qual analysis for extracting search predicates from JOIN conditions
//!
//! This module extends the existing qual inspection logic to handle join conditions
//! where search predicates may span multiple relations.

use crate::api::operator::anyelement_query_input_opoid;
use crate::postgres::customscan::builders::custom_path::RestrictInfoType;
use crate::postgres::customscan::pdbscan::qual_inspect::{extract_quals, Qual};
use crate::postgres::customscan::pdbscan::{bms_iter, get_rel_name};
use crate::postgres::rel_get_bm25_index;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::{pg_sys, warning, PgList, PgRelation};
use serde;

/// Represents search predicates extracted from a join condition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinSearchPredicates {
    /// Search predicates for the outer relation
    pub outer_predicates: Vec<RelationSearchPredicate>,
    /// Search predicates for the inner relation  
    pub inner_predicates: Vec<RelationSearchPredicate>,
    /// Join conditions that connect the relations
    pub join_conditions: Vec<JoinCondition>,
}

/// A search predicate for a specific relation in the join
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelationSearchPredicate {
    /// Range table index of the relation
    pub rti: pg_sys::Index,
    /// Relation OID
    pub relid: pg_sys::Oid,
    /// Relation name for debugging
    pub relname: String,
    /// Extracted search query
    pub query: SearchQueryInput,
    /// Whether this predicate uses the @@@ operator
    pub uses_search_operator: bool,
}

/// A join condition between two relations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinCondition {
    /// Left side relation RTI
    pub left_rti: pg_sys::Index,
    /// Right side relation RTI  
    pub right_rti: pg_sys::Index,
    /// Left side column name
    pub left_column: String,
    /// Right side column name
    pub right_column: String,
    /// Join operator (e.g., equality)
    pub operator: pg_sys::Oid,
    /// Join condition type (e.g., "equality", "other")
    pub condition_type: String,
}

impl JoinSearchPredicates {
    /// Create empty join search predicates
    pub fn empty() -> Self {
        Self {
            outer_predicates: Vec::new(),
            inner_predicates: Vec::new(),
            join_conditions: Vec::new(),
        }
    }

    /// Check if there are any search predicates
    pub fn has_search_predicates(&self) -> bool {
        !self.outer_predicates.is_empty() || !self.inner_predicates.is_empty()
    }

    /// Check if both sides have search predicates
    pub fn has_bilateral_search(&self) -> bool {
        !self.outer_predicates.is_empty() && !self.inner_predicates.is_empty()
    }
}

/// Extract search predicates from join conditions
///
/// This function analyzes the WHERE clause and JOIN conditions to extract
/// search predicates that can be pushed down to each relation in the join.
pub unsafe fn extract_join_search_predicates(
    root: *mut pg_sys::PlannerInfo,
    joinrel: *mut pg_sys::RelOptInfo,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
    extra: *mut pg_sys::JoinPathExtraData,
) -> Option<JoinSearchPredicates> {
    warning!("ParadeDB: Analyzing join search predicates");

    let mut result = JoinSearchPredicates::empty();

    // Get the join clauses from the extra data
    if !extra.is_null() && !(*extra).restrictlist.is_null() {
        let restrictlist = PgList::<pg_sys::RestrictInfo>::from_pg((*extra).restrictlist);
        warning!(
            "ParadeDB: Found {} join restriction clauses",
            restrictlist.len()
        );

        // Analyze each restriction clause
        for restrict_info in restrictlist.iter_ptr() {
            if let Some(predicates) = analyze_join_clause(root, restrict_info, outerrel, innerrel) {
                // Merge predicates into result
                result.outer_predicates.extend(predicates.outer_predicates);
                result.inner_predicates.extend(predicates.inner_predicates);
                result.join_conditions.extend(predicates.join_conditions);
            }
        }
    }

    // Also check for search predicates in the base relations' restrict info
    // These are WHERE clause conditions that apply to individual relations
    result
        .outer_predicates
        .extend(extract_relation_search_predicates(root, outerrel));
    result
        .inner_predicates
        .extend(extract_relation_search_predicates(root, innerrel));

    warning!(
        "ParadeDB: Extracted {} outer predicates, {} inner predicates, {} join conditions",
        result.outer_predicates.len(),
        result.inner_predicates.len(),
        result.join_conditions.len()
    );

    if result.has_search_predicates() {
        Some(result)
    } else {
        None
    }
}

/// Analyze a single join clause to extract search predicates
unsafe fn analyze_join_clause(
    root: *mut pg_sys::PlannerInfo,
    restrict_info: *mut pg_sys::RestrictInfo,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
) -> Option<JoinSearchPredicates> {
    let clause = (*restrict_info).clause;
    warning!(
        "ParadeDB: Analyzing join clause of type {:?}",
        (*clause).type_
    );

    // Extract join conditions from the clause
    if let Some(join_condition) =
        extract_join_condition_from_clause(root, clause, outerrel, innerrel)
    {
        let mut result = JoinSearchPredicates::empty();

        warning!(
            "ParadeDB: Extracted join condition: {}.{} = {}.{}",
            join_condition.left_rti,
            join_condition.left_column,
            join_condition.right_rti,
            join_condition.right_column
        );

        result.join_conditions.push(join_condition);

        Some(result)
    } else {
        None
    }
}

/// Extract join condition from a clause (e.g., ON d.id = f.document_id)
unsafe fn extract_join_condition_from_clause(
    root: *mut pg_sys::PlannerInfo,
    clause: *mut pg_sys::Expr,
    outerrel: *mut pg_sys::RelOptInfo,
    innerrel: *mut pg_sys::RelOptInfo,
) -> Option<JoinCondition> {
    use crate::nodecast;

    // Handle OpExpr (e.g., d.id = f.document_id)
    if let Some(opexpr) = nodecast!(OpExpr, T_OpExpr, clause) {
        let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

        if args.len() == 2 {
            let left_arg = args.get_ptr(0).unwrap();
            let right_arg = args.get_ptr(1).unwrap();

            // Extract variables from both sides
            if let (Some(left_var), Some(right_var)) = (
                extract_var_from_expr(left_arg),
                extract_var_from_expr(right_arg),
            ) {
                // Determine which variable belongs to which relation
                let left_rti = (*left_var).varno as pg_sys::Index;
                let right_rti = (*right_var).varno as pg_sys::Index;

                // Check if this is actually a join condition between our relations
                if is_var_in_reloptinfo(left_rti, outerrel)
                    || is_var_in_reloptinfo(left_rti, innerrel)
                {
                    if is_var_in_reloptinfo(right_rti, outerrel)
                        || is_var_in_reloptinfo(right_rti, innerrel)
                    {
                        // Get column names from the variables
                        let left_column = get_column_name_from_var(root, left_var);
                        let right_column = get_column_name_from_var(root, right_var);

                        // This is a valid join condition between our relations
                        return Some(JoinCondition {
                            left_rti,
                            right_rti,
                            left_column,
                            right_column,
                            operator: (*opexpr).opno,
                            condition_type: "equality".to_string(),
                        });
                    }
                }
            }
        }
    }

    None
}

/// Extract a Var node from an expression, handling RelabelType wrappers
unsafe fn extract_var_from_expr(expr: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    use crate::nodecast;

    // Direct Var
    if let Some(var) = nodecast!(Var, T_Var, expr) {
        return Some(var);
    }

    // Var wrapped in RelabelType
    if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
        if let Some(var) = nodecast!(Var, T_Var, (*relabel).arg) {
            return Some(var);
        }
    }

    None
}

/// Check if a variable's RTI is present in a RelOptInfo
unsafe fn is_var_in_reloptinfo(rti: pg_sys::Index, reloptinfo: *mut pg_sys::RelOptInfo) -> bool {
    if reloptinfo.is_null() {
        return false;
    }

    let relids = (*reloptinfo).relids;
    if relids.is_null() {
        return false;
    }

    pg_sys::bms_is_member(rti as i32, relids)
}

/// Get column name from a Var node
unsafe fn get_column_name_from_var(
    root: *mut pg_sys::PlannerInfo,
    var: *mut pg_sys::Var,
) -> String {
    let rti = (*var).varno as pg_sys::Index;
    let attno = (*var).varattno;

    // Get the RTE for this relation
    let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
    if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
        let relid = (*rte).relid;

        // Open the relation to get column name
        let heaprel = pg_sys::relation_open(relid, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        if !heaprel.is_null() {
            let tuple_desc = pgrx::PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);
            if let Some(attribute) = tuple_desc.get((attno - 1) as usize) {
                let column_name = attribute.name().to_string();
                pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
                return column_name;
            }
            pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as pg_sys::LOCKMODE);
        }
    }

    // Fallback to generic name
    format!("attr_{}", attno)
}

/// Extract search predicates from a single relation's restrict info
unsafe fn extract_relation_search_predicates(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
) -> Vec<RelationSearchPredicate> {
    let mut predicates = Vec::new();

    // Get the base restrict info (WHERE clause conditions for this relation)
    if !(*rel).baserestrictinfo.is_null() {
        let restrictlist = PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo);
        warning!(
            "ParadeDB: Analyzing {} base restriction clauses for relation",
            restrictlist.len()
        );

        // For each relation in this RelOptInfo
        for rti in bms_iter((*rel).relids) {
            if let Some(predicate) = extract_single_relation_predicates(root, rti, &restrictlist) {
                predicates.push(predicate);
            }
        }
    }

    predicates
}

/// Extract search predicates for a single relation
unsafe fn extract_single_relation_predicates(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    restrictlist: &PgList<pg_sys::RestrictInfo>,
) -> Option<RelationSearchPredicate> {
    // Get relation info
    let rte = pg_sys::rt_fetch(rti, (*(*root).parse).rtable);
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION {
        return None;
    }

    let relid = (*rte).relid;
    let relname = get_rel_name(relid);

    // Check if this relation has a BM25 index
    let (table, bm25_index) = rel_get_bm25_index(relid)?;

    warning!(
        "ParadeDB: Extracting predicates for relation {} (rti {})",
        relname,
        rti
    );

    // Get the search index schema
    let indexrel = PgRelation::with_lock(bm25_index.oid(), pg_sys::AccessShareLock as _);
    let directory = crate::index::mvcc::MVCCDirectory::snapshot(bm25_index.oid());
    let index = tantivy::Index::open(directory).ok()?;
    let schema = SearchIndexSchema::open(index.schema(), &indexrel);

    // Extract quals from the restrict list
    let mut uses_our_operator = false;
    let mut all_quals = Vec::new();

    for restrict_info in restrictlist.iter_ptr() {
        if let Some(qual) = extract_quals(
            root,
            rti,
            restrict_info.cast(),
            anyelement_query_input_opoid(),
            RestrictInfoType::BaseRelation,
            &schema,
            &mut uses_our_operator,
        ) {
            all_quals.push(qual);
        }
    }

    if all_quals.is_empty() {
        return None;
    }

    // Combine all quals into a single query
    let combined_qual = if all_quals.len() == 1 {
        all_quals.into_iter().next().unwrap()
    } else {
        Qual::And(all_quals)
    };

    let query = SearchQueryInput::from(&combined_qual);

    warning!(
        "ParadeDB: Extracted search query for {}: uses_search_operator={}",
        relname,
        uses_our_operator
    );

    Some(RelationSearchPredicate {
        rti,
        relid,
        relname,
        query,
        uses_search_operator: uses_our_operator,
    })
}
