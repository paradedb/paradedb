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

use std::sync::OnceLock;

use datafusion::common::{Column, ScalarValue, TableReference};
use datafusion::logical_expr::{col, lit, BinaryExpr, Expr, Operator};
use pgrx::{pg_sys, PgList};

use crate::api::HashMap;
use crate::postgres::customscan::joinscan::build::{
    JoinLevelExpr, JoinLevelSearchPredicate, JoinSource,
};
use crate::postgres::customscan::joinscan::privdat::{OutputColumnInfo, SCORE_COL_NAME};
use crate::postgres::customscan::opexpr::{
    initialize_equality_operator_lookup, OperatorAccepts, PostgresOperatorOid, TantivyOperator,
};
use crate::scan::SearchPredicateUDF;

static OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();

pub(super) trait ColumnMapper {
    /// Map a PostgreSQL variable to a DataFusion Column expression
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr>;
}

/// Helper struct for translating PostgreSQL expression trees into DataFusion `Expr`s.
pub(super) struct PredicateTranslator<'a> {
    pub sources: &'a [JoinSource],
    mapper: Option<Box<dyn ColumnMapper + 'a>>,
}

impl<'a> PredicateTranslator<'a> {
    pub fn new(sources: &'a [JoinSource]) -> Self {
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
    pub unsafe fn translate_join_level_expr(
        expr: &JoinLevelExpr,
        custom_exprs: &[Expr],
        ctid_map: &HashMap<pg_sys::Index, Expr>,
        predicates: &[JoinLevelSearchPredicate],
    ) -> Option<Expr> {
        match expr {
            JoinLevelExpr::SingleTablePredicate {
                source_idx: _,
                predicate_idx,
            } => {
                let predicate = predicates.get(*predicate_idx)?;
                let col = ctid_map.get(&predicate.rti)?;
                // Create a SearchPredicateUDF that carries the search query.
                // This will be pushed down to PgSearchTableProvider via filter pushdown.
                let udf = SearchPredicateUDF::new(
                    predicate.indexrelid,
                    predicate.heaprelid,
                    predicate.query.clone(),
                    predicate.expr_ptr.as_ptr(),
                    predicate.planner_info_ptr.as_ptr(),
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
                )?;
                for child in &children[1..] {
                    let right =
                        Self::translate_join_level_expr(child, custom_exprs, ctid_map, predicates)?;
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
                )?;
                for child in &children[1..] {
                    let right =
                        Self::translate_join_level_expr(child, custom_exprs, ctid_map, predicates)?;
                    result = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(result),
                        Operator::Or,
                        Box::new(right),
                    ));
                }
                Some(result)
            }
            JoinLevelExpr::Not(child) => {
                let inner =
                    Self::translate_join_level_expr(child, custom_exprs, ctid_map, predicates)?;
                Some(Expr::Not(Box::new(inner)))
            }
        }
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

        match (*node).type_ {
            pg_sys::NodeTag::T_OpExpr => self.translate_op_expr(node as *mut pg_sys::OpExpr),
            pg_sys::NodeTag::T_Var => self.translate_var(node as *mut pg_sys::Var),
            pg_sys::NodeTag::T_Const => self.translate_const(node as *mut pg_sys::Const),
            pg_sys::NodeTag::T_BoolExpr => self.translate_bool_expr(node as *mut pg_sys::BoolExpr),
            // Type casts (RelabelType, CoerceViaIO, FuncExpr) are not supported because
            // they may change value semantics. Cross-type comparisons like INT < NUMERIC
            // require proper type coercion that DataFusion cannot perform correctly when
            // the underlying fast field storage uses different scales/representations.
            _ => None,
        }
    }

    unsafe fn translate_op_expr(&self, op_expr: *mut pg_sys::OpExpr) -> Option<Expr> {
        let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
        if args.len() != 2 {
            return None; // Only support binary operators for now
        }

        let left = self.translate(args.get_ptr(0)?)?;
        let right = self.translate(args.get_ptr(1)?)?;

        let opno = (*op_expr).opno;
        let op_str = self.lookup_operator(opno)?;

        let op = match op_str {
            "=" => Operator::Eq,
            "<>" => Operator::NotEq,
            "<" => Operator::Lt,
            "<=" => Operator::LtEq,
            ">" => Operator::Gt,
            ">=" => Operator::GtEq,
            _ => return None,
        };

        Some(Expr::BinaryExpr(BinaryExpr::new(
            Box::new(left),
            op,
            Box::new(right),
        )))
    }

    unsafe fn translate_var(&self, var: *mut pg_sys::Var) -> Option<Expr> {
        let varno = (*var).varno as pg_sys::Index;
        let varattno = (*var).varattno;

        // INDEX_VAR is allowed because it represents a reference to the custom scan's output
        if varno != pg_sys::INDEX_VAR as pg_sys::Index
            && !self.sources.iter().any(|s| s.contains_rti(varno))
        {
            return None;
        }

        if let Some(ref mapper) = self.mapper {
            if let Some(expr) = mapper.map_var(varno, varattno) {
                return Some(expr);
            }
            return None;
        }

        Some(col("placeholder"))
    }

    unsafe fn translate_const(&self, c: *mut pg_sys::Const) -> Option<Expr> {
        if (*c).constisnull {
            return Some(lit(ScalarValue::Null));
        }

        let type_oid = (*c).consttype;
        let datum = (*c).constvalue;

        // Simple mapping for common types
        let scalar_value = match type_oid {
            pg_sys::INT2OID => {
                let val = datum.value() as i16;
                ScalarValue::Int16(Some(val))
            }
            pg_sys::INT4OID => {
                let val = datum.value() as i32;
                ScalarValue::Int32(Some(val))
            }
            pg_sys::INT8OID => {
                let val = datum.value() as i64;
                ScalarValue::Int64(Some(val))
            }
            pg_sys::FLOAT4OID => {
                let val = f32::from_bits(datum.value() as u32);
                ScalarValue::Float32(Some(val))
            }
            pg_sys::FLOAT8OID => {
                let val = f64::from_bits(datum.value() as u64);
                ScalarValue::Float64(Some(val))
            }
            pg_sys::BOOLOID => {
                let val = datum.value() != 0;
                ScalarValue::Boolean(Some(val))
            }
            _ => return None,
        };

        Some(lit(scalar_value))
    }

    unsafe fn translate_bool_expr(&self, bool_expr: *mut pg_sys::BoolExpr) -> Option<Expr> {
        let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

        match (*bool_expr).boolop {
            pg_sys::BoolExprType::AND_EXPR => {
                if args.len() < 2 {
                    return None;
                }
                let mut expr = self.translate(args.get_ptr(0)?)?;
                for i in 1..args.len() {
                    let right = self.translate(args.get_ptr(i)?)?;
                    expr = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(expr),
                        Operator::And,
                        Box::new(right),
                    ));
                }
                Some(expr)
            }
            pg_sys::BoolExprType::OR_EXPR => {
                if args.len() < 2 {
                    return None;
                }
                let mut expr = self.translate(args.get_ptr(0)?)?;
                for i in 1..args.len() {
                    let right = self.translate(args.get_ptr(i)?)?;
                    expr = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(expr),
                        Operator::Or,
                        Box::new(right),
                    ));
                }
                Some(expr)
            }
            pg_sys::BoolExprType::NOT_EXPR => {
                if args.len() != 1 {
                    return None;
                }
                let child = self.translate(args.get_ptr(0)?)?;
                Some(Expr::Not(Box::new(child)))
            }
            _ => None,
        }
    }

    fn lookup_operator(&self, opno: pg_sys::Oid) -> Option<&'static str> {
        let lookup = OPERATOR_LOOKUP
            .get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::All) });
        lookup.get(&opno).copied()
    }
}

/// Creates a DataFusion column expression with a bare table reference.
/// This is preferred over `datafusion::logical_expr::col()` because `col()` parses the input string,
pub(super) fn make_col(relation: &str, name: &str) -> Expr {
    Expr::Column(Column::new(
        Some(TableReference::Bare {
            table: relation.into(),
        }),
        name,
    ))
}

pub(super) struct CombinedMapper<'a> {
    pub sources: &'a [JoinSource],
    pub output_columns: &'a [OutputColumnInfo],
}

impl<'a> ColumnMapper for CombinedMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        // 1. Resolve to (rti, attno, is_score)
        let (rti, attno, is_score) = if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            let idx = (varattno - 1) as usize;
            let info = self.output_columns.get(idx)?;
            (info.rti, info.original_attno, info.is_score)
        } else {
            (varno, varattno, false)
        };

        // 2. Find the source
        let (source_idx, source) = self
            .sources
            .iter()
            .enumerate()
            .find(|(_, s)| s.contains_rti(rti))?;

        let alias = source.execution_alias(source_idx);

        // 3. Resolve column name
        if is_score {
            // Try to resolve score via map_var(rti, 0) first (for nested joins)
            if let Some(col_idx) = source.map_var(rti, 0) {
                if let Some(name) = source.column_name(col_idx) {
                    return Some(make_col(&alias, &name));
                }
            }
            // Default to alias-specific score alias
            return Some(make_col(&alias, SCORE_COL_NAME));
        }

        // Normal column
        // We need to map the rti/attno to the source's output attno
        let mapped_attno = source.map_var(rti, attno)?;
        let col_name = source.column_name(mapped_attno)?;
        Some(make_col(&alias, &col_name))
    }
}
