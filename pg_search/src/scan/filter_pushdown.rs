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
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::pdb;
use crate::schema::SearchFieldType;
use datafusion::common::ScalarValue;
use datafusion::logical_expr::{BinaryExpr, Expr, Operator};
use datafusion::physical_plan::PhysicalExpr;
use std::collections::Bound;
use std::sync::Arc;

/// Analyzes DataFusion filters and converts supported ones to SearchQueryInput.
///
/// This handles regular SQL predicates: Equality, range, IN list on indexed columns.
///
/// Note: baserestrictinfo predicates (single-table predicates) are handled separately
/// via scan_info.query. The filters passed here are join-level predicates that
/// couldn't be applied at the base relation level.
pub struct FilterAnalyzer<'a> {
    fields: &'a [WhichFastField],
}

impl<'a> FilterAnalyzer<'a> {
    pub fn new(fields: &'a [WhichFastField]) -> Self {
        Self { fields }
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
            minimum_should_match: None,
        })
    }

    fn translate_or(&self, left: &Expr, right: &Expr) -> Option<SearchQueryInput> {
        let left_query = self.try_analyze(left)?;
        let right_query = self.try_analyze(right)?;
        Some(SearchQueryInput::Boolean {
            must: vec![],
            should: vec![left_query, right_query],
            must_not: vec![],
            minimum_should_match: None,
        })
    }

    fn translate_not(&self, inner: &Expr) -> Option<SearchQueryInput> {
        let inner_query = self.try_analyze(inner)?;
        Some(SearchQueryInput::Boolean {
            must: vec![SearchQueryInput::All],
            should: vec![],
            must_not: vec![inner_query],
            minimum_should_match: None,
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
        let value = PdbOwnedValue::from_scalar(&scalar, field_type)?;
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
                PdbOwnedValue::from_scalar(&scalar, field_type)
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

    fn term_query(&self, field: FieldName, value: PdbOwnedValue) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::Term { value },
        }
    }

    fn term_set_query(&self, field: FieldName, terms: Vec<PdbOwnedValue>) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::TermSet { terms },
        }
    }

    fn range_query(
        &self,
        field: FieldName,
        lower: Bound<PdbOwnedValue>,
        upper: Bound<PdbOwnedValue>,
    ) -> SearchQueryInput {
        SearchQueryInput::FieldedQuery {
            field,
            query: pdb::Query::Range {
                lower_bound: lower,
                upper_bound: upper,
            },
        }
    }

    fn not_query(&self, query: SearchQueryInput) -> SearchQueryInput {
        SearchQueryInput::Boolean {
            must: vec![SearchQueryInput::All],
            should: vec![],
            must_not: vec![query],
            minimum_should_match: None,
        }
    }

    // -------------------------------------------------------------------------
    // Field lookup
    // -------------------------------------------------------------------------

    fn find_field(&self, name: &str) -> Option<&SearchFieldType> {
        self.fields.iter().find_map(|field| {
            if let WhichFastField::Named(field_name, field_type)
            | WhichFastField::Array(field_name, field_type) = field
                && field_name == name
            {
                return Some(field_type);
            }
            None
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

pub fn extract_scalar_value(expr: &Expr) -> Option<ScalarValue> {
    match expr {
        Expr::Literal(scalar, _) => Some(scalar.clone()),
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
            minimum_should_match: None,
        }),
    }
}

/// Build a `ChildFilterDescription` for a unary execution plan node that preserves its child's
/// schema identically (e.g. `VisibilityFilterExec`, `TantivyLookupExec`, `FilterPassthroughExec`,
/// `SegmentedTopKExec`).
///
/// DataFusion's `ChildFilterDescription::from_child` uses `FilterRemapper`, which looks up columns
/// by name via `child_schema.index_of(col.name())`. When the child plan has duplicate column names
/// (common in joins where multiple joined tables share column names like `"id"` or `"color"`),
/// `index_of` always returns the *first* column with that name, silently corrupting the column
/// index of filters intended for later columns (e.g. rewriting `products.id@5` to `users.id@2`).
///
/// Since schema-preserving nodes do not modify the schema or column positions, every column index
/// in `parent_filters` is already valid and refers to the exact intended column in the child.
/// This function verifies that all referenced columns match the schema at their current indices
/// without altering them.
pub fn schema_preserving_child_filter_description(
    parent_filters: &[Arc<dyn PhysicalExpr>],
    schema: &arrow_schema::Schema,
    allowed_indices: Option<&std::collections::HashSet<usize>>,
) -> datafusion::common::Result<datafusion::physical_plan::filter_pushdown::ChildFilterDescription>
{
    use datafusion::common::tree_node::{TreeNode, TreeNodeRecursion};
    use datafusion::physical_expr::expressions::Column;
    use datafusion::physical_plan::ExecutionPlan;
    use datafusion::physical_plan::empty::EmptyExec;
    use datafusion::physical_plan::filter_pushdown::ChildFilterDescription;
    use std::collections::{HashMap, HashSet};

    // Collect referenced column indices for each column name in parent_filters
    let mut referenced_names: HashMap<String, usize> = HashMap::new();
    let mut conflicting_names: HashSet<String> = HashSet::new();

    for filter in parent_filters {
        let _ = filter.apply(|node| {
            if let Some(col) = node.downcast_ref::<Column>()
                && col.index() < schema.fields().len()
                && schema.field(col.index()).name() == col.name()
            {
                match referenced_names.get(col.name()) {
                    Some(&existing_idx) if existing_idx != col.index() => {
                        conflicting_names.insert(col.name().to_string());
                    }
                    None => {
                        referenced_names.insert(col.name().to_string(), col.index());
                    }
                    _ => {}
                }
            }
            Ok(TreeNodeRecursion::Continue)
        });
    }

    // Build a disambiguated schema where duplicate column names that are not the target
    // index for a referenced column are given unique dummy names so `index_of` returns
    // the exact referenced index.
    let disambiguated_fields: Vec<Arc<arrow_schema::Field>> = schema
        .fields()
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            if let Some(&target_idx) = referenced_names.get(field.name())
                && !conflicting_names.contains(field.name())
                && target_idx != idx
            {
                return Arc::new(
                    field
                        .as_ref()
                        .clone()
                        .with_name(format!("{}__shadowed_{idx}", field.name())),
                );
            }
            Arc::clone(field)
        })
        .collect();

    let disambiguated_schema = Arc::new(arrow_schema::Schema::new(disambiguated_fields));
    let dummy_child = Arc::new(EmptyExec::new(disambiguated_schema)) as Arc<dyn ExecutionPlan>;

    let allowed: HashSet<usize> = match allowed_indices {
        Some(indices) => indices
            .iter()
            .copied()
            .filter(|&i| i < schema.fields().len())
            .collect(),
        None => (0..schema.fields().len()).collect(),
    };

    ChildFilterDescription::from_child_with_allowed_indices(parent_filters, allowed, &dummy_child)
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

    #[test]
    fn test_schema_preserving_child_filter_description_duplicate_names() {
        use arrow_schema::{DataType, Field, Schema};
        use datafusion::physical_expr::expressions::Column;
        use datafusion::physical_plan::filter_pushdown::{FilterDescription, PushedDown};

        // A join schema with duplicate column names: "id" at index 2 and index 5
        let schema = Schema::new(vec![
            Field::new("ctid_1", DataType::UInt64, false),
            Field::new("tags", DataType::Utf8, true),
            Field::new("id", DataType::Int64, false),
            Field::new("ctid_2", DataType::UInt64, false),
            Field::new("tags", DataType::Utf8, true),
            Field::new("id", DataType::Int64, false),
        ]);

        let col_id_5 =
            Arc::new(Column::new("id", 5)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>;
        let col_tags_1 =
            Arc::new(Column::new("tags", 1)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>;
        let col_invalid_name =
            Arc::new(Column::new("wrong", 5)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>;
        let col_out_of_bounds =
            Arc::new(Column::new("id", 10)) as Arc<dyn datafusion::physical_expr::PhysicalExpr>;

        let filters = vec![
            Arc::clone(&col_id_5),
            Arc::clone(&col_tags_1),
            col_invalid_name,
            col_out_of_bounds,
        ];

        let desc = schema_preserving_child_filter_description(&filters, &schema, None).unwrap();
        let parent_filters = FilterDescription::new().with_child(desc).parent_filters();

        // Verify index 5 is preserved as supported and NOT remapped to index 2
        assert!(matches!(parent_filters[0][0].discriminant, PushedDown::Yes));
        let preserved_col = parent_filters[0][0]
            .predicate
            .downcast_ref::<Column>()
            .unwrap();
        assert_eq!(preserved_col.name(), "id");
        assert_eq!(preserved_col.index(), 5);

        // Verify tags@1 is preserved
        assert!(matches!(parent_filters[0][1].discriminant, PushedDown::Yes));
        let preserved_col2 = parent_filters[0][1]
            .predicate
            .downcast_ref::<Column>()
            .unwrap();
        assert_eq!(preserved_col2.name(), "tags");
        assert_eq!(preserved_col2.index(), 1);

        // Verify name mismatch and out of bounds are unsupported
        assert!(matches!(parent_filters[0][2].discriminant, PushedDown::No));
        assert!(matches!(parent_filters[0][3].discriminant, PushedDown::No));

        // With allowed_indices: index 5 is blocked
        let mut allowed = std::collections::HashSet::new();
        allowed.insert(1);
        let desc_allowed =
            schema_preserving_child_filter_description(&filters, &schema, Some(&allowed)).unwrap();
        let parent_filters_allowed = FilterDescription::new()
            .with_child(desc_allowed)
            .parent_filters();
        assert!(matches!(
            parent_filters_allowed[0][0].discriminant,
            PushedDown::No
        ));
        assert!(matches!(
            parent_filters_allowed[0][1].discriminant,
            PushedDown::Yes
        ));
    }
}
