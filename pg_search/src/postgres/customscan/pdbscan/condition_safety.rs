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
    /// The join condition is not an equi-join, making cross-table OR decomposition unsafe
    NonEquiJoinCondition,
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
            UnsafetyReason::NonEquiJoinCondition => write!(f, "non-equi join condition"),
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

        // Check if this is a cross-table OR condition by examining all clauses
        let mut target_relation_clauses = Vec::new();
        let mut external_clauses = Vec::new();

        for clause in clauses {
            if self.condition_references_relation(clause, target_rti)
                && !clause.contains_external_var()
            {
                target_relation_clauses.push(clause);
            } else {
                external_clauses.push(clause);
            }
        }

        // Handle cross-table OR conditions based on join type and join condition type
        if !target_relation_clauses.is_empty() && !external_clauses.is_empty() {
            // Check if the join type and join condition type support cross-table OR decomposition
            if let Some(ref join_context) = self.join_context {
                if !join_context.is_safe_for_cross_table_or_decomposition(target_rti) {
                    // Check if it's the join type or join condition type that's unsafe
                    let join_type_safe = if let (Some(left_rti), Some(right_rti)) =
                        (join_context.left_rti, join_context.right_rti)
                    {
                        join_context
                            .join_type
                            .is_safe_for_cross_table_or_decomposition(
                                target_rti, left_rti, right_rti,
                            )
                    } else {
                        matches!(
                            join_context.join_type,
                            crate::postgres::customscan::pdbscan::join_analysis::JoinType::Inner
                        )
                    };

                    let condition_type_safe = join_context
                        .join_condition_type
                        .is_safe_for_cross_table_or_decomposition();

                    if !condition_type_safe {
                        warning!(
                            "Cross-table OR with join type {:?} and condition type {:?} - cannot decompose safely: {} clauses for relation {}, {} external clauses",
                            join_context.join_type,
                            join_context.join_condition_type,
                            target_relation_clauses.len(),
                            target_rti,
                            external_clauses.len()
                        );
                        warning!(
                            "Cross-table OR decomposition rejected: join condition type {:?} is not an equi-join (only equi-joins are safe)",
                            join_context.join_condition_type
                        );
                        return ConditionSafety::Unsafe(UnsafetyReason::NonEquiJoinCondition);
                    } else if !join_type_safe {
                        warning!(
                            "Cross-table OR with join type {:?} and condition type {:?} - cannot decompose safely: {} clauses for relation {}, {} external clauses",
                            join_context.join_type,
                            join_context.join_condition_type,
                            target_relation_clauses.len(),
                            target_rti,
                            external_clauses.len()
                        );
                        warning!(
                            "Cross-table OR decomposition rejected: join type {:?} is not safe for target relation {}",
                            join_context.join_type,
                            target_rti
                        );
                        return ConditionSafety::Unsafe(UnsafetyReason::OuterJoinSemantics);
                    } else {
                        warning!(
                            "Cross-table OR with join type {:?} and condition type {:?} - cannot decompose safely: {} clauses for relation {}, {} external clauses",
                            join_context.join_type,
                            join_context.join_condition_type,
                            target_relation_clauses.len(),
                            target_rti,
                            external_clauses.len()
                        );
                        warning!(
                            "Cross-table OR decomposition would change query semantics - rejecting"
                        );
                        return ConditionSafety::Unsafe(UnsafetyReason::SemanticViolation);
                    }
                } else {
                    warning!(
                        "Cross-table OR with join type {:?} and condition type {:?} - safe to decompose: {} clauses for relation {}, {} external clauses",
                        join_context.join_type,
                        join_context.join_condition_type,
                        target_relation_clauses.len(),
                        target_rti,
                        external_clauses.len()
                    );
                }
            } else {
                // Unknown join context - be conservative
                warning!(
                    "Cross-table OR with unknown join context - cannot decompose safely: {} clauses for relation {}, {} external clauses",
                    target_relation_clauses.len(),
                    target_rti,
                    external_clauses.len()
                );
                return ConditionSafety::Unsafe(UnsafetyReason::SemanticViolation);
            }
        }

        // Process OR conditions (either single-relation or safe cross-table)
        if target_relation_clauses.len() == clauses.len() {
            warning!(
                "Single-relation OR condition - safe to push down {} clauses to relation {}",
                clauses.len(),
                target_rti
            );
        } else {
            warning!(
                "Cross-table OR condition with {:?} - processing {} target clauses for relation {}",
                self.join_context
                    .as_ref()
                    .map(|ctx| &ctx.join_type)
                    .unwrap_or(
                        &crate::postgres::customscan::pdbscan::join_analysis::JoinType::Unknown
                    ),
                target_relation_clauses.len(),
                target_rti
            );
        }

        let mut safe_conditions = Vec::new();
        for clause in &target_relation_clauses {
            match self.classify_condition_safety(clause, target_rti) {
                ConditionSafety::Safe(mut conditions) => {
                    safe_conditions.append(&mut conditions);
                }
                ConditionSafety::Unsafe(_) => {
                    // Skip unsafe clauses but continue with the safe ones
                    warning!(
                        "Skipping unsafe clause in OR condition for relation {}",
                        target_rti
                    );
                }
                ConditionSafety::PartialSafe(mut safe_parts, _) => {
                    safe_conditions.append(&mut safe_parts);
                }
            }
        }

        // Create a single OR condition for this relation
        if safe_conditions.is_empty() {
            warning!("No safe conditions found for relation {} in OR", target_rti);
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        } else {
            ConditionSafety::Safe(vec![RelationSpecificCondition {
                relation_rti: target_rti,
                condition: if safe_conditions.len() == 1 {
                    safe_conditions[0].condition.clone()
                } else {
                    Qual::Or(
                        safe_conditions
                            .iter()
                            .map(|c| c.condition.clone())
                            .collect(),
                    )
                },
                original_context: ConditionContext::Where,
            }])
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
            // For conditions that reference the target relation, we can always push them down
            // if the relation has a BM25 index. The distinction between search and non-search
            // conditions is handled at the execution level.
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
        let mut unsafe_conditions = Vec::new();

        for clause in clauses {
            if self.condition_references_relation(clause, target_rti)
                && !clause.contains_external_var()
            {
                target_conditions.push(RelationSpecificCondition {
                    relation_rti: target_rti,
                    condition: clause.clone(),
                    original_context: ConditionContext::Where,
                });
            } else {
                // This condition doesn't belong to the target relation
                unsafe_conditions.push(UnsafeCondition {
                    condition: clause.clone(),
                    reason: UnsafetyReason::CrossTableDependency,
                });
            }
        }

        if target_conditions.is_empty() {
            ConditionSafety::Unsafe(UnsafetyReason::CrossTableDependency)
        } else if unsafe_conditions.is_empty() {
            // All conditions belong to this relation
            ConditionSafety::Safe(vec![RelationSpecificCondition {
                relation_rti: target_rti,
                condition: if target_conditions.len() == 1 {
                    target_conditions[0].condition.clone()
                } else {
                    Qual::Or(
                        target_conditions
                            .iter()
                            .map(|c| c.condition.clone())
                            .collect(),
                    )
                },
                original_context: ConditionContext::Where,
            }])
        } else {
            // Mixed conditions - some belong to this relation, some don't
            // This is expected for cross-table OR conditions
            ConditionSafety::Safe(vec![RelationSpecificCondition {
                relation_rti: target_rti,
                condition: if target_conditions.len() == 1 {
                    target_conditions[0].condition.clone()
                } else {
                    Qual::Or(
                        target_conditions
                            .iter()
                            .map(|c| c.condition.clone())
                            .collect(),
                    )
                },
                original_context: ConditionContext::Where,
            }])
        }
    }

    /// Check if a condition references a specific relation
    fn condition_references_relation(&self, condition: &Qual, target_rti: pg_sys::Index) -> bool {
        let references = match condition {
            Qual::OpExpr { .. } => {
                // For OpExpr, we need to check if it references the target relation
                // This is a simplified check - we assume that if it doesn't contain external vars,
                // it belongs to the current relation being processed
                !condition.contains_external_var()
            }
            Qual::HeapExpr { .. } => {
                // HeapExpr conditions can belong to any relation
                // We need to check if it references the target relation
                // For now, assume it belongs to the target relation if it doesn't have external vars
                !condition.contains_external_var()
            }
            Qual::ExternalVar | Qual::ExternalExpr => false,
            Qual::And(clauses) => {
                // For AND conditions, check if this entire condition is safe for the target relation
                // An AND condition is only safe if ALL clauses belong to the target relation
                self.is_and_condition_safe_for_relation(clauses, target_rti)
            }
            Qual::Or(clauses) => clauses
                .iter()
                .any(|clause| self.condition_references_relation(clause, target_rti)),
            Qual::Not(inner_condition) => {
                self.condition_references_relation(inner_condition, target_rti)
            }
            _ => {
                // For other condition types, check if they contain external variables
                // If they don't, assume they belong to the target relation
                !condition.contains_external_var()
            }
        };

        references
    }

    /// Check if an AND condition is safe for a specific relation
    /// An AND condition is only safe if ALL its clauses belong to the same relation
    fn is_and_condition_safe_for_relation(
        &self,
        clauses: &[Qual],
        target_rti: pg_sys::Index,
    ) -> bool {
        // An AND condition is cross-table if it contains any cross-table dependencies
        // We need to check if the entire AND condition can be safely attributed to one relation

        let mut has_target_relation_clause = false;
        let mut has_cross_table_dependency = false;

        for clause in clauses {
            if clause.contains_external_var() {
                // External variables indicate cross-table dependencies
                has_cross_table_dependency = true;
                break;
            }

            // Check if this clause might be a join condition
            if self.is_potential_join_condition(clause) {
                has_cross_table_dependency = true;
                break;
            }

            // Check if this clause references the target relation
            if self.condition_references_simple(clause, target_rti) {
                has_target_relation_clause = true;
            }
        }

        // Return true only if we have target relation clauses and no cross-table dependencies
        has_target_relation_clause && !has_cross_table_dependency
    }

    /// Check if a condition might be a join condition
    /// This is a heuristic to detect conditions that involve multiple tables
    fn is_potential_join_condition(&self, condition: &Qual) -> bool {
        match condition {
            Qual::OpExpr { .. } => {
                // For OpExpr, we use a heuristic: if it contains external vars, it's likely a join condition
                // This is not perfect, but it's better than missing join conditions
                condition.contains_external_var()
            }
            Qual::ExternalVar | Qual::ExternalExpr => true,
            Qual::Not(inner_condition) => self.is_potential_join_condition(inner_condition),
            _ => false,
        }
    }

    /// Simple check for condition reference without recursion into AND/OR
    fn condition_references_simple(&self, condition: &Qual, target_rti: pg_sys::Index) -> bool {
        match condition {
            Qual::OpExpr { .. } | Qual::HeapExpr { .. } => {
                // For simple expressions, assume they belong to target relation if no external vars
                !condition.contains_external_var()
            }
            Qual::ExternalVar | Qual::ExternalExpr => false,
            Qual::Not(inner_condition) => {
                self.condition_references_simple(inner_condition, target_rti)
            }
            _ => !condition.contains_external_var(),
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
