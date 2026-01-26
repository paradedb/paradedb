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

use std::any::Any;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use arrow_array::builder::BooleanBuilder;
use arrow_array::{Array, RecordBatch, UInt64Array};
use arrow_schema::{DataType, Schema};
use datafusion_common::{Result, ScalarValue};
use datafusion_expr::{ColumnarValue, Operator};
use datafusion_physical_expr::expressions::{BinaryExpr, Column, Literal, NotExpr};
use datafusion_physical_expr::PhysicalExpr;
use pgrx::{pg_sys, PgList};

use crate::api::HashMap;
use crate::postgres::customscan::joinscan::build::JoinLevelExpr;
use crate::postgres::customscan::joinscan::build::JoinSideInfo;
use crate::postgres::customscan::opexpr::{
    initialize_equality_operator_lookup, OperatorAccepts, PostgresOperatorOid, TantivyOperator,
};

static OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> = OnceLock::new();

#[derive(Debug, Clone, Eq)]
pub struct RowInSetExpr {
    arg: Arc<dyn PhysicalExpr>,
    set: Arc<Vec<u64>>, // Sorted set of valid ctids
}

impl Hash for RowInSetExpr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.arg.hash(state);
        self.set.hash(state);
    }
}

impl PartialEq for RowInSetExpr {
    fn eq(&self, other: &Self) -> bool {
        self.arg.eq(&other.arg) && self.set == other.set
    }
}

impl std::fmt::Display for RowInSetExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "RowInSetExpr(set_len={})", self.set.len())
    }
}

impl PhysicalExpr for RowInSetExpr {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn data_type(&self, _input_schema: &Schema) -> Result<DataType> {
        Ok(DataType::Boolean)
    }
    fn nullable(&self, _input_schema: &Schema) -> Result<bool> {
        Ok(false)
    }
    fn evaluate(&self, batch: &RecordBatch) -> Result<ColumnarValue> {
        let arg_val = self.arg.evaluate(batch)?;
        match arg_val {
            ColumnarValue::Array(array) => {
                let ctids = array
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("Expected UInt64Array for ctid");
                let mut builder = BooleanBuilder::with_capacity(ctids.len());
                for i in 0..ctids.len() {
                    if ctids.is_null(i) {
                        builder.append_null();
                    } else {
                        let ctid = ctids.value(i);
                        // binary search since set is sorted
                        builder.append_value(self.set.binary_search(&ctid).is_ok());
                    }
                }
                Ok(ColumnarValue::Array(Arc::new(builder.finish())))
            }
            ColumnarValue::Scalar(scalar) => match scalar {
                ScalarValue::UInt64(Some(ctid)) => {
                    let is_present = self.set.binary_search(&ctid).is_ok();
                    Ok(ColumnarValue::Scalar(ScalarValue::Boolean(Some(
                        is_present,
                    ))))
                }
                _ => Ok(ColumnarValue::Scalar(ScalarValue::Boolean(None))),
            },
        }
    }
    fn children(&self) -> Vec<&Arc<dyn PhysicalExpr>> {
        vec![&self.arg]
    }
    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn PhysicalExpr>>,
    ) -> Result<Arc<dyn PhysicalExpr>> {
        Ok(Arc::new(RowInSetExpr {
            arg: children[0].clone(),
            set: self.set.clone(),
        }))
    }
    fn fmt_sql(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} IN SET (size={})", self.arg, self.set.len())
    }
}

pub trait ColumnMapper {
    fn map_var(&self, varno: pg_sys::Index, varattno: pg_sys::AttrNumber) -> Option<usize>;
}

/// Helper struct for translating PostgreSQL expression trees into DataFusion `PhysicalExpr`s.
///
/// This is used during planning to convert join conditions and filters into a format
/// that DataFusion can execute. It handles:
/// - Mapping PostgreSQL `Var` nodes to DataFusion column indices
/// - Translating common operators and boolean expressions
/// - Converting PostgreSQL constants to DataFusion `ScalarValue`s
///
/// It also handles the translation of the special `JoinLevelExpr` tree, which combines
/// standard SQL predicates with Tantivy search results (bitmap scans).
pub struct PredicateTranslator<'a> {
    // We keep these for context, though they might be unused in simple validation
    #[allow(dead_code)]
    outer_side: &'a JoinSideInfo,
    #[allow(dead_code)]
    inner_side: &'a JoinSideInfo,
    outer_rti: pg_sys::Index,
    inner_rti: pg_sys::Index,
    mapper: Option<Box<dyn ColumnMapper + 'a>>,
}

impl<'a> PredicateTranslator<'a> {
    pub fn new(
        outer_side: &'a JoinSideInfo,
        inner_side: &'a JoinSideInfo,
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

    /// Translate a `JoinLevelExpr` tree to a DataFusion `PhysicalExpr`.
    pub unsafe fn translate_join_level_expr(
        expr: &JoinLevelExpr,
        custom_exprs: &[Arc<dyn PhysicalExpr>],
        join_level_sets: &[Arc<Vec<u64>>],
        outer_ctid_col: &Arc<dyn PhysicalExpr>,
        inner_ctid_col: &Arc<dyn PhysicalExpr>,
    ) -> Option<Arc<dyn PhysicalExpr>> {
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
                Some(Arc::new(RowInSetExpr { arg: col, set }))
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
                    result = Arc::new(BinaryExpr::new(result, Operator::And, right));
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
                    result = Arc::new(BinaryExpr::new(result, Operator::Or, right));
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
                Some(Arc::new(NotExpr::new(inner)))
            }
        }
    }

    /// Translate a PostgreSQL expression to a DataFusion `PhysicalExpr`.
    ///
    /// Returns `None` if the expression cannot be translated.
    pub unsafe fn translate(&self, node: *mut pg_sys::Node) -> Option<Arc<dyn PhysicalExpr>> {
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

    unsafe fn translate_op_expr(
        &self,
        op_expr: *mut pg_sys::OpExpr,
    ) -> Option<Arc<dyn PhysicalExpr>> {
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

        Some(Arc::new(BinaryExpr::new(left, op, right)))
    }

    unsafe fn translate_var(&self, var: *mut pg_sys::Var) -> Option<Arc<dyn PhysicalExpr>> {
        let varno = (*var).varno as pg_sys::Index;
        let varattno = (*var).varattno;

        // Check if the var belongs to one of the relations we are handling
        if varno != self.outer_rti && varno != self.inner_rti {
            // Reference to a relation outside the join?
            return None;
        }

        if let Some(ref mapper) = self.mapper {
            if let Some(index) = mapper.map_var(varno, varattno) {
                // TODO: Get actual column name from schema if possible, or just use "colX"
                return Some(Arc::new(Column::new(&format!("col{index}"), index)));
            }
            return None;
        }

        // For validation (no mapper), we just need to ensure it's a valid reference.
        Some(Arc::new(Column::new("placeholder", 0)))
    }

    unsafe fn translate_const(&self, c: *mut pg_sys::Const) -> Option<Arc<dyn PhysicalExpr>> {
        if (*c).constisnull {
            return Some(Arc::new(Literal::new(ScalarValue::Null)));
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

        Some(Arc::new(Literal::new(scalar_value)))
    }

    unsafe fn translate_bool_expr(
        &self,
        bool_expr: *mut pg_sys::BoolExpr,
    ) -> Option<Arc<dyn PhysicalExpr>> {
        let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

        match (*bool_expr).boolop {
            pg_sys::BoolExprType::AND_EXPR => {
                if args.len() < 2 {
                    return None;
                }
                let mut expr = self.translate(args.get_ptr(0)?)?;
                for i in 1..args.len() {
                    let right = self.translate(args.get_ptr(i)?)?;
                    expr = Arc::new(BinaryExpr::new(expr, Operator::And, right));
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
                    expr = Arc::new(BinaryExpr::new(expr, Operator::Or, right));
                }
                Some(expr)
            }
            pg_sys::BoolExprType::NOT_EXPR => {
                if args.len() != 1 {
                    return None;
                }
                let child = self.translate(args.get_ptr(0)?)?;
                Some(Arc::new(NotExpr::new(child)))
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
