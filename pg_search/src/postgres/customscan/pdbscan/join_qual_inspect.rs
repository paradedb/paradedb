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

    // For now, we'll focus on simple cases
    // More complex join analysis will be added in future iterations
    None
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
