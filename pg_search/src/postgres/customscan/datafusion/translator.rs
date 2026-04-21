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

use datafusion::common::{Column, JoinType, Result, TableReference};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{col, lit, BinaryExpr, Expr, Operator};
use datafusion::prelude::DataFrame;
use pgrx::pg_sys;

use crate::api::{HashMap, HashSet};
use crate::postgres::customscan::joinscan::build::{
    JoinLevelExpr, JoinLevelSearchPredicate, JoinNode, JoinSource, JoinType as PgJoinType,
    RelationAlias,
};
use crate::postgres::customscan::joinscan::privdat::{OutputColumnInfo, SCORE_COL_NAME};
use crate::scan::SearchPredicateUDF;

pub trait ColumnMapper {
    /// Map a PostgreSQL variable to a DataFusion Column expression
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr>;
}

/// Helper struct for translating PostgreSQL expression trees into DataFusion `Expr`s.
pub struct PredicateTranslator<'a> {
    pub sources: &'a [&'a JoinSource],
    pub(crate) mapper: Option<Box<dyn ColumnMapper + 'a>>,
}

impl<'a> PredicateTranslator<'a> {
    pub fn new(sources: &'a [&'a JoinSource]) -> Self {
        Self {
            sources,
            mapper: None,
        }
    }

    pub fn with_mapper(mut self, mapper: Box<dyn ColumnMapper + 'a>) -> Self {
        self.mapper = Some(mapper);
        self
    }

    /// Translate a `JoinLevelExpr` tree to a DataFusion `Expr`.
    ///
    /// This creates `SearchPredicateUDF` expressions for single-table predicates,
    /// which can be pushed down to `PgSearchTableProvider` via DataFusion's
    /// filter pushdown mechanism.
    /// Translate a `JoinLevelExpr` tree to a DataFusion `Expr`.
    ///
    /// `deferred_positions` contains plan_positions whose ctid columns still hold
    /// packed DocAddresses at this point in the plan. Each `SingleTablePredicate`
    /// checks whether its `plan_position` is in the set to determine whether its
    /// `SearchPredicateUDF` should emit packed DocAddresses or real ctids.
    pub unsafe fn translate_join_level_expr(
        expr: &JoinLevelExpr,
        custom_exprs: &[Expr],
        ctid_map: &HashMap<pg_sys::Index, Expr>,
        predicates: &[JoinLevelSearchPredicate],
        deferred_positions: &crate::api::HashSet<usize>,
        sources: &[&JoinSource],
    ) -> Option<Expr> {
        match expr {
            JoinLevelExpr::SingleTablePredicate {
                plan_position,
                predicate_idx,
            } => {
                let predicate = predicates.get(*predicate_idx)?;
                let col = ctid_map.get(&(*plan_position as pg_sys::Index))?;
                let deferred = deferred_positions.contains(plan_position);
                let udf = SearchPredicateUDF::with_deferred_visibility(
                    predicate.indexrelid,
                    predicate.heaprelid,
                    predicate.query.clone(),
                    predicate.display_string.clone(),
                    deferred,
                    Some(*plan_position),
                );
                Some(udf.into_expr(col.clone()))
            }
            JoinLevelExpr::MultiTablePredicate { predicate_idx } => {
                custom_exprs.get(*predicate_idx).cloned()
            }
            JoinLevelExpr::And(children) => {
                if children.is_empty() {
                    return None;
                }
                let mut result = Self::translate_join_level_expr(
                    &children[0],
                    custom_exprs,
                    ctid_map,
                    predicates,
                    deferred_positions,
                    sources,
                )?;
                for child in &children[1..] {
                    let right = Self::translate_join_level_expr(
                        child,
                        custom_exprs,
                        ctid_map,
                        predicates,
                        deferred_positions,
                        sources,
                    )?;
                    result = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(result),
                        Operator::And,
                        Box::new(right),
                    ));
                }
                Some(result)
            }
            JoinLevelExpr::Or(children) => {
                if children.is_empty() {
                    return None;
                }
                let mut result = Self::translate_join_level_expr(
                    &children[0],
                    custom_exprs,
                    ctid_map,
                    predicates,
                    deferred_positions,
                    sources,
                )?;
                for child in &children[1..] {
                    let right = Self::translate_join_level_expr(
                        child,
                        custom_exprs,
                        ctid_map,
                        predicates,
                        deferred_positions,
                        sources,
                    )?;
                    result = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(result),
                        Operator::Or,
                        Box::new(right),
                    ));
                }
                Some(result)
            }
            JoinLevelExpr::Not(child) => {
                let inner = Self::translate_join_level_expr(
                    child,
                    custom_exprs,
                    ctid_map,
                    predicates,
                    deferred_positions,
                    sources,
                )?;
                Some(Expr::Not(Box::new(inner)))
            }
            JoinLevelExpr::MarkOrNull {
                is_anti,
                null_test_varno,
                null_test_attno,
            } => {
                // Resolve the outer column name from source metadata.
                let source = sources.iter().find(|s| s.contains_rti(*null_test_varno));
                let col_name = source.and_then(|s| s.column_name(*null_test_attno))?;

                // Build: mark = true OR col IS NULL  (for IN)
                //        mark = false OR col IS NULL  (for NOT IN)
                let mark_check = if *is_anti {
                    col("mark").eq(lit(false))
                } else {
                    col("mark").eq(lit(true))
                };
                let null_check = Expr::IsNull(Box::new(col(col_name)));
                Some(mark_check.or(null_check))
            }
            // `PgExpression` is only produced as a top-level `JoinNode.filter`
            // for Semi/Anti joins, translated in [`build_join_df_with_filter`]
            // via `stringToNode` + [`PredicateTranslator::translate`] with a
            // `CombinedMapper` against the join's sources. It should never
            // appear inside a `RelNode::Filter` predicate tree.
            JoinLevelExpr::PgExpression { .. } => None,
        }
    }

    /// Check whether an expression can be translated to DataFusion.
    /// Validates both node-type support and column resolution against
    /// the provided sources, without requiring plan_position or
    /// output_columns.
    pub unsafe fn can_translate(sources: &'a [&'a JoinSource], node: *mut pg_sys::Node) -> bool {
        struct ValidationMapper<'a> {
            sources: &'a [&'a JoinSource],
        }

        impl<'a> ColumnMapper for ValidationMapper<'a> {
            /// Just confirm the source exists — field registration hasn't
            /// happened yet at this planning stage. Column validity is
            /// covered by all_vars_are_fast_fields_recursive which runs first.
            fn map_var(&self, varno: pg_sys::Index, _varattno: pg_sys::AttrNumber) -> Option<Expr> {
                let _source = self.sources.iter().find(|s| s.contains_rti(varno))?;
                Some(col("placeholder"))
            }
        }

        let mapper = ValidationMapper { sources };
        let translator = Self::new(sources).with_mapper(Box::new(mapper));
        translator.translate(node).is_some()
    }

    /// Translate a PostgreSQL expression to a DataFusion `Expr`.
    ///
    /// Returns `None` if the expression cannot be translated.
    ///
    /// IMPORTANT: This translator is used to check if a predicate CAN be translated,
    /// but the actual predicate evaluation happens via heap fetch + PostgreSQL evaluation.
    /// Cross-type comparisons (e.g., INT < NUMERIC) involve type casts that change value
    /// semantics - we cannot simply look through them because the underlying storage
    /// representations differ (e.g., INT 95 vs Numeric64 5225 for 52.25).
    ///
    /// For predicates involving type casts, we return None to indicate that the predicate
    /// cannot be evaluated purely in DataFusion and must fall back to PostgreSQL evaluation.
    pub unsafe fn translate(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        if node.is_null() {
            return None;
        }

        let native = match (*node).type_ {
            pg_sys::NodeTag::T_OpExpr => self.translate_op_expr(node as *mut pg_sys::OpExpr),
            pg_sys::NodeTag::T_Var => self.translate_var(node as *mut pg_sys::Var),
            pg_sys::NodeTag::T_Const => self.translate_const(node as *mut pg_sys::Const),
            pg_sys::NodeTag::T_BoolExpr => self.translate_bool_expr(node as *mut pg_sys::BoolExpr),
            pg_sys::NodeTag::T_RelabelType => {
                // Binary-compatible cast (e.g. varchar → text). The underlying
                // datum is identical — just recurse into the inner expression.
                let relabel = node as *mut pg_sys::RelabelType;
                self.translate((*relabel).arg.cast())
            }
            pg_sys::NodeTag::T_FuncExpr => self.translate_func_expr(node),
            pg_sys::NodeTag::T_NullTest => self.translate_null_test(node),
            pg_sys::NodeTag::T_BooleanTest => self.translate_boolean_test(node),
            pg_sys::NodeTag::T_CaseExpr => self.translate_case_expr(node),
            pg_sys::NodeTag::T_CoalesceExpr => self.translate_coalesce_expr(node),
            pg_sys::NodeTag::T_NullIfExpr => self.translate_nullif_expr(node),
            pg_sys::NodeTag::T_MinMaxExpr => self.translate_min_max_expr(node),
            pg_sys::NodeTag::T_ScalarArrayOpExpr => self.translate_scalar_array_op_expr(node),
            pg_sys::NodeTag::T_CoerceViaIO => self.translate_coerce_via_io(node),
            _ => None,
        };

        // Fallback runs per recursive call, so the innermost failing subtree
        // gets wrapped, and its parent can still evaluate natively.
        native.or_else(|| self.try_wrap_as_udf(node))
    }

    // `translate_op_expr`, `translate_var`, `translate_const`,
    // `translate_bool_expr` — plus the extended node-type translators and
    // the UDF fallback — are defined in `expr_translators.rs` on a split
    // `impl PredicateTranslator` block.
}

/// Translate a `JoinLevelExpr` predicate into a DataFusion filter expression
/// and apply it to `df`. This is the shared spine of the `RelNode::Filter`
/// arm in JoinScan's and AggregateScan's `build_relnode_df` recursions —
/// both modules previously inlined the same translate-and-filter sequence.
///
/// `handle_mark` controls JoinScan-specific cleanup: when `true`, a
/// `MarkOrNull` predicate triggers a post-filter projection that drops the
/// synthetic `mark` column from the schema. AggregateScan never produces
/// `MarkOrNull` predicates and passes `false`.
#[allow(clippy::too_many_arguments)]
pub fn apply_join_level_filter(
    mut df: DataFrame,
    predicate: &JoinLevelExpr,
    translated_exprs: &[Expr],
    ctid_map: &HashMap<pg_sys::Index, Expr>,
    join_level_predicates: &[JoinLevelSearchPredicate],
    deferred_positions: &HashSet<usize>,
    sources: &[&JoinSource],
    handle_mark: bool,
) -> Result<DataFrame> {
    let filter_expr = unsafe {
        PredicateTranslator::translate_join_level_expr(
            predicate,
            translated_exprs,
            ctid_map,
            join_level_predicates,
            deferred_positions,
            sources,
        )
    }
    .ok_or_else(|| {
        DataFusionError::Internal(format!(
            "Failed to translate join level expression tree: {:?}",
            predicate
        ))
    })?;

    df = df.filter(filter_expr)?;

    // For MarkOrNull filters in JoinScan, drop the synthetic "mark" column
    // post-filter so it doesn't leak into the projection.
    if handle_mark && matches!(predicate, JoinLevelExpr::MarkOrNull { .. }) {
        let schema = df.schema().clone();
        let proj_cols: Vec<Expr> = schema
            .columns()
            .into_iter()
            .filter(|c| c.name != "mark")
            .map(col)
            .collect();
        if !proj_cols.is_empty() {
            df = df.select(proj_cols)?;
        }
    }

    Ok(df)
}

/// Creates a DataFusion column expression with a bare table reference.
/// This is preferred over `datafusion::logical_expr::col()` because `col()` parses the input string,
pub fn make_col(relation: &str, name: &str) -> Expr {
    Expr::Column(Column::new(
        Some(TableReference::Bare {
            table: relation.into(),
        }),
        name,
    ))
}

/// Allow-list of `JoinNode::join_type` variants accepted by [`build_join_df`].
///
/// JoinScan supports the full set; AggregateScan only the four equi-join
/// variants. The variant choice is exposed at the call site so the policy
/// difference is grep-able.
#[derive(Copy, Clone, Debug)]
pub enum JoinTypeAllowList {
    /// JoinScan: Inner, Left, Right, Full, Semi, Anti, LeftMark, RightMark,
    /// RightSemi, RightAnti.
    All,
    /// AggregateScan: Inner, Left, Right, Full only.
    EquiOnly,
}

/// Lower a `JoinNode` over already-built left/right `DataFrame`s.
///
/// Builds equi-join keys with [`build_equi_join_exprs`], maps the
/// [`super::build::JoinType`] into DataFusion's `JoinType` (subject to
/// `allowed_join_types`), and dispatches to `join_on` (when there are
/// equi keys) or `join` (cross join). Caller is responsible for any
/// post-join filter handling — `JoinNode::filter` is not consulted here.
///
/// Returns `Err(NotImplemented)` if the join type is outside the allow-list.
pub fn build_join_df(
    left: DataFrame,
    right: DataFrame,
    join: &JoinNode,
    allowed_join_types: JoinTypeAllowList,
) -> Result<DataFrame> {
    let on = build_equi_join_exprs(join)?;
    let df_join_type = map_join_type(join.join_type, allowed_join_types)?;

    if on.is_empty() {
        left.join(right, df_join_type, &[], &[], None)
    } else {
        left.join_on(right, df_join_type, on)
    }
}

/// Map a [`super::build::JoinType`] to DataFusion's `JoinType`, honoring the
/// allow-list policy.
fn map_join_type(jt: PgJoinType, allowed_join_types: JoinTypeAllowList) -> Result<JoinType> {
    Ok(match (jt, allowed_join_types) {
        (PgJoinType::Inner, _) => JoinType::Inner,
        (PgJoinType::Left, _) => JoinType::Left,
        (PgJoinType::Right, _) => JoinType::Right,
        (PgJoinType::Full, _) => JoinType::Full,
        (PgJoinType::Semi, JoinTypeAllowList::All) => JoinType::LeftSemi,
        (PgJoinType::Anti, JoinTypeAllowList::All) => JoinType::LeftAnti,
        (PgJoinType::LeftMark, JoinTypeAllowList::All) => JoinType::LeftMark,
        (PgJoinType::RightMark, JoinTypeAllowList::All) => JoinType::RightMark,
        (PgJoinType::RightSemi, JoinTypeAllowList::All) => JoinType::RightSemi,
        (PgJoinType::RightAnti, JoinTypeAllowList::All) => JoinType::RightAnti,
        (jt, JoinTypeAllowList::EquiOnly) => {
            return Err(DataFusionError::NotImplemented(format!(
                "Aggregate-on-join does not support {} JOIN",
                jt
            )));
        }
        (jt, JoinTypeAllowList::All) => {
            return Err(DataFusionError::NotImplemented(format!(
                "{} JOIN is not supported in JoinScan execution",
                jt
            )));
        }
    })
}

/// Like [`build_join_df`], but translates [`JoinNode::filter`] and attaches it
/// to the join (passed to `DataFrame::join`'s filter parameter, which
/// DataFusion lowers to `NestedLoopJoinExec` when there are no equi keys).
///
/// Required for Semi/Anti joins whose condition cannot be expressed as equi
/// keys (e.g. `a.col = b.x OR a.col = b.y`): a post-join filter would mean
/// "cross-join then filter", which drops every left row under LeftAnti
/// semantics. Placing the predicate on the join itself evaluates it per
/// (left, right) pair and preserves correctness.
///
/// The filter expression is built by deserializing the PostgreSQL expression
/// tree (stored in [`JoinLevelExpr::PgExpression`]) with `stringToNode` and
/// translating it via [`PredicateTranslator::translate`] + [`CombinedMapper`]
/// — the same pipeline used for regular join-level conditions, just without
/// the custom_exprs / setrefs round-trip that fails under Semi/Anti tlist
/// pruning.
pub fn build_join_df_with_filter(
    left: DataFrame,
    right: DataFrame,
    join: &JoinNode,
    sources: &[&JoinSource],
    output_columns: &[OutputColumnInfo],
    allowed_join_types: JoinTypeAllowList,
) -> Result<DataFrame> {
    let df_join_type = map_join_type(join.join_type, allowed_join_types)?;

    let filter_expr = match &join.filter {
        Some(JoinLevelExpr::PgExpression { pg_node_string, .. }) => unsafe {
            translate_pg_expression_filter(pg_node_string, sources, output_columns)?
        },
        Some(other) => {
            return Err(DataFusionError::NotImplemented(format!(
                "JoinNode.filter variant {:?} not yet supported in build_join_df_with_filter",
                other
            )));
        }
        None => {
            return build_join_df(left, right, join, allowed_join_types);
        }
    };

    if join.equi_keys.is_empty() {
        return left.join(right, df_join_type, &[], &[], Some(filter_expr));
    }

    // The mixed case (equi keys + join filter) is not exercised today: only
    // the disjunctive Semi/Anti path populates `JoinNode.filter`, and it
    // always yields empty equi keys. Surface an explicit error so a future
    // planner change that introduces the mixed case is noticed instead of
    // silently producing a cross-join.
    Err(DataFusionError::NotImplemented(
        "JoinNode.filter combined with equi-join keys is not yet supported".into(),
    ))
}

/// Deserialize a PostgreSQL expression from its `nodeToString` representation
/// and translate it to a DataFusion `Expr` against `sources`, using the
/// caller-supplied `mapper` to resolve Var nodes. `context` is used only in
/// error messages to disambiguate call sites.
pub unsafe fn translate_pg_node_string<'a>(
    pg_node_string: &str,
    sources: &'a [&'a JoinSource],
    mapper: Box<dyn ColumnMapper + 'a>,
    context: &'static str,
) -> Result<Expr> {
    let c_str = std::ffi::CString::new(pg_node_string).map_err(|e| {
        DataFusionError::Internal(format!("CString conversion failed for {context}: {e}"))
    })?;
    let node = pg_sys::stringToNode(c_str.as_ptr().cast_mut());
    if node.is_null() {
        return Err(DataFusionError::Internal(format!(
            "stringToNode returned null for {context}"
        )));
    }

    let translator = PredicateTranslator::new(sources).with_mapper(mapper);
    translator.translate(node.cast()).ok_or_else(|| {
        DataFusionError::Internal(format!(
            "PredicateTranslator failed to translate deserialized {context}"
        ))
    })
}

/// Translate a `PgExpression` join filter using a [`CombinedMapper`] so Var
/// nodes (carrying their original `(varno, varattno)`) resolve to qualified
/// DataFusion columns.
unsafe fn translate_pg_expression_filter(
    pg_node_string: &str,
    sources: &[&JoinSource],
    output_columns: &[OutputColumnInfo],
) -> Result<Expr> {
    let mapper = CombinedMapper {
        sources,
        output_columns,
    };
    translate_pg_node_string(pg_node_string, sources, Box::new(mapper), "PgExpression")
}

/// Build equi-join filter expressions from a [`JoinNode`]'s key pairs.
///
/// Shared between JoinScan and AggregateScan `build_relnode_df` implementations.
/// Each key pair is resolved against the join's left/right subtrees and converted
/// to a `left_col = right_col` DataFusion expression.
pub fn build_equi_join_exprs(join: &JoinNode) -> Result<Vec<Expr>> {
    let mut on = Vec::with_capacity(join.equi_keys.len());
    for jk in &join.equi_keys {
        let ((left_source, left_attno), (right_source, right_attno)) =
            jk.resolve_against(&join.left, &join.right).ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "Failed to resolve join key to current join sides: outer_rti={}, inner_rti={}",
                    jk.outer_rti, jk.inner_rti
                ))
            })?;

        let left_col_name = left_source
            .column_name(left_attno)
            .ok_or_else(|| DataFusionError::Internal("Missing left join-key column".into()))?;
        let right_col_name = right_source
            .column_name(right_attno)
            .ok_or_else(|| DataFusionError::Internal("Missing right join-key column".into()))?;

        let left_expr = make_source_col(left_source, &left_col_name);
        let right_expr = make_source_col(right_source, &right_col_name);
        on.push(left_expr.eq(right_expr));
    }
    Ok(on)
}

/// Build a DataFusion column expression for a `(source, field_name)` pair.
///
/// The execution alias is built from the source's optional alias and its
/// `plan_position` (the DFS index assigned by `JoinCSClause::new`). Both
/// JoinScan and AggregateScan use this exact pattern; sharing it keeps the
/// alias-construction policy in one place.
pub fn make_source_col(source: &JoinSource, field_name: &str) -> Expr {
    let alias =
        RelationAlias::new(source.scan_info.alias.as_deref()).execution(source.plan_position);
    make_col(&alias, field_name)
}

/// Build a DataFusion column expression for the synthetic score column on the
/// given source. Equivalent to `make_source_col(source, SCORE_COL_NAME)`.
pub fn make_source_score_col(source: &JoinSource) -> Expr {
    make_source_col(source, SCORE_COL_NAME)
}

pub struct CombinedMapper<'a> {
    pub sources: &'a [&'a JoinSource],
    pub output_columns: &'a [OutputColumnInfo],
}

impl<'a> ColumnMapper for CombinedMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        let (rti, attno, is_score) = if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            let idx = (varattno - 1) as usize;
            match self.output_columns.get(idx)? {
                OutputColumnInfo::Var {
                    rti,
                    original_attno,
                    ..
                } => (*rti, *original_attno, false),
                OutputColumnInfo::Score { rti, .. } => (*rti, 0, true),
                OutputColumnInfo::Pruned => return None,
            }
        } else {
            (varno, varattno, false)
        };

        let (plan_position, source) = self
            .sources
            .iter()
            .enumerate()
            .find(|(_, s)| s.contains_rti(rti))?;

        let alias = RelationAlias::new(source.scan_info.alias.as_deref()).execution(plan_position);

        if is_score {
            if let Some(col_idx) = source.map_var(rti, 0) {
                if let Some(name) = source.column_name(col_idx) {
                    return Some(make_col(&alias, &name));
                }
            }
            return Some(make_col(&alias, SCORE_COL_NAME));
        }

        let mapped_attno = source.map_var(rti, attno)?;
        let col_name = source.column_name(mapped_attno)?;
        Some(make_col(&alias, &col_name))
    }
}
