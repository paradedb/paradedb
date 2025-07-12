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

use crate::postgres::customscan::pdbscan::condition_safety::{
    ConditionSafety, ConditionSafetyClassifier,
};
use crate::postgres::customscan::pdbscan::join_analysis::{JoinContext, JoinType, RelationInfo};
use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use pgrx::{pg_sys, warning};
use std::collections::{HashMap, HashSet};

/// Result of OR decomposition operation
#[derive(Debug, Clone)]
pub enum DecompositionResult {
    /// Decomposition was successful
    Success(HashMap<pg_sys::Index, Qual>),
    /// Decomposition failed for a specific reason
    Failed(DecompositionError),
    /// Decomposition was partially successful
    Partial(HashMap<pg_sys::Index, Qual>, DecompositionError),
}

/// Errors that can occur during OR decomposition
#[derive(Debug, Clone)]
pub enum DecompositionError {
    /// Join type doesn't support OR decomposition
    UnsafeJoinType(JoinType),
    /// Condition contains cross-table dependencies
    CrossTableDependency,
    /// No conditions found for target relation
    NoConditionsForTarget,
    /// Condition is too complex to decompose
    ComplexCondition,
    /// Missing required index on target relation
    MissingIndex(pg_sys::Index),
}

impl std::fmt::Display for DecompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecompositionError::UnsafeJoinType(join_type) => {
                write!(f, "unsafe join type: {:?}", join_type)
            }
            DecompositionError::CrossTableDependency => {
                write!(f, "cross-table dependency detected")
            }
            DecompositionError::NoConditionsForTarget => {
                write!(f, "no conditions found for target relation")
            }
            DecompositionError::ComplexCondition => {
                write!(f, "condition too complex to decompose")
            }
            DecompositionError::MissingIndex(rti) => {
                write!(f, "missing index on relation {}", rti)
            }
        }
    }
}

/// Statistics about OR decomposition
#[derive(Debug, Clone)]
pub struct DecompositionStats {
    /// Total number of clauses in the original OR condition
    pub total_clauses: usize,
    /// Number of clauses that were safe to decompose
    pub safe_clauses: usize,
    /// Number of clauses that were unsafe
    pub unsafe_clauses: usize,
    /// Number of relations that received conditions
    pub relations_with_conditions: usize,
    /// Details about each relation's conditions
    pub relation_details: HashMap<pg_sys::Index, RelationDecompositionDetails>,
}

/// Details about conditions extracted for a specific relation
#[derive(Debug, Clone)]
pub struct RelationDecompositionDetails {
    /// Number of conditions extracted for this relation
    pub condition_count: usize,
    /// Whether all conditions for this relation were safe
    pub all_safe: bool,
    /// The combined condition for this relation
    pub combined_condition: Option<Qual>,
}

/// Engine for decomposing OR conditions across multiple relations
pub struct OrDecomposer {
    /// Join types that are safe for OR decomposition
    safe_join_types: HashSet<JoinType>,
    /// Mapping from RTI to relation information
    relation_mapping: HashMap<pg_sys::Index, RelationInfo>,
    /// Condition safety classifier
    safety_classifier: ConditionSafetyClassifier,
}

impl OrDecomposer {
    /// Create a new OR decomposer
    pub fn new() -> Self {
        let mut safe_join_types = HashSet::new();
        safe_join_types.insert(JoinType::Inner);
        safe_join_types.insert(JoinType::Cross);

        Self {
            safe_join_types,
            relation_mapping: HashMap::new(),
            safety_classifier: ConditionSafetyClassifier::new(),
        }
    }

    /// Add relation information to the decomposer
    pub fn add_relation(&mut self, rti: pg_sys::Index, relation_info: RelationInfo) {
        self.relation_mapping.insert(rti, relation_info);
    }

    /// Set the join context for safety analysis
    pub fn set_join_context(&mut self, join_context: JoinContext) {
        self.safety_classifier.set_join_context(join_context);
    }

    /// Set the relations that have BM25 indexes
    pub fn set_bm25_relations(&mut self, bm25_relations: Vec<pg_sys::Index>) {
        self.safety_classifier.set_bm25_relations(bm25_relations);
    }

    /// Decompose an OR condition into relation-specific conditions
    pub fn decompose_or_condition(
        &self,
        or_condition: &Qual,
        available_relations: &[pg_sys::Index],
        join_context: &JoinContext,
    ) -> DecompositionResult {
        // Check if the join type is safe for OR decomposition
        if !self.safe_join_types.contains(&join_context.join_type) {
            return DecompositionResult::Failed(DecompositionError::UnsafeJoinType(
                join_context.join_type.clone(),
            ));
        }

        // Extract OR clauses
        let or_clauses = match or_condition {
            Qual::Or(clauses) => clauses,
            _ => return DecompositionResult::Failed(DecompositionError::ComplexCondition),
        };

        // Decompose each clause and group by relation
        let mut relation_conditions: HashMap<pg_sys::Index, Vec<Qual>> = HashMap::new();
        let mut stats = DecompositionStats {
            total_clauses: or_clauses.len(),
            safe_clauses: 0,
            unsafe_clauses: 0,
            relations_with_conditions: 0,
            relation_details: HashMap::new(),
        };

        for clause in or_clauses {
            // Analyze each clause for all available relations
            for &rti in available_relations {
                let safety = self
                    .safety_classifier
                    .classify_condition_safety(clause, rti);

                match safety {
                    ConditionSafety::Safe(conditions) => {
                        stats.safe_clauses += 1;
                        for condition in conditions {
                            if condition.relation_rti == rti {
                                relation_conditions
                                    .entry(rti)
                                    .or_insert_with(Vec::new)
                                    .push(condition.condition);
                            }
                        }
                    }
                    ConditionSafety::PartialSafe(safe_conditions, _) => {
                        for condition in safe_conditions {
                            if condition.relation_rti == rti {
                                relation_conditions
                                    .entry(rti)
                                    .or_insert_with(Vec::new)
                                    .push(condition.condition);
                            }
                        }
                    }
                    ConditionSafety::Unsafe(_) => {
                        // Skip unsafe conditions for this relation
                    }
                }
            }
        }

        // Update statistics
        stats.unsafe_clauses = stats.total_clauses - stats.safe_clauses;
        stats.relations_with_conditions = relation_conditions.len();

        // Create final conditions for each relation
        let mut final_conditions = HashMap::new();
        for (rti, conditions) in relation_conditions {
            let condition_count = conditions.len();
            let combined_condition = match condition_count {
                0 => None,
                1 => Some(conditions.into_iter().next().unwrap()),
                _ => Some(Qual::Or(conditions)),
            };

            if let Some(condition) = combined_condition {
                final_conditions.insert(rti, condition.clone());
                stats.relation_details.insert(
                    rti,
                    RelationDecompositionDetails {
                        condition_count,
                        all_safe: true, // We only include safe conditions
                        combined_condition: Some(condition),
                    },
                );
            }
        }

        // Log the decomposition statistics
        self.log_decomposition_stats(&stats);

        if final_conditions.is_empty() {
            DecompositionResult::Failed(DecompositionError::NoConditionsForTarget)
        } else {
            DecompositionResult::Success(final_conditions)
        }
    }

    /// Decompose an OR condition for a specific target relation
    pub fn decompose_or_condition_for_relation(
        &self,
        or_condition: &Qual,
        target_rti: pg_sys::Index,
        join_context: &JoinContext,
    ) -> Result<Option<Qual>, DecompositionError> {
        // Check if the join type is safe for OR decomposition
        if !self.safe_join_types.contains(&join_context.join_type) {
            return Err(DecompositionError::UnsafeJoinType(
                join_context.join_type.clone(),
            ));
        }

        // Extract OR clauses
        let or_clauses = match or_condition {
            Qual::Or(clauses) => clauses,
            _ => return Err(DecompositionError::ComplexCondition),
        };

        // Extract conditions that belong to the target relation
        let mut target_conditions = Vec::new();

        for clause in or_clauses {
            let safety = self
                .safety_classifier
                .classify_condition_safety(clause, target_rti);

            match safety {
                ConditionSafety::Safe(conditions) => {
                    for condition in conditions {
                        if condition.relation_rti == target_rti {
                            target_conditions.push(condition.condition);
                        }
                    }
                }
                ConditionSafety::PartialSafe(safe_conditions, _) => {
                    for condition in safe_conditions {
                        if condition.relation_rti == target_rti {
                            target_conditions.push(condition.condition);
                        }
                    }
                }
                ConditionSafety::Unsafe(_) => {
                    // Skip unsafe conditions
                }
            }
        }

        // Create the final condition
        let result = match target_conditions.len() {
            0 => None,
            1 => Some(target_conditions.into_iter().next().unwrap()),
            _ => Some(Qual::Or(target_conditions)),
        };

        Ok(result)
    }

    /// Check if a condition can be safely decomposed
    pub fn can_decompose_condition(&self, condition: &Qual, join_context: &JoinContext) -> bool {
        // Check join type safety
        if !self.safe_join_types.contains(&join_context.join_type) {
            return false;
        }

        // Check if it's an OR condition
        match condition {
            Qual::Or(_) => true,
            _ => false,
        }
    }

    /// Extract relation-specific conditions from a complex condition
    pub fn extract_relation_conditions(
        &self,
        condition: &Qual,
        target_rti: pg_sys::Index,
    ) -> Vec<Qual> {
        let mut conditions = Vec::new();

        match condition {
            Qual::Or(clauses) => {
                for clause in clauses {
                    let safety = self
                        .safety_classifier
                        .classify_condition_safety(clause, target_rti);
                    if let ConditionSafety::Safe(safe_conditions) = safety {
                        for safe_condition in safe_conditions {
                            if safe_condition.relation_rti == target_rti {
                                conditions.push(safe_condition.condition);
                            }
                        }
                    }
                }
            }
            Qual::And(clauses) => {
                for clause in clauses {
                    let mut sub_conditions = self.extract_relation_conditions(clause, target_rti);
                    conditions.append(&mut sub_conditions);
                }
            }
            _ => {
                let safety = self
                    .safety_classifier
                    .classify_condition_safety(condition, target_rti);
                if let ConditionSafety::Safe(safe_conditions) = safety {
                    for safe_condition in safe_conditions {
                        if safe_condition.relation_rti == target_rti {
                            conditions.push(safe_condition.condition);
                        }
                    }
                }
            }
        }

        conditions
    }

    /// Log decomposition statistics
    fn log_decomposition_stats(&self, stats: &DecompositionStats) {
        warning!(
            "OR decomposition stats: {} total clauses, {} safe, {} unsafe, {} relations with conditions",
            stats.total_clauses,
            stats.safe_clauses,
            stats.unsafe_clauses,
            stats.relations_with_conditions
        );

        for (rti, details) in &stats.relation_details {
            warning!(
                "Relation {} decomposition: {} conditions, all_safe: {}",
                rti,
                details.condition_count,
                details.all_safe
            );
        }
    }
}

/// Decompose an OR condition into relation-specific parts
pub fn decompose_or_for_relations(
    or_condition: &Qual,
    available_relations: &[pg_sys::Index],
    join_context: &JoinContext,
    bm25_relations: &[pg_sys::Index],
) -> HashMap<pg_sys::Index, Qual> {
    let mut decomposer = OrDecomposer::new();

    // Set up the decomposer with available relations
    for &rti in available_relations {
        let has_bm25 = bm25_relations.contains(&rti);
        decomposer.add_relation(
            rti,
            RelationInfo {
                rti,
                relid: pg_sys::InvalidOid, // Would need to be populated in real usage
                relname: format!("relation_{}", rti),
                has_bm25_index: has_bm25,
            },
        );
    }

    decomposer.set_join_context(join_context.clone());
    decomposer.set_bm25_relations(bm25_relations.to_vec());

    match decomposer.decompose_or_condition(or_condition, available_relations, join_context) {
        DecompositionResult::Success(conditions) => conditions,
        DecompositionResult::Partial(conditions, error) => {
            warning!("Partial OR decomposition: {}", error);
            conditions
        }
        DecompositionResult::Failed(error) => {
            warning!("OR decomposition failed: {}", error);
            HashMap::new()
        }
    }
}

/// Extract conditions for a specific relation from an OR expression
pub fn extract_or_conditions_for_relation(
    or_condition: &Qual,
    target_rti: pg_sys::Index,
    join_context: &JoinContext,
    has_bm25_index: bool,
) -> Option<Qual> {
    let mut decomposer = OrDecomposer::new();

    // Set up the decomposer
    decomposer.add_relation(
        target_rti,
        RelationInfo {
            rti: target_rti,
            relid: pg_sys::InvalidOid,
            relname: format!("relation_{}", target_rti),
            has_bm25_index,
        },
    );

    decomposer.set_join_context(join_context.clone());
    decomposer.set_bm25_relations(if has_bm25_index {
        vec![target_rti]
    } else {
        vec![]
    });

    match decomposer.decompose_or_condition_for_relation(or_condition, target_rti, join_context) {
        Ok(condition) => condition,
        Err(error) => {
            warning!(
                "Failed to extract OR conditions for relation {}: {}",
                target_rti,
                error
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::pdbscan::join_analysis::JoinType;

    #[test]
    fn test_or_decomposer_creation() {
        let decomposer = OrDecomposer::new();
        assert!(decomposer.safe_join_types.contains(&JoinType::Inner));
        assert!(decomposer.safe_join_types.contains(&JoinType::Cross));
        assert!(!decomposer.safe_join_types.contains(&JoinType::LeftOuter));
    }

    #[test]
    fn test_decomposition_error_display() {
        let error = DecompositionError::UnsafeJoinType(JoinType::LeftOuter);
        assert!(error.to_string().contains("unsafe join type"));
    }

    #[test]
    fn test_can_decompose_condition() {
        let mut decomposer = OrDecomposer::new();
        let join_context = JoinContext::new(JoinType::Inner, vec![1, 2]);

        // Test with safe join type
        let or_condition = Qual::Or(vec![Qual::All]);
        assert!(decomposer.can_decompose_condition(&or_condition, &join_context));

        // Test with unsafe join type
        let unsafe_join_context = JoinContext::new(JoinType::LeftOuter, vec![1, 2]);
        assert!(!decomposer.can_decompose_condition(&or_condition, &unsafe_join_context));

        // Test with non-OR condition
        let non_or_condition = Qual::All;
        assert!(!decomposer.can_decompose_condition(&non_or_condition, &join_context));
    }
}
