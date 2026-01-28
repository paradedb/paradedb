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

use std::sync::{Arc, OnceLock};

use datafusion::common::{Column, ScalarValue, TableReference};
use datafusion::logical_expr::{col, lit, BinaryExpr, Expr, Operator};
use pgrx::{pg_sys, PgList};

use crate::api::HashMap;
use crate::postgres::customscan::joinscan::build::JoinLevelExpr;
use crate::postgres::customscan::joinscan::build::ScanInfo;
use crate::postgres::customscan::joinscan::privdat::{
    OutputColumnInfo, INNER_SCORE_ALIAS, OUTER_SCORE_ALIAS,
};
use crate::postgres::customscan::joinscan::udf::RowInSetUDF;
use crate::postgres::customscan::opexpr::{
    initialize_equality_operator_lookup, OperatorAccepts, PostgresOperatorOid, TantivyOperator,
};

static OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();

pub trait ColumnMapper {
    /// Map a PostgreSQL variable to a DataFusion Column expression
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr>;
}

/// Helper struct for translating PostgreSQL expression trees into DataFusion `Expr`s.
///
/// This struct handles the mapping of PostgreSQL variables (Vars) to DataFusion columns,
/// taking into account the source relation (outer vs inner) and any necessary aliasing.
/// It also handles the translation of operators and constants.
pub struct PredicateTranslator<'a> {
    #[allow(dead_code)]
    outer_side: &'a ScanInfo,
    #[allow(dead_code)]
    inner_side: &'a ScanInfo,
    outer_rti: pg_sys::Index,
    inner_rti: pg_sys::Index,
    mapper: Option<Box<dyn ColumnMapper + 'a>>,
}

impl<'a> PredicateTranslator<'a> {
    pub fn new(
        outer_side: &'a ScanInfo,
        inner_side: &'a ScanInfo,
        outer_rti: pg_sys::Index,
        inner_rti: pg_sys::Index,
    ) -> Self {
        Self {
            outer_side,
            inner_side,
            outer_rti,
            inner_rti,
            mapper: None,
        }
    }

    pub fn with_mapper(mut self, mapper: Box<dyn ColumnMapper + 'a>) -> Self {
        self.mapper = Some(mapper);
        self
    }

    /// Translate a `JoinLevelExpr` tree to a DataFusion `Expr`.
    pub unsafe fn translate_join_level_expr(
        expr: &JoinLevelExpr,
        custom_exprs: &[Expr],
        join_level_sets: &[Arc<Vec<u64>>],
        outer_ctid_col: &Expr,
        inner_ctid_col: &Expr,
    ) -> Option<Expr> {
        match expr {
            JoinLevelExpr::SingleTablePredicate {
                side,
                predicate_idx,
            } => {
                let set = join_level_sets.get(*predicate_idx)?.clone();
                let col = match side {
                    crate::postgres::customscan::joinscan::build::JoinSide::Outer => {
                        outer_ctid_col.clone()
                    }
                    crate::postgres::customscan::joinscan::build::JoinSide::Inner => {
                        inner_ctid_col.clone()
                    }
                };
                let udf = datafusion::logical_expr::ScalarUDF::new_from_impl(RowInSetUDF::new(set));
                Some(udf.call(vec![col]))
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
                    join_level_sets,
                    outer_ctid_col,
                    inner_ctid_col,
                )?;
                for child in &children[1..] {
                    let right = Self::translate_join_level_expr(
                        child,
                        custom_exprs,
                        join_level_sets,
                        outer_ctid_col,
                        inner_ctid_col,
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
                    join_level_sets,
                    outer_ctid_col,
                    inner_ctid_col,
                )?;
                for child in &children[1..] {
                    let right = Self::translate_join_level_expr(
                        child,
                        custom_exprs,
                        join_level_sets,
                        outer_ctid_col,
                        inner_ctid_col,
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
                    join_level_sets,
                    outer_ctid_col,
                    inner_ctid_col,
                )?;
                Some(Expr::Not(Box::new(inner)))
            }
        }
    }

    /// Translate a PostgreSQL expression to a DataFusion `Expr`.
    ///
    /// Returns `None` if the expression cannot be translated.
    pub unsafe fn translate(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        if node.is_null() {
            return None;
        }

        match (*node).type_ {
            pg_sys::NodeTag::T_OpExpr => self.translate_op_expr(node as *mut pg_sys::OpExpr),
            pg_sys::NodeTag::T_Var => self.translate_var(node as *mut pg_sys::Var),
            pg_sys::NodeTag::T_Const => self.translate_const(node as *mut pg_sys::Const),
            pg_sys::NodeTag::T_BoolExpr => self.translate_bool_expr(node as *mut pg_sys::BoolExpr),
            // T_RelabelType is common (casting), often safe to ignore if types are compatible
            // For now, stricter is better.
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

        // Check if the var belongs to one of the relations we are handling.
        // INDEX_VAR is allowed because it represents a reference to the custom scan's output,
        // which contains the columns from both sides of the join.
        if varno != self.outer_rti
            && varno != self.inner_rti
            && varno != pg_sys::INDEX_VAR as pg_sys::Index
        {
            // Reference to a relation outside the join?
            return None;
        }

        if let Some(ref mapper) = self.mapper {
            if let Some(expr) = mapper.map_var(varno, varattno) {
                return Some(expr);
            }
            return None;
        }

        // For validation (no mapper), we just need to ensure it's a valid reference.
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
            // TODO: Add support for TEXT strings via pgrx utils
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
///
/// This is preferred over `datafusion::logical_expr::col()` because `col()` parses the input string,
/// which can lead to normalization issues with mixed-case identifiers unless strictly quoted.
/// Constructing `Expr::Column` directly ensures the identifier is used exactly as provided.
pub fn make_col(relation: &str, name: &str) -> Expr {
    Expr::Column(Column::new(
        Some(TableReference::Bare {
            table: relation.into(),
        }),
        name,
    ))
}

/// Helper to map PostgreSQL variables to DataFusion column expressions across both sides
/// of the join. Used during predicate translation.
///
/// `output_columns` is used to resolve `INDEX_VAR` references. These variables
/// point to the output columns of the custom scan itself, and must be mapped
/// back to the original source relation (outer or inner) and its attribute
/// to find the correct column in the DataFusion plan.
pub struct CombinedMapper<'a> {
    pub outer: &'a ScanInfo,
    pub inner: &'a ScanInfo,
    pub output_columns: &'a [OutputColumnInfo],
    pub outer_alias: &'a str,
    pub inner_alias: &'a str,
}

impl<'a> CombinedMapper<'a> {
    pub fn get_field_name(side: &ScanInfo, attno: pg_sys::AttrNumber) -> Option<String> {
        side.fields
            .iter()
            .find(|f| f.attno == attno)
            .map(|f| f.field.name())
    }
}

impl<'a> ColumnMapper for CombinedMapper<'a> {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<Expr> {
        // Handle INDEX_VAR references which point to the custom scan output columns
        if varno == pg_sys::INDEX_VAR as pg_sys::Index {
            let idx = (varattno - 1) as usize;
            if idx >= self.output_columns.len() {
                return None;
            }
            let info = &self.output_columns[idx];

            if info.is_outer {
                if info.is_score {
                    return Some(make_col(self.outer_alias, OUTER_SCORE_ALIAS));
                } else {
                    let field_name = Self::get_field_name(self.outer, info.original_attno)?;
                    return Some(make_col(self.outer_alias, &field_name));
                }
            } else if info.is_score {
                return Some(make_col(self.inner_alias, INNER_SCORE_ALIAS));
            } else {
                let field_name = Self::get_field_name(self.inner, info.original_attno)?;
                return Some(make_col(self.inner_alias, &field_name));
            }
        }

        if varno == self.outer.heap_rti.unwrap_or(0) {
            let field_name = Self::get_field_name(self.outer, varattno)?;
            Some(make_col(self.outer_alias, &field_name))
        } else if varno == self.inner.heap_rti.unwrap_or(0) {
            let field_name = Self::get_field_name(self.inner, varattno)?;
            Some(make_col(self.inner_alias, &field_name))
        } else {
            None
        }
    }
}
