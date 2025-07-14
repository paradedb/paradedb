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

/// Represents the type of join condition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JoinConditionType {
    /// Equi-join: uses equality operator between columns (e.g., a.id = b.ref_id)
    /// This is safe for cross-table OR decomposition as it guarantees one-to-one matching
    Equi,
    /// Non-equi join: uses non-equality operators (e.g., a.id > b.ref_id, a.id != b.ref_id)
    /// This is unsafe for cross-table OR decomposition as it can produce multiple matches
    NonEqui,
    /// Cross join: no explicit join condition (implicit TRUE condition)
    /// This is unsafe for cross-table OR decomposition as it produces cartesian product
    Cross,
    /// Complex condition: contains multiple operators or complex expressions
    /// This is unsafe for cross-table OR decomposition as the semantics are unclear
    Complex,
    /// Unknown or unparseable condition
    Unknown,
}

impl JoinConditionType {
    /// Returns true if this join condition type is safe for cross-table OR decomposition
    pub fn is_safe_for_cross_table_or_decomposition(&self) -> bool {
        matches!(self, JoinConditionType::Equi)
    }
}

impl JoinType {
    /// Returns true if this join type is safe for cross-table OR condition decomposition
    /// for the specified target relation
    pub fn is_safe_for_cross_table_or_decomposition(
        &self,
        target_rti: pg_sys::Index,
        left_rti: pg_sys::Index,
        right_rti: pg_sys::Index,
    ) -> bool {
        match self {
            JoinType::Inner => true,
            JoinType::LeftOuter => {
                // LEFT JOIN: safe to push conditions to the right side (non-preserved side)
                target_rti == right_rti
            }
            JoinType::RightOuter => {
                // RIGHT JOIN: safe to push conditions to the left side (non-preserved side)
                target_rti == left_rti
            }
            JoinType::Cross
            | JoinType::FullOuter
            | JoinType::Semi
            | JoinType::Anti
            | JoinType::Unknown => false,
        }
    }

    /// Returns true if this join type is safe for OR condition decomposition
    pub fn is_safe_for_or_decomposition(&self) -> bool {
        matches!(
            self,
            JoinType::Inner | JoinType::LeftOuter | JoinType::RightOuter
        )
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
    /// The type of join condition
    pub join_condition_type: JoinConditionType,
    /// Relations participating in the join
    pub participating_relations: Vec<pg_sys::Index>,
    /// Left relation in the join (for LEFT/RIGHT JOIN support)
    pub left_rti: Option<pg_sys::Index>,
    /// Right relation in the join (for LEFT/RIGHT JOIN support)
    pub right_rti: Option<pg_sys::Index>,
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

        // Default to unknown join condition type - will be set by analyze_join_context
        let join_condition_type = JoinConditionType::Unknown;

        let is_safe = match join_type {
            JoinType::Inner | JoinType::LeftOuter | JoinType::RightOuter => true,
            JoinType::Cross
            | JoinType::FullOuter
            | JoinType::Semi
            | JoinType::Anti
            | JoinType::Unknown => false,
        };

        Self {
            join_type,
            join_condition_type,
            participating_relations,
            left_rti: None,
            right_rti: None,
            nullable_relations,
            join_conditions: vec![],
            is_safe,
        }
    }

    /// Set the join condition type
    pub fn set_join_condition_type(&mut self, join_condition_type: JoinConditionType) {
        self.join_condition_type = join_condition_type;
    }

    /// Set the left and right relation RTIs for proper LEFT/RIGHT JOIN support
    pub fn set_join_sides(&mut self, left_rti: pg_sys::Index, right_rti: pg_sys::Index) {
        self.left_rti = Some(left_rti);
        self.right_rti = Some(right_rti);
    }

    /// Check if this join context is safe for cross-table OR decomposition for the target relation
    pub fn is_safe_for_cross_table_or_decomposition(&self, target_rti: pg_sys::Index) -> bool {
        // First check if the join condition type is safe
        if !self
            .join_condition_type
            .is_safe_for_cross_table_or_decomposition()
        {
            return false;
        }

        // Then check if the join type is safe for the target relation
        if let (Some(left_rti), Some(right_rti)) = (self.left_rti, self.right_rti) {
            self.join_type
                .is_safe_for_cross_table_or_decomposition(target_rti, left_rti, right_rti)
        } else {
            // If we don't have join side information, fall back to conservative check
            matches!(self.join_type, JoinType::Inner)
        }
    }

    /// Returns true if this join context is safe for OR decomposition
    pub fn is_safe_for_or_decomposition(&self) -> bool {
        self.join_type.is_safe_for_or_decomposition()
    }

    /// Returns true if the specified relation can be nullable in this join
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

        // Extract the actual join type using our new implementation
        let join_type = if participating_relations.len() > 1 {
            extract_join_type(root, target_rti)
        } else {
            JoinType::Inner // Single relation
        };

        let mut join_context = JoinContext::new(join_type, participating_relations.clone());

        // Set join sides for proper LEFT/RIGHT JOIN support
        // For binary joins, we need to determine which is left and which is right
        if participating_relations.len() == 2 {
            // For now, use the order in the range table as left/right indication
            // The target_rti and the other relation form the join pair
            let other_rti = participating_relations
                .iter()
                .find(|&&rti| rti != target_rti)
                .copied()
                .unwrap_or(target_rti);

            // Determine left/right based on range table order
            if target_rti < other_rti {
                join_context.set_join_sides(target_rti, other_rti);
            } else {
                join_context.set_join_sides(other_rti, target_rti);
            }
        }

        // Analyze join condition type
        let join_condition_type = analyze_join_condition_type(root, target_rti);
        join_context.set_join_condition_type(join_condition_type);

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

    // Extract the actual join type using our new implementation
    let join_type = if participating_relations.len() > 1 {
        extract_join_type(root, target_rti)
    } else {
        JoinType::Inner // Single relation
    };

    let mut join_context = JoinContext::new(join_type, participating_relations.clone());

    // Set join sides for proper LEFT/RIGHT JOIN support
    // For binary joins, we need to determine which is left and which is right
    if participating_relations.len() == 2 {
        // For now, use the order in the range table as left/right indication
        // The target_rti and the other relation form the join pair
        let other_rti = participating_relations
            .iter()
            .find(|&&rti| rti != target_rti)
            .copied()
            .unwrap_or(target_rti);

        // Determine left/right based on range table order
        if target_rti < other_rti {
            join_context.set_join_sides(target_rti, other_rti);
        } else {
            join_context.set_join_sides(other_rti, target_rti);
        }
    }

    // Analyze join condition type
    let join_condition_type = analyze_join_condition_type(root, target_rti);
    join_context.set_join_condition_type(join_condition_type);

    join_context
}

/// Extract join type from PostgreSQL join information
pub unsafe fn extract_join_type(
    root: *mut pg_sys::PlannerInfo,
    target_rti: pg_sys::Index,
) -> JoinType {
    // First, check if we can find join information in the range table
    let query = (*root).parse;
    let rtable = (*query).rtable;
    let rtable_len = pg_sys::list_length(rtable);

    // Look for JOIN range table entries that might indicate the join type
    for i in 1..=rtable_len {
        let rti = i as pg_sys::Index;
        let rte = pg_sys::rt_fetch(rti, rtable);

        // Check if this is a JOIN RTE
        if (*rte).rtekind == pg_sys::RTEKind::RTE_JOIN {
            // RTE_JOIN entries contain join type information
            if let Some(join_type) = extract_join_type_from_join_rte(rte, target_rti) {
                return join_type;
            }
        }
    }

    // If no explicit JOIN RTE found, analyze join relationships through RelOptInfo
    if let Some(join_type) = analyze_join_relationships(root, target_rti) {
        return join_type;
    }

    // If we have multiple relations but no explicit join information,
    // it's likely an implicit inner join or cross join
    if rtable_len > 1 {
        // Check if there are explicit join conditions - if so, analyze them
        if has_join_conditions(root, target_rti) {
            // There are join conditions, check if they indicate a CROSS JOIN
            let condition_type = analyze_join_condition_type(root, target_rti);
            if condition_type == JoinConditionType::Cross {
                JoinType::Cross
            } else {
                JoinType::Inner
            }
        } else {
            // No explicit join conditions - could be CROSS JOIN
            JoinType::Cross
        }
    } else {
        // Single relation - no joins
        JoinType::Inner
    }
}

/// Extract join type from a JOIN range table entry
unsafe fn extract_join_type_from_join_rte(
    rte: *mut pg_sys::RangeTblEntry,
    _target_rti: pg_sys::Index,
) -> Option<JoinType> {
    // JOIN RTEs contain the join type in the jointype field
    match (*rte).jointype {
        pg_sys::JoinType::JOIN_INNER => Some(JoinType::Inner),
        pg_sys::JoinType::JOIN_LEFT => Some(JoinType::LeftOuter),
        pg_sys::JoinType::JOIN_RIGHT => Some(JoinType::RightOuter),
        pg_sys::JoinType::JOIN_FULL => Some(JoinType::FullOuter),
        pg_sys::JoinType::JOIN_SEMI => Some(JoinType::Semi),
        pg_sys::JoinType::JOIN_ANTI => Some(JoinType::Anti),
        // For other join types or if we can't determine, return None
        _ => None,
    }
}

/// Analyze join relationships through RelOptInfo and join clauses
unsafe fn analyze_join_relationships(
    root: *mut pg_sys::PlannerInfo,
    target_rti: pg_sys::Index,
) -> Option<JoinType> {
    // Access the RelOptInfo for the target relation
    if target_rti as usize >= (*root).simple_rel_array_size as usize {
        return None;
    }

    let relinfo = *(*root).simple_rel_array.add(target_rti as usize);
    if relinfo.is_null() {
        return None;
    }

    // Check if this relation has join information
    let joinlist = (*relinfo).joininfo;
    if joinlist.is_null() {
        return None;
    }

    // Analyze the join clauses to infer join type
    // For now, we assume most explicit joins are INNER JOINs
    // This could be enhanced to analyze the specific join conditions
    Some(JoinType::Inner)
}

/// Check if there are join conditions involving the target relation
unsafe fn has_join_conditions(root: *mut pg_sys::PlannerInfo, target_rti: pg_sys::Index) -> bool {
    if target_rti as usize >= (*root).simple_rel_array_size as usize {
        return false;
    }

    let relinfo = *(*root).simple_rel_array.add(target_rti as usize);
    if relinfo.is_null() {
        return false;
    }

    let joinlist = (*relinfo).joininfo;
    !joinlist.is_null() && pg_sys::list_length(joinlist) > 0
}

/// Analyze the join condition type to determine if it's an equi-join
pub unsafe fn analyze_join_condition_type(
    root: *mut pg_sys::PlannerInfo,
    target_rti: pg_sys::Index,
) -> JoinConditionType {
    // Access the RelOptInfo for the target relation
    if target_rti as usize >= (*root).simple_rel_array_size as usize {
        pgrx::warning!(
            "analyze_join_condition_type: target_rti {} out of bounds",
            target_rti
        );
        return JoinConditionType::Unknown;
    }

    let relinfo = *(*root).simple_rel_array.add(target_rti as usize);
    if relinfo.is_null() {
        pgrx::warning!(
            "analyze_join_condition_type: relinfo is null for target_rti {}",
            target_rti
        );
        return JoinConditionType::Unknown;
    }

    let joinlist = (*relinfo).joininfo;
    if joinlist.is_null() || pg_sys::list_length(joinlist) == 0 {
        // No join conditions - this is likely a CROSS JOIN
        pgrx::warning!("analyze_join_condition_type: no join conditions found for target_rti {}, returning Cross", target_rti);
        return JoinConditionType::Cross;
    }

    // Check each join condition
    let joininfo = pgrx::PgList::<pg_sys::RestrictInfo>::from_pg(joinlist);
    let mut has_equi_condition = false;
    let mut has_non_equi_condition = false;

    pgrx::warning!(
        "analyze_join_condition_type: found {} join conditions for target_rti {}",
        joininfo.len(),
        target_rti
    );

    for ri in joininfo.iter_ptr() {
        let clause = (*ri).clause as *mut pg_sys::Node;
        let condition_type = analyze_single_join_condition(clause);

        pgrx::warning!(
            "analyze_join_condition_type: analyzed join condition, type = {:?}",
            condition_type
        );

        match condition_type {
            JoinConditionType::Equi => has_equi_condition = true,
            JoinConditionType::NonEqui => has_non_equi_condition = true,
            JoinConditionType::Complex => {
                pgrx::warning!("analyze_join_condition_type: returning Complex");
                return JoinConditionType::Complex;
            }
            JoinConditionType::Cross => {
                pgrx::warning!("analyze_join_condition_type: returning Cross");
                return JoinConditionType::Cross;
            }
            JoinConditionType::Unknown => {
                pgrx::warning!("analyze_join_condition_type: returning Unknown");
                return JoinConditionType::Unknown;
            }
        }
    }

    // If we have both equi and non-equi conditions, it's complex
    if has_equi_condition && has_non_equi_condition {
        pgrx::warning!(
            "analyze_join_condition_type: mixed equi and non-equi conditions, returning Complex"
        );
        return JoinConditionType::Complex;
    }

    // If we have any non-equi conditions, it's non-equi
    if has_non_equi_condition {
        pgrx::warning!("analyze_join_condition_type: has non-equi conditions, returning NonEqui");
        return JoinConditionType::NonEqui;
    }

    // If we have equi conditions, it's equi
    if has_equi_condition {
        pgrx::warning!("analyze_join_condition_type: has equi conditions, returning Equi");
        return JoinConditionType::Equi;
    }

    // No conditions found - this is likely a CROSS JOIN
    pgrx::warning!("analyze_join_condition_type: no conditions matched, returning Cross");
    JoinConditionType::Cross
}

/// Analyze a single join condition to determine its type
unsafe fn analyze_single_join_condition(clause: *mut pg_sys::Node) -> JoinConditionType {
    if clause.is_null() {
        pgrx::warning!("analyze_single_join_condition: clause is null");
        return JoinConditionType::Unknown;
    }

    pgrx::warning!(
        "analyze_single_join_condition: clause type = {:?}",
        (*clause).type_
    );

    match (*clause).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            pgrx::warning!("analyze_single_join_condition: processing OpExpr");
            let opexpr = clause as *mut pg_sys::OpExpr;
            analyze_opexpr_join_condition(opexpr)
        }
        pg_sys::NodeTag::T_BoolExpr => {
            pgrx::warning!("analyze_single_join_condition: processing BoolExpr");
            let boolexpr = clause as *mut pg_sys::BoolExpr;
            analyze_boolexpr_join_condition(boolexpr)
        }
        pg_sys::NodeTag::T_Const => {
            pgrx::warning!("analyze_single_join_condition: processing Const");
            let const_node = clause as *mut pg_sys::Const;
            // Check if this is a constant TRUE or FALSE
            if (*const_node).consttype == pg_sys::BOOLOID {
                if (*const_node).constisnull {
                    return JoinConditionType::Unknown;
                }
                let value = (*const_node).constvalue.value() != 0;
                if value {
                    // TRUE constant - this is a CROSS JOIN
                    return JoinConditionType::Cross;
                } else {
                    // FALSE constant - this is unusual but not an equi-join
                    return JoinConditionType::NonEqui;
                }
            }
            JoinConditionType::Unknown
        }
        _ => {
            pgrx::warning!("analyze_single_join_condition: unknown clause type, returning Complex");
            JoinConditionType::Complex
        }
    }
}

/// Analyze an OpExpr to determine if it's an equi-join condition
unsafe fn analyze_opexpr_join_condition(opexpr: *mut pg_sys::OpExpr) -> JoinConditionType {
    // Check if this is an equality operator
    let op_oid = (*opexpr).opno;

    // Use PostgreSQL's system catalog to determine if this is an equality operator
    let is_equality = is_equality_operator(op_oid);

    // Debug logging to understand what's happening
    pgrx::warning!(
        "Analyzing OpExpr: operator OID = {}, is_equality = {}",
        op_oid,
        is_equality
    );

    if is_equality {
        // For equality, we need to check if the operands are from different relations
        let args = pgrx::PgList::<pg_sys::Node>::from_pg((*opexpr).args);
        if args.len() == 2 {
            let left_arg = args.get_ptr(0).unwrap();
            let right_arg = args.get_ptr(1).unwrap();

            // Check if both arguments are Vars from different relations
            if is_var_from_different_relations(left_arg, right_arg) {
                pgrx::warning!("OpExpr is an equi-join condition");
                return JoinConditionType::Equi;
            } else {
                pgrx::warning!("OpExpr is equality but not from different relations");
            }
        } else {
            pgrx::warning!("OpExpr has {} args, expected 2", args.len());
        }
    }

    // Any other operator is non-equi
    pgrx::warning!("OpExpr is non-equi");
    JoinConditionType::NonEqui
}

/// Analyze a BoolExpr to determine if it's an equi-join condition
unsafe fn analyze_boolexpr_join_condition(boolexpr: *mut pg_sys::BoolExpr) -> JoinConditionType {
    pgrx::warning!(
        "analyze_boolexpr_join_condition: boolop = {:?}",
        (*boolexpr).boolop
    );

    match (*boolexpr).boolop {
        pg_sys::BoolExprType::AND_EXPR => {
            // For AND expressions, we need to find the actual join conditions
            // and ignore single-relation filter conditions
            let args = pgrx::PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut join_conditions = Vec::new();
            let mut has_non_equi_join = false;

            pgrx::warning!(
                "analyze_boolexpr_join_condition: AND_EXPR with {} args",
                args.len()
            );

            for (i, arg) in args.iter_ptr().enumerate() {
                // Check if this is actually a join condition (involves different relations)
                let is_join_condition = match (*arg).type_ {
                    pg_sys::NodeTag::T_OpExpr => {
                        let opexpr = arg as *mut pg_sys::OpExpr;
                        let args = pgrx::PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                        if args.len() == 2 {
                            let left_arg = args.get_ptr(0).unwrap();
                            let right_arg = args.get_ptr(1).unwrap();
                            is_var_from_different_relations(left_arg, right_arg)
                        } else {
                            false
                        }
                    }
                    _ => false, // Other node types are not simple join conditions
                };

                if is_join_condition {
                    let condition_type = analyze_single_join_condition(arg);
                    pgrx::warning!(
                        "analyze_boolexpr_join_condition: AND_EXPR arg[{}] is join condition, type = {:?}",
                        i,
                        condition_type
                    );

                    if condition_type != JoinConditionType::Equi {
                        has_non_equi_join = true;
                    }
                    join_conditions.push(condition_type);
                } else {
                    pgrx::warning!(
                        "analyze_boolexpr_join_condition: AND_EXPR arg[{}] is not a join condition, ignoring",
                        i
                    );
                }
            }

            // If we found no actual join conditions, this is likely a cross join
            if join_conditions.is_empty() {
                pgrx::warning!(
                    "analyze_boolexpr_join_condition: AND_EXPR no join conditions found, returning Cross"
                );
                return JoinConditionType::Cross;
            }

            // If all join conditions are equi-joins, return Equi
            if !has_non_equi_join {
                pgrx::warning!(
                    "analyze_boolexpr_join_condition: AND_EXPR all join conditions are equi, returning Equi"
                );
                JoinConditionType::Equi
            } else {
                pgrx::warning!(
                    "analyze_boolexpr_join_condition: AND_EXPR has non-equi join conditions, returning Complex"
                );
                JoinConditionType::Complex
            }
        }
        pg_sys::BoolExprType::OR_EXPR => {
            // OR expressions are always complex for join conditions
            pgrx::warning!("analyze_boolexpr_join_condition: OR_EXPR, returning Complex");
            JoinConditionType::Complex
        }
        pg_sys::BoolExprType::NOT_EXPR => {
            // NOT expressions are always complex for join conditions
            pgrx::warning!("analyze_boolexpr_join_condition: NOT_EXPR, returning Complex");
            JoinConditionType::Complex
        }
        _ => {
            // Unknown boolean expression type
            pgrx::warning!("analyze_boolexpr_join_condition: unknown boolop, returning Complex");
            JoinConditionType::Complex
        }
    }
}

/// Check if an operator is an equality operator by consulting PostgreSQL's system catalog
unsafe fn is_equality_operator(op_oid: pg_sys::Oid) -> bool {
    // Use PostgreSQL's built-in catalog access to check operator strategy
    // Equality operators typically have strategy number 3 (BTEqualStrategyNumber) in btree opclasses

    // For now, use a focused set of common equality operators that are likely to be used in joins
    // This is more conservative but should catch the most common cases
    let common_equality_ops = [
        pg_sys::Oid::from(96u32),   // INT4EQ (=)
        pg_sys::Oid::from(98u32),   // TEXTEQ (=)
        pg_sys::Oid::from(410u32),  // INT8EQ (=)
        pg_sys::Oid::from(23u32),   // INT2EQ (=)
        pg_sys::Oid::from(92u32),   // BOOLEQ (=)
        pg_sys::Oid::from(352u32),  // OIDEQ (=)
        pg_sys::Oid::from(1752u32), // NUMERICEQ (=)
        pg_sys::Oid::from(1158u32), // VARCHAREQ (=)
        pg_sys::Oid::from(1840u32), // BPCHAREQ (=)
        pg_sys::Oid::from(1093u32), // DATEEQ (=)
        pg_sys::Oid::from(1108u32), // TIMEEQ (=)
        pg_sys::Oid::from(1054u32), // TIMESTAMPEQ (=)
        pg_sys::Oid::from(1070u32), // TIMESTAMPTZ_EQ (=)
        pg_sys::Oid::from(1120u32), // INTERVAL_EQ (=)
        pg_sys::Oid::from(416u32),  // FLOAT4EQ (=)
        pg_sys::Oid::from(470u32),  // FLOAT8EQ (=)
        pg_sys::Oid::from(1862u32), // BYTEAEQ (=)
        pg_sys::Oid::from(2972u32), // UUIDEQ (=)
        pg_sys::Oid::from(184u32),  // NAMEEQ (=)
        pg_sys::Oid::from(61u32),   // CHAREQ (=)
        pg_sys::Oid::from(1534u32), // MONEQ (=)
        pg_sys::Oid::from(1676u32), // MACADDREQ (=)
        pg_sys::Oid::from(1955u32), // INET_EQ (=)
        pg_sys::Oid::from(1959u32), // CIDR_EQ (=)
        pg_sys::Oid::from(1817u32), // TIDEQ (=)
        pg_sys::Oid::from(1801u32), // XIDEQ (=)
        pg_sys::Oid::from(1805u32), // XIDEQ8 (=)
        pg_sys::Oid::from(1809u32), // CIDEQ (=)
        pg_sys::Oid::from(1813u32), // CIDEQ8 (=)
        pg_sys::Oid::from(1950u32), // ARRAY_EQ (=)
        pg_sys::Oid::from(2060u32), // RECORD_EQ (=)
    ];

    common_equality_ops.contains(&op_oid)
}

/// Check if two nodes are Vars from different relations
unsafe fn is_var_from_different_relations(
    left: *mut pg_sys::Node,
    right: *mut pg_sys::Node,
) -> bool {
    // Strip RelabelType nodes if present
    let left_var = strip_relabel_type(left);
    let right_var = strip_relabel_type(right);

    // Check if both are Vars
    if (*left_var).type_ != pg_sys::NodeTag::T_Var || (*right_var).type_ != pg_sys::NodeTag::T_Var {
        return false;
    }

    let left_var = left_var as *mut pg_sys::Var;
    let right_var = right_var as *mut pg_sys::Var;

    // Check if they're from different relations
    (*left_var).varno != (*right_var).varno
}

/// Strip RelabelType nodes to get to the underlying expression
unsafe fn strip_relabel_type(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    while (*node).type_ == pg_sys::NodeTag::T_RelabelType {
        let relabel = node as *mut pg_sys::RelabelType;
        node = (*relabel).arg as *mut pg_sys::Node;
    }
    node
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
