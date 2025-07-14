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

use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use pgrx::pg_sys;
use std::collections::HashMap;

/// Represents the type of join operation being performed
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JoinType {
    /// INNER JOIN - only matching rows from both tables
    Inner,
    /// CROSS JOIN - cartesian product of both tables
    Cross,
    /// LEFT OUTER JOIN - all rows from left table, matched rows from right
    LeftOuter,
    /// RIGHT OUTER JOIN - all rows from right table, matched rows from left
    RightOuter,
    /// FULL OUTER JOIN - all rows from both tables
    FullOuter,
    /// SEMI JOIN - rows from left table that have matches in right table
    Semi,
    /// ANTI JOIN - rows from left table that have no matches in right table
    Anti,
    /// Unknown or complex join type
    Unknown,
}

impl JoinType {
    /// Returns true if this join type is safe for OR condition decomposition
    pub fn is_safe_for_or_decomposition(&self) -> bool {
        matches!(self, JoinType::Inner | JoinType::Cross)
    }

    /// Returns true if this join type can generate NULL values
    pub fn can_generate_nulls(&self) -> bool {
        matches!(
            self,
            JoinType::LeftOuter | JoinType::RightOuter | JoinType::FullOuter
        )
    }

    /// Returns which relations can be nullable in this join type
    pub fn nullable_relations(
        &self,
        left_rti: pg_sys::Index,
        right_rti: pg_sys::Index,
    ) -> Vec<pg_sys::Index> {
        match self {
            JoinType::LeftOuter => vec![right_rti],
            JoinType::RightOuter => vec![left_rti],
            JoinType::FullOuter => vec![left_rti, right_rti],
            _ => vec![],
        }
    }
}

/// Context information about a join operation
#[derive(Debug, Clone)]
pub struct JoinContext {
    /// The type of join being performed
    pub join_type: JoinType,
    /// Relations participating in the join
    pub participating_relations: Vec<pg_sys::Index>,
    /// Relations that can produce NULL values (for outer joins)
    pub nullable_relations: Vec<pg_sys::Index>,
    /// Join conditions that connect the relations
    pub join_conditions: Vec<Qual>,
    /// Whether this join context is safe for OR decomposition
    pub is_safe: bool,
}

impl JoinContext {
    /// Create a new join context
    pub fn new(join_type: JoinType, participating_relations: Vec<pg_sys::Index>) -> Self {
        let nullable_relations = if participating_relations.len() == 2 {
            join_type.nullable_relations(participating_relations[0], participating_relations[1])
        } else {
            vec![]
        };

        let is_safe = join_type.is_safe_for_or_decomposition();

        Self {
            join_type,
            participating_relations,
            nullable_relations,
            join_conditions: vec![],
            is_safe,
        }
    }

    /// Returns true if OR decomposition is safe for this join context
    pub fn is_safe_for_or_decomposition(&self) -> bool {
        self.is_safe
    }

    /// Returns true if the given relation can be nullable in this join
    pub fn is_relation_nullable(&self, rti: pg_sys::Index) -> bool {
        self.nullable_relations.contains(&rti)
    }

    /// Add a join condition to this context
    pub fn add_join_condition(&mut self, condition: Qual) {
        self.join_conditions.push(condition);
    }
}

/// Information about a specific relation in the query
#[derive(Debug, Clone)]
pub struct RelationInfo {
    /// Range table index of the relation
    pub rti: pg_sys::Index,
    /// OID of the relation
    pub relid: pg_sys::Oid,
    /// Name of the relation (for debugging)
    pub relname: String,
    /// Whether this relation has a BM25 index
    pub has_bm25_index: bool,
}

/// Analyzer for join contexts and relation information
pub struct JoinAnalyzer {
    /// Mapping from RTI to relation information
    relation_info: HashMap<pg_sys::Index, RelationInfo>,
    /// Current join context
    join_context: Option<JoinContext>,
}

impl JoinAnalyzer {
    /// Create a new join analyzer
    pub fn new() -> Self {
        Self {
            relation_info: HashMap::new(),
            join_context: None,
        }
    }

    /// Add relation information
    pub fn add_relation(
        &mut self,
        rti: pg_sys::Index,
        relid: pg_sys::Oid,
        relname: String,
        has_bm25_index: bool,
    ) {
        self.relation_info.insert(
            rti,
            RelationInfo {
                rti,
                relid,
                relname,
                has_bm25_index,
            },
        );
    }

    /// Get relation information for a given RTI
    pub fn get_relation_info(&self, rti: pg_sys::Index) -> Option<&RelationInfo> {
        self.relation_info.get(&rti)
    }

    /// Set the join context
    pub fn set_join_context(&mut self, join_context: JoinContext) {
        self.join_context = Some(join_context);
    }

    /// Get the current join context
    pub fn get_join_context(&self) -> Option<&JoinContext> {
        self.join_context.as_ref()
    }

    /// Analyze the join context from PostgreSQL planner information
    pub unsafe fn analyze_join_context(
        &mut self,
        root: *mut pg_sys::PlannerInfo,
        target_rti: pg_sys::Index,
    ) -> JoinContext {
        // For now, implement a basic analysis
        // In a full implementation, we would analyze the join tree structure
        // from the PlannerInfo to determine the exact join type and participants

        // Get the relation info for analysis
        let rtable = (*(*root).parse).rtable;
        let mut participating_relations = vec![target_rti];

        // Collect all relations that might be involved in joins
        // This is a simplified approach - in practice, we'd need to analyze
        // the join tree structure more carefully
        for i in 1..=pg_sys::list_length(rtable) {
            let rti = i as pg_sys::Index;
            if rti != target_rti {
                let rte = pg_sys::rt_fetch(rti, rtable);
                if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
                    participating_relations.push(rti);
                }
            }
        }

        // For now, assume INNER join for multi-table queries
        // In a full implementation, we would analyze the join tree
        let join_type = if participating_relations.len() > 1 {
            JoinType::Inner
        } else {
            JoinType::Cross
        };

        let join_context = JoinContext::new(join_type, participating_relations);
        self.join_context = Some(join_context.clone());

        join_context
    }

    /// Determine if a condition can be safely decomposed for OR operations
    pub fn can_decompose_or_condition(&self, condition: &Qual) -> bool {
        match &self.join_context {
            Some(ctx) => ctx.is_safe_for_or_decomposition(),
            None => false,
        }
    }

    /// Get all relations that have BM25 indexes
    pub fn get_bm25_relations(&self) -> Vec<pg_sys::Index> {
        self.relation_info
            .iter()
            .filter(|(_, info)| info.has_bm25_index)
            .map(|(rti, _)| *rti)
            .collect()
    }

    /// Check if a relation exists and has a BM25 index
    pub fn relation_has_bm25_index(&self, rti: pg_sys::Index) -> bool {
        self.relation_info
            .get(&rti)
            .map(|info| info.has_bm25_index)
            .unwrap_or(false)
    }
}

/// Analyze join context from PostgreSQL planner information
pub unsafe fn analyze_join_context(
    root: *mut pg_sys::PlannerInfo,
    target_rti: pg_sys::Index,
) -> JoinContext {
    // This is a simplified implementation
    // In practice, we would need to analyze the join tree structure
    // from the PlannerInfo to determine the exact join type and participants

    let rtable = (*(*root).parse).rtable;
    let mut participating_relations = vec![target_rti];

    // Collect all relations that might be involved in joins
    for i in 1..=pg_sys::list_length(rtable) {
        let rti = i as pg_sys::Index;
        if rti != target_rti {
            let rte = pg_sys::rt_fetch(rti, rtable);
            if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
                participating_relations.push(rti);
            }
        }
    }

    // For now, assume INNER join for multi-table queries
    // TODO: Implement proper join tree analysis to determine actual join types
    let join_type = if participating_relations.len() > 1 {
        JoinType::Inner
    } else {
        JoinType::Cross
    };

    JoinContext::new(join_type, participating_relations)
}

/// Extract join type from PostgreSQL join information
pub unsafe fn extract_join_type(
    _root: *mut pg_sys::PlannerInfo,
    _target_rti: pg_sys::Index,
) -> JoinType {
    // This is a placeholder implementation
    // In a full implementation, we would analyze the join tree structure
    // to determine the actual join type (INNER, LEFT OUTER, etc.)

    // For now, default to INNER which is safe for OR decomposition
    JoinType::Inner
}

/// Check if relations are connected by a join
pub unsafe fn relations_are_joined(
    root: *mut pg_sys::PlannerInfo,
    rti1: pg_sys::Index,
    rti2: pg_sys::Index,
) -> bool {
    // Check if two relations are connected by a join condition
    // This is a simplified check - in practice, we'd analyze the join tree

    if rti1 == rti2 {
        return false;
    }

    // Get the relation info for both RTIs
    let rtable = (*(*root).parse).rtable;
    let rte1 = pg_sys::rt_fetch(rti1, rtable);
    let rte2 = pg_sys::rt_fetch(rti2, rtable);

    // Both must be regular relations
    (*rte1).rtekind == pg_sys::RTEKind::RTE_RELATION
        && (*rte2).rtekind == pg_sys::RTEKind::RTE_RELATION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_type_safety() {
        assert!(JoinType::Inner.is_safe_for_or_decomposition());
        assert!(JoinType::Cross.is_safe_for_or_decomposition());
        assert!(!JoinType::LeftOuter.is_safe_for_or_decomposition());
        assert!(!JoinType::RightOuter.is_safe_for_or_decomposition());
        assert!(!JoinType::FullOuter.is_safe_for_or_decomposition());
    }

    #[test]
    fn test_nullable_relations() {
        let left_rti = 1;
        let right_rti = 2;

        assert_eq!(
            JoinType::Inner.nullable_relations(left_rti, right_rti),
            Vec::<pg_sys::Index>::new()
        );
        assert_eq!(
            JoinType::LeftOuter.nullable_relations(left_rti, right_rti),
            vec![right_rti]
        );
        assert_eq!(
            JoinType::RightOuter.nullable_relations(left_rti, right_rti),
            vec![left_rti]
        );
        assert_eq!(
            JoinType::FullOuter.nullable_relations(left_rti, right_rti),
            vec![left_rti, right_rti]
        );
    }

    #[test]
    fn test_join_context() {
        let join_context = JoinContext::new(JoinType::Inner, vec![1, 2]);
        assert!(join_context.is_safe_for_or_decomposition());
        assert!(!join_context.is_relation_nullable(1));
        assert!(!join_context.is_relation_nullable(2));

        let outer_join_context = JoinContext::new(JoinType::LeftOuter, vec![1, 2]);
        assert!(!outer_join_context.is_safe_for_or_decomposition());
        assert!(!outer_join_context.is_relation_nullable(1));
        assert!(outer_join_context.is_relation_nullable(2));
    }
}
