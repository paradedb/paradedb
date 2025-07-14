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

use crate::postgres::customscan::pdbscan::join_analysis::{JoinContext, JoinType};
use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use pgrx::{pg_sys, warning};

/// Represents the safety classification of a condition for pushdown
#[derive(Debug, Clone)]
pub enum ConditionSafety {
    /// The condition is completely safe and can be pushed down
    Safe(Vec<RelationSpecificCondition>),
    /// The condition is completely unsafe and cannot be pushed down
    Unsafe(UnsafetyReason),
    /// The condition is partially safe - some parts can be pushed down
    PartialSafe(Vec<RelationSpecificCondition>, Vec<UnsafeCondition>),
}

/// Represents a condition that is specific to a particular relation
#[derive(Debug, Clone)]
pub struct RelationSpecificCondition {
    /// The relation this condition applies to
    pub relation_rti: pg_sys::Index,
    /// The condition itself
    pub condition: Qual,
    /// The original context where this condition was found
    pub original_context: ConditionContext,
}

/// Represents a condition that is unsafe to push down
#[derive(Debug, Clone)]
pub struct UnsafeCondition {
    /// The unsafe condition
    pub condition: Qual,
    /// The reason why it's unsafe
    pub reason: UnsafetyReason,
}

/// Represents the context in which a condition was found
#[derive(Debug, Clone)]
pub enum ConditionContext {
    /// Found in a WHERE clause
    Where,
    /// Found in a JOIN condition
    Join,
    /// Found in a HAVING clause
    Having,
    /// Found in a subquery
    Subquery,
    /// Unknown context
    Unknown,
}

/// Reasons why a condition might be unsafe to push down
#[derive(Debug, Clone)]
pub enum UnsafetyReason {
    /// The condition references variables from multiple tables
    CrossTableDependency,
    /// The condition is in an outer join context that doesn't support pushdown
    OuterJoinSemantics,
    /// The condition involves a correlated subquery
    CorrelatedSubquery,
    /// The condition contains an unknown expression type
    UnknownExpression,
    /// The condition references a relation that doesn't have the required index
    MissingIndex,
    /// The condition would change the semantics of the query if pushed down
    SemanticViolation,
}

impl std::fmt::Display for UnsafetyReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnsafetyReason::CrossTableDependency => write!(f, "cross-table dependency"),
            UnsafetyReason::OuterJoinSemantics => write!(f, "outer join semantics"),
            UnsafetyReason::CorrelatedSubquery => write!(f, "correlated subquery"),
            UnsafetyReason::UnknownExpression => write!(f, "unknown expression"),
            UnsafetyReason::MissingIndex => write!(f, "missing index"),
            UnsafetyReason::SemanticViolation => write!(f, "semantic violation"),
        }
    }
}

/// Detailed analysis of a condition for diagnostic purposes
#[derive(Debug, Clone)]
pub enum ConditionAnalysis {
    /// OR decomposition was successful
    SafeOrDecomposition {
        total_clauses: usize,
        safe_clauses: usize,
        target_clauses: usize,
    },
    /// OR decomposition was rejected due to outer join
    UnsafeOuterJoin { join_type: JoinType, reason: String },
    /// OR decomposition was rejected due to cross-table dependencies
    CrossTableDependency { dependencies: Vec<String> },
    /// Condition partially accepted with some parts rejected
    PartialAcceptance {
        safe_parts: usize,
        unsafe_parts: usize,
        details: String,
    },
    /// Condition completely rejected
    CompleteRejection {
        reason: UnsafetyReason,
        details: String,
    },
}

/// Classifier for condition safety analysis
pub struct ConditionSafetyClassifier {
    /// Current join context
    join_context: Option<JoinContext>,
    /// Relations that have BM25 indexes
    bm25_relations: Vec<pg_sys::Index>,
}

impl ConditionSafetyClassifier {
    /// Create a new condition safety classifier
    pub fn new() -> Self {
        Self {
            join_context: None,
            bm25_relations: vec![],
        }
    }

    /// Set the join context
    pub fn set_join_context(&mut self, join_context: JoinContext) {
        self.join_context = Some(join_context);
    }

    /// Set the relations that have BM25 indexes
    pub fn set_bm25_relations(&mut self, bm25_relations: Vec<pg_sys::Index>) {
        self.bm25_relations = bm25_relations;
    }

    /// Classify the safety of a condition
    pub fn classify_condition_safety(
        &self,
        condition: &Qual,
        target_rti: pg_sys::Index,
    ) -> ConditionSafety {
        match condition {
            Qual::Or(clauses) => self.classify_or_condition(clauses, target_rti),
            Qual::And(clauses) => self.classify_and_condition(clauses, target_rti),
            Qual::Not(inner_condition) => self.classify_not_condition(inner_condition, target_rti),
            Qual::ExternalVar | Qual::ExternalExpr => {
                ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
            }
            _ => self.classify_simple_condition(condition, target_rti),
        }
    }

    /// Classify an OR condition
    fn classify_or_condition(
        &self,
        clauses: &[Qual],
        target_rti: pg_sys::Index,
    ) -> ConditionSafety {
        // Check if the join context supports OR decomposition
        if let Some(ref join_context) = self.join_context {
            if !join_context.is_safe_for_or_decomposition() {
                return ConditionSafety::Unsafe(UnsafetyReason::OuterJoinSemantics);
            }
        }

        let mut safe_conditions = Vec::new();
        let mut unsafe_conditions = Vec::new();

        for clause in clauses {
            match self.classify_condition_safety(clause, target_rti) {
                ConditionSafety::Safe(mut conditions) => {
                    safe_conditions.append(&mut conditions);
                }
                ConditionSafety::Unsafe(reason) => {
                    unsafe_conditions.push(UnsafeCondition {
                        condition: clause.clone(),
                        reason,
                    });
                }
                ConditionSafety::PartialSafe(mut safe_parts, mut unsafe_parts) => {
                    safe_conditions.append(&mut safe_parts);
                    unsafe_conditions.append(&mut unsafe_parts);
                }
            }
        }

        // For OR conditions, we need ALL clauses to be safe or safely decomposable
        if unsafe_conditions.is_empty() {
            ConditionSafety::Safe(safe_conditions)
        } else if safe_conditions.is_empty() {
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        } else {
            // For OR conditions, we can only push down if we can decompose
            // the OR into relation-specific parts
            self.attempt_or_decomposition(clauses, target_rti)
        }
    }

    /// Classify an AND condition
    fn classify_and_condition(
        &self,
        clauses: &[Qual],
        target_rti: pg_sys::Index,
    ) -> ConditionSafety {
        let mut safe_conditions = Vec::new();
        let mut unsafe_conditions = Vec::new();

        for clause in clauses {
            match self.classify_condition_safety(clause, target_rti) {
                ConditionSafety::Safe(mut conditions) => {
                    safe_conditions.append(&mut conditions);
                }
                ConditionSafety::Unsafe(reason) => {
                    unsafe_conditions.push(UnsafeCondition {
                        condition: clause.clone(),
                        reason,
                    });
                }
                ConditionSafety::PartialSafe(mut safe_parts, mut unsafe_parts) => {
                    safe_conditions.append(&mut safe_parts);
                    unsafe_conditions.append(&mut unsafe_parts);
                }
            }
        }

        // For AND conditions, we can safely push down the safe parts
        if safe_conditions.is_empty() {
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        } else if unsafe_conditions.is_empty() {
            ConditionSafety::Safe(safe_conditions)
        } else {
            ConditionSafety::PartialSafe(safe_conditions, unsafe_conditions)
        }
    }

    /// Classify a NOT condition
    fn classify_not_condition(
        &self,
        inner_condition: &Qual,
        target_rti: pg_sys::Index,
    ) -> ConditionSafety {
        // NOT conditions have the same safety as their inner condition
        match self.classify_condition_safety(inner_condition, target_rti) {
            ConditionSafety::Safe(conditions) => {
                // Wrap each condition in a NOT
                let not_conditions = conditions
                    .into_iter()
                    .map(|mut cond| {
                        cond.condition = Qual::Not(Box::new(cond.condition));
                        cond
                    })
                    .collect();
                ConditionSafety::Safe(not_conditions)
            }
            other => other,
        }
    }

    /// Classify a simple condition (not AND/OR/NOT)
    fn classify_simple_condition(
        &self,
        condition: &Qual,
        target_rti: pg_sys::Index,
    ) -> ConditionSafety {
        // Check if this condition references the target relation
        if self.condition_references_relation(condition, target_rti) {
            // Check if the target relation has a BM25 index
            if self.bm25_relations.contains(&target_rti) {
                ConditionSafety::Safe(vec![RelationSpecificCondition {
                    relation_rti: target_rti,
                    condition: condition.clone(),
                    original_context: ConditionContext::Unknown,
                }])
            } else {
                ConditionSafety::Unsafe(UnsafetyReason::MissingIndex)
            }
        } else {
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        }
    }

    /// Attempt to decompose an OR condition into relation-specific parts
    fn attempt_or_decomposition(
        &self,
        clauses: &[Qual],
        target_rti: pg_sys::Index,
    ) -> ConditionSafety {
        // For OR decomposition, we need to extract conditions that belong to the target relation
        let mut target_conditions = Vec::new();

        for clause in clauses {
            if self.condition_references_relation(clause, target_rti)
                && !clause.contains_external_var()
            {
                target_conditions.push(RelationSpecificCondition {
                    relation_rti: target_rti,
                    condition: clause.clone(),
                    original_context: ConditionContext::Where,
                });
            }
        }

        if target_conditions.is_empty() {
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        } else {
            // We can push down the OR of conditions that reference this relation
            ConditionSafety::Safe(vec![RelationSpecificCondition {
                relation_rti: target_rti,
                condition: Qual::Or(
                    target_conditions
                        .iter()
                        .map(|c| c.condition.clone())
                        .collect(),
                ),
                original_context: ConditionContext::Where,
            }])
        }
    }

    /// Check if a condition references a specific relation
    fn condition_references_relation(&self, condition: &Qual, target_rti: pg_sys::Index) -> bool {
        match condition {
            Qual::OpExpr { .. } => {
                // For OpExpr, we need to check if it references the target relation
                // This is a simplified check - in practice, we'd need to analyze the expression
                !condition.contains_external_var()
            }
            Qual::ExternalVar | Qual::ExternalExpr => false,
            Qual::And(clauses) | Qual::Or(clauses) => clauses
                .iter()
                .any(|clause| self.condition_references_relation(clause, target_rti)),
            Qual::Not(inner_condition) => {
                self.condition_references_relation(inner_condition, target_rti)
            }
            _ => true, // Assume other conditions reference the target relation
        }
    }
}

/// Log detailed condition analysis for diagnostic purposes
pub fn log_condition_analysis(analysis: &ConditionAnalysis) {
    match analysis {
        ConditionAnalysis::SafeOrDecomposition {
            total_clauses,
            safe_clauses,
            target_clauses,
        } => {
            warning!(
                "OR decomposition successful: {} total clauses, {} safe clauses, {} for target relation",
                total_clauses, safe_clauses, target_clauses
            );
        }
        ConditionAnalysis::UnsafeOuterJoin { join_type, reason } => {
            warning!(
                "OR decomposition rejected: {:?} join type incompatible with cross-table OR ({})",
                join_type,
                reason
            );
        }
        ConditionAnalysis::CrossTableDependency { dependencies } => {
            warning!(
                "OR decomposition rejected: cross-table dependencies detected: {:?}",
                dependencies
            );
        }
        ConditionAnalysis::PartialAcceptance {
            safe_parts,
            unsafe_parts,
            details,
        } => {
            warning!(
                "Condition partially accepted: {} safe parts, {} unsafe parts ({})",
                safe_parts,
                unsafe_parts,
                details
            );
        }
        ConditionAnalysis::CompleteRejection { reason, details } => {
            warning!("Condition completely rejected: {} ({})", reason, details);
        }
    }
}

/// Log the results of condition safety classification
pub fn log_condition_safety_result(safety: &ConditionSafety, condition: &Qual) {
    match safety {
        ConditionSafety::Safe(conditions) => {
            warning!(
                "Condition classified as SAFE: {} relation-specific conditions extracted",
                conditions.len()
            );
        }
        ConditionSafety::Unsafe(reason) => {
            warning!(
                "Condition classified as UNSAFE: {} - condition rejected",
                reason
            );
        }
        ConditionSafety::PartialSafe(safe_conditions, unsafe_conditions) => {
            warning!(
                "Condition classified as PARTIALLY SAFE: {} safe conditions, {} unsafe conditions",
                safe_conditions.len(),
                unsafe_conditions.len()
            );
        }
    }
}

/// Extract conditions that are safe to push down to a specific relation
pub fn extract_safe_conditions_for_relation(
    safety: &ConditionSafety,
    target_rti: pg_sys::Index,
) -> Option<Qual> {
    match safety {
        ConditionSafety::Safe(conditions) => {
            let target_conditions: Vec<Qual> = conditions
                .iter()
                .filter(|c| c.relation_rti == target_rti)
                .map(|c| c.condition.clone())
                .collect();

            match target_conditions.len() {
                0 => None,
                1 => Some(target_conditions.into_iter().next().unwrap()),
                _ => Some(Qual::And(target_conditions)),
            }
        }
        ConditionSafety::PartialSafe(safe_conditions, _) => {
            let target_conditions: Vec<Qual> = safe_conditions
                .iter()
                .filter(|c| c.relation_rti == target_rti)
                .map(|c| c.condition.clone())
                .collect();

            match target_conditions.len() {
                0 => None,
                1 => Some(target_conditions.into_iter().next().unwrap()),
                _ => Some(Qual::And(target_conditions)),
            }
        }
        ConditionSafety::Unsafe(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::customscan::pdbscan::join_analysis::JoinType;

    #[test]
    fn test_condition_safety_display() {
        let reason = UnsafetyReason::CrossTableDependency;
        assert_eq!(reason.to_string(), "cross-table dependency");
    }

    #[test]
    fn test_condition_safety_classifier() {
        let mut classifier = ConditionSafetyClassifier::new();

        // Set up a safe join context
        let join_context = JoinContext::new(JoinType::Inner, vec![1, 2]);
        classifier.set_join_context(join_context);
        classifier.set_bm25_relations(vec![1, 2]);

        // Test external variable condition
        let external_condition = Qual::ExternalVar;
        let safety = classifier.classify_condition_safety(&external_condition, 1);
        matches!(
            safety,
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        );
    }

    #[test]
    fn test_extract_safe_conditions() {
        let safe_condition = RelationSpecificCondition {
            relation_rti: 1,
            condition: Qual::All,
            original_context: ConditionContext::Where,
        };

        let safety = ConditionSafety::Safe(vec![safe_condition]);
        let extracted = extract_safe_conditions_for_relation(&safety, 1);
        assert!(extracted.is_some());
    }
}
