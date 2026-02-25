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

//! Filter pushdown support for DataFusion TableProvider.
//!
//! This module translates DataFusion `Expr` filters to Tantivy queries via `SearchQueryInput`.

use crate::api::FieldName;
use crate::index::fast_fields_helper::WhichFastField;
use crate::query::pdb_query::pdb;
use crate::query::SearchQueryInput;
use crate::scan::search_predicate_udf::SearchPredicateUDF;
use crate::schema::SearchFieldType;
use datafusion::common::ScalarValue;
use datafusion::logical_expr::{BinaryExpr, Expr, Operator};
use pgrx::pg_sys;
use std::collections::Bound;
use tantivy::schema::OwnedValue;

/// Analyzes DataFusion filters and converts supported ones to SearchQueryInput.
///
/// This handles:
/// - SearchPredicateUDF: Created by JoinScan for @@@ predicates at the join level.
///   These MUST always be pushed down because they cannot be evaluated elsewhere.
/// - Regular SQL predicates: Equality, range, IN list on indexed columns
///
/// Note: baserestrictinfo predicates (single-table predicates) are handled separately
/// via scan_info.query. The filters passed here are join-level predicates that
/// couldn't be applied at the base relation level.
pub struct FilterAnalyzer<'a> {
    fields: &'a [WhichFastField],
    index_oid: pg_sys::Oid,
}

impl<'a> FilterAnalyzer<'a> {
    pub fn new(fields: &'a [WhichFastField], index_oid: pg_sys::Oid) -> Self {
        Self { fields, index_oid }
    }

    /// Check if a filter can be pushed down.
    pub fn supports(&self, expr: &Expr) -> bool {
        self.try_analyze(expr).is_some()
    }

    /// Analyze a filter expression. Panics if the filter is not supported.
    pub fn analyze(&self, expr: &Expr) -> SearchQueryInput {
        self.try_analyze(expr)
            .unwrap_or_else(|| panic!("unsupported filter expression: {expr}"))
    }

    fn try_analyze(&self, expr: &Expr) -> Option<SearchQueryInput> {
        // SearchPredicateUDF contains the Tantivy query for @@@ predicates at the join level.
        // When pushed down here, the query is folded into the Tantivy scan (preferred path).
        //
        // For cross-table ORs (e.g., `p @@@ 'x' OR s @@@ 'y'`), both paths are active:
        // DataFusion pushes the per-table arms down to individual scans (reducing rows
        // entering the join), while the full cross-table expression also remains as a
        // HashJoinExec filter where invoke_with_args / execute_search evaluates it.
        if let Some(search_udf) = SearchPredicateUDF::try_from_expr(expr) {
            // Only process if it matches our index
            if search_udf.index_oid == self.index_oid {
                return Some(search_udf.query());
            }
            // Different index - not our responsibility
            return None;
        }

        match expr {
            Expr::BinaryExpr(BinaryExpr { left, right, op }) => match op {
                Operator::And => self.translate_and(left, right),
                Operator::Or => self.translate_or(left, right),
                _ => self.translate_comparison(left, right, *op),
            },
            Expr::Not(inner) => self.translate_not(inner),
            Expr::InList(in_list) => self.translate_in_list(in_list),
            Expr::IsNull(inner) => self.translate_null_check(inner, true),
            Expr::IsNotNull(inner) => self.translate_null_check(inner, false),
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Boolean operators
    // -------------------------------------------------------------------------

    fn translate_and(&self, left: &Expr, right: &Expr) -> Option<SearchQueryInput> {
        let left_query = self.try_analyze(left)?;
        let right_query = self.try_analyze(right)?;
        Some(SearchQueryInput::Boolean {
            must: vec![left_query, right_query],
            should: vec![],
            must_not: vec![],
        })
    }

    fn translate_or(&self, left: &Expr, right: &Expr) -> Option<SearchQueryInput> {
        let left_query = self.try_analyze(left)?;
        let right_query = self.try_analyze(right)?;
        Some(SearchQueryInput::Boolean {
            must: vec![],
            should: vec![left_query, right_query],
            must_not: vec![],
        })
    }

    fn translate_not(&self, inner: &Expr) -> Option<SearchQueryInput> {
        let inner_query = self.try_analyze(inner)?;
        Some(SearchQueryInput::Boolean {
            must: vec![SearchQueryInput::All],
            should: vec![],
            must_not: vec![inner_query],
        })
    }

    // -------------------------------------------------------------------------
    // Comparison operators
    // -------------------------------------------------------------------------

    fn translate_comparison(
        &self,
        left: &Expr,
        right: &Expr,
        op: Operator,
    ) -> Option<SearchQueryInput> {
        // Try column op literal
        if let Some(query) = self.try_column_op_literal(left, right, op) {
            return Some(query);
        }
        // Try literal op column (with flipped operator)
        if let Some(query) = self.try_column_op_literal(right, left, flip_operator(op)?) {
            return Some(query);
        }
        None
    }

    fn try_column_op_literal(
        &self,
        column_expr: &Expr,
        literal_expr: &Expr,
        op: Operator,
    ) -> Option<SearchQueryInput> {
        let column_name = extract_column_name(column_expr)?;
        let field_type = self.find_field(&column_name)?;
        let scalar = extract_scalar_value(literal_expr)?;
        let value = scalar_to_owned_value(&scalar, field_type)?;
        let field: FieldName = column_name.into();

        match op {
            Operator::Eq => Some(self.term_query(field, value)),
            Operator::NotEq => Some(self.not_query(self.term_query(field, value))),
            Operator::Lt => Some(self.range_query(field, Bound::Unbounded, Bound::Excluded(value))),
            Operator::LtEq => {
                Some(self.range_query(field, Bound::Unbounded, Bound::Included(value)))
            }
            Operator::Gt => Some(self.range_query(field, Bound::Excluded(value), Bound::Unbounded)),
            Operator::GtEq => {
                Some(self.range_query(field, Bound::Included(value), Bound::Unbounded))
            }
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // IN list
    // -------------------------------------------------------------------------

    fn translate_in_list(
        &self,
        in_list: &datafusion::logical_expr::expr::InList,
    ) -> Option<SearchQueryInput> {
        if in_list.negated {
            return None;
        }

        let column_name = extract_column_name(&in_list.expr)?;
        let field_type = self.find_field(&column_name)?;
        let field: FieldName = column_name.into();

        let terms: Vec<_> = in_list
            .list
            .iter()
            .filter_map(|expr| {
                let scalar = extract_scalar_value(expr)?;
                scalar_to_owned_value(&scalar, field_type)
            })
            .collect();

        if terms.len() != in_list.list.len() {
            return None;
        }

        Some(self.term_set_query(field, terms))
    }

    // -------------------------------------------------------------------------
    // NULL checks
    // -------------------------------------------------------------------------

    fn translate_null_check(&self, inner: &Expr, is_null: bool) -> Option<SearchQueryInput> {
        let column_name = extract_column_name(inner)?;
        self.find_field(&column_name)?;

        let field: FieldName = column_name.into();
        let exists_query = SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::Exists,
        };

        if is_null {
            Some(self.not_query(exists_query))
        } else {
            Some(exists_query)
        }
    }

    // -------------------------------------------------------------------------
    // Query builders
    // -------------------------------------------------------------------------

    fn term_query(&self, field: FieldName, value: OwnedValue) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::Term {
                value,
                is_datetime: false,
            },
        }
    }

    fn term_set_query(&self, field: FieldName, terms: Vec<OwnedValue>) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::TermSet { terms },
        }
    }

    fn range_query(
        &self,
        field: FieldName,
        lower: Bound<OwnedValue>,
        upper: Bound<OwnedValue>,
    ) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::Range {
                lower_bound: lower,
                upper_bound: upper,
                is_datetime: false,
            },
        }
    }

    fn not_query(&self, query: SearchQueryInput) -> SearchQueryInput {
        SearchQueryInput::Boolean {
            must: vec![SearchQueryInput::All],
            should: vec![],
            must_not: vec![query],
        }
    }

    // -------------------------------------------------------------------------
    // Field lookup
    // -------------------------------------------------------------------------

    fn find_field(&self, name: &str) -> Option<&SearchFieldType> {
        self.fields.iter().find_map(|field| match field {
            WhichFastField::Named(field_name, field_type)
            | WhichFastField::Deferred {
                name: field_name,
                field_type,
                ..
            } if field_name == name => Some(field_type),
            _ => None,
        })
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn flip_operator(op: Operator) -> Option<Operator> {
    match op {
        Operator::Eq => Some(Operator::Eq),
        Operator::NotEq => Some(Operator::NotEq),
        Operator::Lt => Some(Operator::Gt),
        Operator::LtEq => Some(Operator::GtEq),
        Operator::Gt => Some(Operator::Lt),
        Operator::GtEq => Some(Operator::LtEq),
        _ => None,
    }
}

fn extract_column_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Column(col) => Some(col.name.clone()),
        _ => None,
    }
}

fn extract_scalar_value(expr: &Expr) -> Option<ScalarValue> {
    match expr {
        Expr::Literal(scalar, _) => Some(scalar.clone()),
        _ => None,
    }
}

fn scalar_to_owned_value(scalar: &ScalarValue, field_type: &SearchFieldType) -> Option<OwnedValue> {
    match (scalar, field_type) {
        // Integer types (I64)
        (ScalarValue::Int8(Some(v)), SearchFieldType::I64(_)) => Some(OwnedValue::I64(*v as i64)),
        (ScalarValue::Int16(Some(v)), SearchFieldType::I64(_)) => Some(OwnedValue::I64(*v as i64)),
        (ScalarValue::Int32(Some(v)), SearchFieldType::I64(_)) => Some(OwnedValue::I64(*v as i64)),
        (ScalarValue::Int64(Some(v)), SearchFieldType::I64(_)) => Some(OwnedValue::I64(*v)),

        // Unsigned integer types (U64)
        (ScalarValue::UInt8(Some(v)), SearchFieldType::U64(_)) => Some(OwnedValue::U64(*v as u64)),
        (ScalarValue::UInt16(Some(v)), SearchFieldType::U64(_)) => Some(OwnedValue::U64(*v as u64)),
        (ScalarValue::UInt32(Some(v)), SearchFieldType::U64(_)) => Some(OwnedValue::U64(*v as u64)),
        (ScalarValue::UInt64(Some(v)), SearchFieldType::U64(_)) => Some(OwnedValue::U64(*v)),

        // Cross-type integer conversions
        (ScalarValue::Int8(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
            Some(OwnedValue::U64(*v as u64))
        }
        (ScalarValue::Int16(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
            Some(OwnedValue::U64(*v as u64))
        }
        (ScalarValue::Int32(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
            Some(OwnedValue::U64(*v as u64))
        }
        (ScalarValue::Int64(Some(v)), SearchFieldType::U64(_)) if *v >= 0 => {
            Some(OwnedValue::U64(*v as u64))
        }

        // Float types (F64)
        (ScalarValue::Float32(Some(v)), SearchFieldType::F64(_)) => {
            Some(OwnedValue::F64(*v as f64))
        }
        (ScalarValue::Float64(Some(v)), SearchFieldType::F64(_)) => Some(OwnedValue::F64(*v)),

        // Integer to float conversion
        (ScalarValue::Int64(Some(v)), SearchFieldType::F64(_)) => Some(OwnedValue::F64(*v as f64)),
        (ScalarValue::Int32(Some(v)), SearchFieldType::F64(_)) => Some(OwnedValue::F64(*v as f64)),

        // Boolean
        (ScalarValue::Boolean(Some(v)), SearchFieldType::Bool(_)) => Some(OwnedValue::Bool(*v)),

        // String/Text types
        (ScalarValue::Utf8(Some(v)), SearchFieldType::Text(_)) => Some(OwnedValue::Str(v.clone())),
        (ScalarValue::LargeUtf8(Some(v)), SearchFieldType::Text(_)) => {
            Some(OwnedValue::Str(v.clone()))
        }
        (ScalarValue::Utf8View(Some(v)), SearchFieldType::Text(_)) => {
            Some(OwnedValue::Str(v.clone()))
        }

        // Numeric64 (scaled integers)
        (ScalarValue::Int64(Some(v)), SearchFieldType::Numeric64(_, scale)) => {
            let multiplier = 10i64.pow(*scale as u32);
            Some(OwnedValue::I64(v * multiplier))
        }
        (ScalarValue::Float64(Some(v)), SearchFieldType::Numeric64(_, scale)) => {
            let multiplier = 10f64.powi(*scale as i32);
            Some(OwnedValue::I64((v * multiplier).round() as i64))
        }

        _ => None,
    }
}

/// Combine multiple SearchQueryInput queries with AND.
pub fn combine_with_and(queries: Vec<SearchQueryInput>) -> Option<SearchQueryInput> {
    match queries.len() {
        0 => None,
        1 => Some(queries.into_iter().next().unwrap()),
        _ => Some(SearchQueryInput::Boolean {
            must: queries,
            should: vec![],
            must_not: vec![],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::logical_expr::col;

    #[test]
    fn test_flip_operator() {
        assert_eq!(flip_operator(Operator::Eq), Some(Operator::Eq));
        assert_eq!(flip_operator(Operator::NotEq), Some(Operator::NotEq));
        assert_eq!(flip_operator(Operator::Lt), Some(Operator::Gt));
        assert_eq!(flip_operator(Operator::LtEq), Some(Operator::GtEq));
        assert_eq!(flip_operator(Operator::Gt), Some(Operator::Lt));
        assert_eq!(flip_operator(Operator::GtEq), Some(Operator::LtEq));
    }

    #[test]
    fn test_extract_column_name() {
        let expr = col("my_column");
        assert_eq!(extract_column_name(&expr), Some("my_column".to_string()));

        let literal = Expr::Literal(ScalarValue::Int32(Some(42)), None);
        assert_eq!(extract_column_name(&literal), None);
    }
}
