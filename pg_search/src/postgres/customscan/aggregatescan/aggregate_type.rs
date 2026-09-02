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

use crate::api::{FieldName, HashSet, MvccVisibility, is_agg_funcoid, visibility_from_agg_arg};
use crate::customscan::builders::custom_path::RestrictInfoType;
use crate::customscan::solve_expr::SolvePostgresExpressions;
use crate::nodecast;
use crate::postgres::PgSearchRelation;
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::opexpr::UnwrapFromExpr;
use crate::postgres::customscan::qual_inspect::{PlannerContext, QualExtractState, extract_quals};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::types::{ConstNode, TantivyValue};
use crate::postgres::var::{VarContext, fieldname_from_var, find_one_var_and_fieldname};
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use pgrx::PgList;
use pgrx::pg_sys::{
    F_AVG_FLOAT4, F_AVG_FLOAT8, F_AVG_INT2, F_AVG_INT4, F_AVG_INT8, F_AVG_NUMERIC, F_COUNT_,
    F_COUNT_ANY, F_MAX_DATE, F_MAX_FLOAT4, F_MAX_FLOAT8, F_MAX_INT2, F_MAX_INT4, F_MAX_INT8,
    F_MAX_NUMERIC, F_MAX_TIME, F_MAX_TIMESTAMP, F_MAX_TIMESTAMPTZ, F_MAX_TIMETZ, F_MIN_DATE,
    F_MIN_FLOAT4, F_MIN_FLOAT8, F_MIN_INT2, F_MIN_INT4, F_MIN_INT8, F_MIN_MONEY, F_MIN_NUMERIC,
    F_MIN_TIME, F_MIN_TIMESTAMP, F_MIN_TIMESTAMPTZ, F_MIN_TIMETZ, F_SUM_FLOAT4, F_SUM_FLOAT8,
    F_SUM_INT2, F_SUM_INT4, F_SUM_INT8, F_SUM_NUMERIC,
};
use pgrx::prelude::*;
use tantivy::aggregation::agg_req::AggregationVariants;
use tantivy::aggregation::metric::{
    AverageAggregation, CountAggregation, MaxAggregation, MinAggregation, SingleMetricResult,
    SumAggregation,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggregateType {
    CountAny {
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Count {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Sum {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Avg {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Min {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Max {
        field: String,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    },
    Custom {
        agg_json: serde_json::Value,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
        mvcc_visibility: MvccVisibility,
    },
}

impl SolvePostgresExpressions for AggregateType {
    fn has_postgres_expressions(&mut self) -> bool {
        self.filter_expr_mut()
            .as_mut()
            .is_some_and(|filter| filter.has_postgres_expressions())
    }

    fn has_parameters(&mut self) -> bool {
        self.filter_expr_mut()
            .as_mut()
            .is_some_and(|filter| filter.has_parameters())
    }

    fn init_postgres_expressions(&mut self, planstate: *mut pg_sys::PlanState) {
        if let Some(filter) = self.filter_expr_mut() {
            filter.init_postgres_expressions(planstate);
        }
    }

    fn solve_postgres_expressions(&mut self, expr_context: *mut pg_sys::ExprContext) {
        if let Some(filter) = self.filter_expr_mut() {
            filter.solve_postgres_expressions(expr_context);
        }
    }
}

impl AggregateType {
    pub unsafe fn try_from(
        aggref: *mut pg_sys::Aggref,
        bm25_index: &PgSearchRelation,
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
        qual_state: &mut QualExtractState,
    ) -> Result<Self, String> {
        let aggfnoid = (*aggref).aggfnoid.to_u32();

        let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);

        let filter_expr = if (*aggref).aggfilter.is_null() {
            None
        } else {
            let context = PlannerContext::from_planner(root);
            extract_quals(
                &context,
                heap_rti,
                (*aggref).aggfilter as *mut pg_sys::Node,
                RestrictInfoType::BaseRelation,
                bm25_index,
                false,
                qual_state,
                true,
            )
        };
        let filter_query = filter_expr.map(|qual| SearchQueryInput::from(&qual));

        // Check for pdb.agg() custom aggregate (any overload)
        if is_agg_funcoid(aggfnoid) {
            // Extract JSON argument (first arg)
            let arg = args.get_ptr(0).expect("pdb.agg missing argument");
            let expr = (*arg).expr;
            let json_value = if let Some(const_node) = nodecast!(Const, T_Const, expr) {
                let json_datum = (*const_node).constvalue;
                pgrx::JsonB::from_datum(json_datum, false)
                    .expect("invalid JSON in pdb.agg")
                    .0
            } else {
                // Parameterized pdb.agg() can't be lowered into the aggregate
                // pushdown plan because we need the JSON spec at planning time
                // to validate fields and choose a strategy. Return Err so the
                // AggregateScan path declines and PG falls back to standard
                // aggregate processing — same behaviour as a query without
                // pdb.agg() pushdown.
                return Err("pdb.agg argument must be a constant for aggregate pushdown".into());
            };

            // Decode the visibility argument (second arg) of the two-arg overloads;
            // the one-arg overload has none and takes the default.
            let mvcc_visibility = visibility_from_agg_arg(
                aggfnoid,
                args.get_ptr(1).map(|arg| (*arg).expr as *mut pg_sys::Node),
            );

            // Check if any existing fields in the custom aggregate are NUMERIC
            // NUMERIC fields do not support aggregate pushdown
            // Note: Non-existent fields are caught by validate_fields() with proper error
            let schema = bm25_index.schema().expect("could not get index schema");
            let mut fields = HashSet::default();
            extract_fields_from_agg_json(&json_value, &mut fields);
            for field_name in &fields {
                // Only check NUMERIC support if field exists in schema
                if schema.search_field(field_name).is_some()
                    && !schema.supports_tantivy_aggregate(field_name)
                {
                    return Err(format!(
                        "field '{}' does not support aggregate pushdown (NUMERIC)",
                        field_name
                    ));
                }
            }

            return Ok(AggregateType::Custom {
                agg_json: json_value,
                filter: filter_query,
                indexrelid: bm25_index.oid(),
                mvcc_visibility,
            });
        }

        if aggfnoid == F_COUNT_ && (*aggref).aggstar {
            return Ok(AggregateType::CountAny {
                filter: filter_query,
                indexrelid: bm25_index.oid(),
            });
        }

        if args.is_empty() {
            return Err("aggregate missing arguments".into());
        }

        let first_arg = args.get_ptr(0).ok_or("aggregate missing argument")?;
        let (field, missing) = parse_aggregate_field(first_arg, root, bm25_index, heap_rti)?;

        // Check if aggregate pushdown is supported for this field type on the
        // Tantivy backend. NUMERIC fields are not supported here; standard SQL
        // aggregates over them route to the DataFusion backend at path
        // creation time and never reach this classifier.
        if !bm25_index
            .supports_tantivy_aggregate(&field)
            .unwrap_or(false)
        {
            return Err(format!(
                "field '{}' does not support aggregate pushdown",
                field
            ));
        }

        let agg_type =
            create_aggregate_from_oid(aggfnoid, field, missing, filter_query, bm25_index.oid())
                .ok_or_else(|| {
                    if let Some(n) = crate::postgres::catalog::lookup_fully_qualified_func_name(
                        pg_sys::Oid::from(aggfnoid),
                    ) {
                        format!("unsupported aggregate function: {}", n)
                    } else {
                        format!("unsupported aggregate function OID: {}", aggfnoid)
                    }
                })?;

        Ok(agg_type)
    }

    pub fn can_use_doc_count(&self) -> bool {
        matches!(self, AggregateType::CountAny { .. }) && !self.has_filter()
    }

    /// Get the field name for field-based aggregates (None for COUNT and Custom)
    pub fn field_name(&self) -> Option<String> {
        match self {
            AggregateType::CountAny { .. } => None,
            AggregateType::Count { field, .. } => Some(field.clone()),
            AggregateType::Sum { field, .. } => Some(field.clone()),
            AggregateType::Avg { field, .. } => Some(field.clone()),
            AggregateType::Min { field, .. } => Some(field.clone()),
            AggregateType::Max { field, .. } => Some(field.clone()),
            AggregateType::Custom { .. } => None,
        }
    }

    pub fn indexrelid(&self) -> pg_sys::Oid {
        match self {
            AggregateType::CountAny { indexrelid, .. } => *indexrelid,
            AggregateType::Count { indexrelid, .. } => *indexrelid,
            AggregateType::Sum { indexrelid, .. } => *indexrelid,
            AggregateType::Avg { indexrelid, .. } => *indexrelid,
            AggregateType::Min { indexrelid, .. } => *indexrelid,
            AggregateType::Max { indexrelid, .. } => *indexrelid,
            AggregateType::Custom { indexrelid, .. } => *indexrelid,
        }
    }

    pub fn missing(&self) -> Option<f64> {
        match self {
            AggregateType::CountAny { .. } => None,
            AggregateType::Count { missing, .. } => *missing,
            AggregateType::Sum { missing, .. } => *missing,
            AggregateType::Avg { missing, .. } => *missing,
            AggregateType::Min { missing, .. } => *missing,
            AggregateType::Max { missing, .. } => *missing,
            AggregateType::Custom { .. } => None,
        }
    }

    pub fn nullish(&self) -> SingleMetricResult {
        match self {
            AggregateType::CountAny { .. } | AggregateType::Count { .. } => {
                SingleMetricResult { value: Some(0.0) }
            }
            AggregateType::Sum { .. }
            | AggregateType::Avg { .. }
            | AggregateType::Min { .. }
            | AggregateType::Max { .. }
            | AggregateType::Custom { .. } => SingleMetricResult { value: None },
        }
    }

    /// Check if this aggregate has a filter
    pub fn has_filter(&self) -> bool {
        match self {
            AggregateType::CountAny { filter, .. } => filter.is_some(),
            AggregateType::Count { filter, .. } => filter.is_some(),
            AggregateType::Sum { filter, .. } => filter.is_some(),
            AggregateType::Avg { filter, .. } => filter.is_some(),
            AggregateType::Min { filter, .. } => filter.is_some(),
            AggregateType::Max { filter, .. } => filter.is_some(),
            AggregateType::Custom { filter, .. } => filter.is_some(),
        }
    }

    /// Get the filter expression if present
    pub fn filter_expr(&self) -> &Option<SearchQueryInput> {
        match self {
            AggregateType::CountAny { filter, .. } => filter,
            AggregateType::Count { filter, .. } => filter,
            AggregateType::Sum { filter, .. } => filter,
            AggregateType::Avg { filter, .. } => filter,
            AggregateType::Min { filter, .. } => filter,
            AggregateType::Max { filter, .. } => filter,
            AggregateType::Custom { filter, .. } => filter,
        }
    }

    pub fn filter_expr_mut(&mut self) -> &mut Option<SearchQueryInput> {
        match self {
            AggregateType::CountAny { filter, .. } => filter,
            AggregateType::Count { filter, .. } => filter,
            AggregateType::Sum { filter, .. } => filter,
            AggregateType::Avg { filter, .. } => filter,
            AggregateType::Min { filter, .. } => filter,
            AggregateType::Max { filter, .. } => filter,
            AggregateType::Custom { filter, .. } => filter,
        }
    }

    /// Get the visibility setting for this aggregate.
    /// Only Custom aggregates (pdb.agg) can have a non-default setting.
    /// All standard SQL aggregates (COUNT, SUM, etc.) use the default (Transaction).
    pub fn mvcc_visibility(&self) -> MvccVisibility {
        match self {
            AggregateType::Custom {
                mvcc_visibility, ..
            } => *mvcc_visibility,
            // Standard SQL aggregates always use default MVCC behavior
            _ => MvccVisibility::default(),
        }
    }

    /// Determines the single query-level visibility setting for a group of aggregates.
    ///
    /// A query resolves to exactly one visibility decision, so every `pdb.agg()` call in
    /// it has to agree. Two different settings are an error even when one of them came
    /// from an omitted argument, since an omitted argument is indistinguishable from an
    /// explicit `'transaction'` by the time we see it.
    pub fn resolve_visibility<'a>(
        aggregates: impl Iterator<Item = &'a AggregateType>,
    ) -> MvccVisibility {
        let mut resolved: Option<MvccVisibility> = None;
        for visibility in aggregates.filter_map(|agg_type| match agg_type {
            AggregateType::Custom {
                mvcc_visibility, ..
            } => Some(*mvcc_visibility),
            // Standard SQL aggregates carry no setting of their own.
            _ => None,
        }) {
            match resolved {
                None => resolved = Some(visibility),
                Some(previous) if previous == visibility => {}
                Some(previous) => pgrx::error!(
                    "pdb.agg() calls have contradicting visibility settings: \
                     '{}' and '{}'. All pdb.agg() calls in a query must use the same \
                     visibility value, and omitting the argument selects '{}'.",
                    previous.as_sql_value(),
                    visibility.as_sql_value(),
                    MvccVisibility::default().as_sql_value()
                ),
            }
        }

        resolved.unwrap_or_default()
    }

    pub fn result_type_oid(&self) -> pg_sys::Oid {
        match &self {
            AggregateType::CountAny { .. } | AggregateType::Count { .. } => pg_sys::INT8OID,
            AggregateType::Sum { .. }
            | AggregateType::Avg { .. }
            | AggregateType::Min { .. }
            | AggregateType::Max { .. } => pg_sys::FLOAT8OID,
            AggregateType::Custom { .. } => pg_sys::JSONBOID,
        }
    }

    /// Validate that fields referenced by this aggregate exist in the schema
    /// and are supported for aggregate pushdown.
    ///
    /// Returns an error if:
    /// - Any referenced field doesn't exist in the index
    /// - Any referenced field is a NUMERIC type (not supported for aggregation)
    ///
    /// TODO: remove field existence check once Tantivy aggregation validation is fixed.
    /// <https://github.com/quickwit-oss/tantivy/issues/2767>
    pub fn validate_fields(&self, schema: &SearchIndexSchema) -> Result<(), String> {
        // Check NUMERIC field support for standard aggregates
        if let Some(field) = self.field_name()
            && !schema.supports_tantivy_aggregate(&field)
        {
            return Err(format!(
                "Aggregate on NUMERIC field '{}' cannot be pushed down. \
                     NUMERIC columns do not support aggregate pushdown.",
                field
            ));
        }

        // For Custom aggregates, validate field existence and NUMERIC support
        if let AggregateType::Custom { agg_json, .. } = self {
            validate_agg_json_fields(agg_json, schema)?;
        }
        Ok(())
    }

    pub fn custom_agg_json(&self) -> Option<&serde_json::Value> {
        if let Self::Custom { agg_json, .. } = self {
            Some(agg_json)
        } else {
            None
        }
    }
}

/// Validate that all fields referenced in a JSON aggregation request exist in the
/// index schema and are supported for aggregate pushdown.
///
/// Returns an error if:
/// - Any referenced field doesn't exist in the index
/// - Any referenced field is a NUMERIC type (not supported for aggregation)
/// - Any `top_hits.sort` key has a type Tantivy's sort accessor does not support
///   (only `I64` / `U64` / `F64` / `Date` / `Numeric64` are accepted)
pub(crate) fn validate_agg_json_fields(
    agg_json: &serde_json::Value,
    schema: &SearchIndexSchema,
) -> Result<(), String> {
    let mut fields = HashSet::default();
    extract_fields_from_agg_json(agg_json, &mut fields);
    // top_hits.sort keys are object keys inside the sort array rather than values under a
    // "field" key, so extract_fields_from_agg_json will not see them. Collect them here so
    // the existence check below covers them and validate_top_hits_sort_fields can rely on
    // schema.get_field_type() returning Some.
    collect_top_hits_sort_field_names(agg_json, &mut fields);
    let indexed_fields: HashSet<String> = schema
        .fields()
        .map(|(_, entry)| entry.name().to_string())
        .collect();

    for field in &fields {
        // Check field exists
        if !indexed_fields.contains(field) {
            let mut available: Vec<_> = indexed_fields
                .iter()
                .filter(|f| *f != "ctid")
                .cloned()
                .collect();
            available.sort();
            return Err(format!(
                "Aggregation references invalid field '{}'. Available indexed fields are: [{}]",
                field,
                available.join(", ")
            ));
        }
        // Check NUMERIC support
        if !schema.supports_tantivy_aggregate(field) {
            return Err(format!(
                "Aggregation references NUMERIC field '{}' which cannot be aggregated. \
                 NUMERIC columns do not support aggregate pushdown.",
                field
            ));
        }
    }

    validate_top_hits_sort_fields(agg_json, schema)?;

    Ok(())
}

/// Recursively walk `agg_json` and validate that every `top_hits.sort` field is a type
/// Tantivy's sort accessor supports (see [`crate::schema::SearchFieldType::supports_top_hits_sort`]).
///
/// A text / uuid / inet / ltree / json / range / vector sort key would fall back to an empty
/// accessor and every hit would silently get `"sort": [null]` with no ordering applied
/// (issue #5710). Raising a clear planning-time error is friendlier than silent wrong
/// results.
///
/// Elasticsearch-style pseudo fields prefixed with `_` (`_score`, `_doc`) are skipped:
/// they do not resolve to schema fields and have their own accessor path in Tantivy.
fn validate_top_hits_sort_fields(
    agg_json: &serde_json::Value,
    schema: &SearchIndexSchema,
) -> Result<(), String> {
    match agg_json {
        serde_json::Value::Object(map) => {
            if let Some(sort) = map
                .get("top_hits")
                .and_then(|v| v.as_object())
                .and_then(|top_hits| top_hits.get("sort"))
                .and_then(|v| v.as_array())
            {
                for entry in sort {
                    let Some(sort_obj) = entry.as_object() else {
                        continue;
                    };
                    for field_name in sort_obj.keys() {
                        if field_name.starts_with('_') {
                            continue;
                        }
                        let root = FieldName::from(field_name.as_str()).root();
                        // Existence is guaranteed by the fields loop in
                        // validate_agg_json_fields (which now includes top_hits.sort keys via
                        // collect_top_hits_sort_field_names). Panic loudly if that invariant
                        // is ever broken so the failure is not silent.
                        let field_type = schema.get_field_type(&root).expect(
                            "top_hits.sort field existence should have been validated by the \
                             indexed_fields loop in validate_agg_json_fields",
                        );
                        if !field_type.supports_top_hits_sort() {
                            return Err(format!(
                                "top_hits.sort field '{}' has an unsupported type for sorting. \
                                 Only numeric and date fields can be used as top_hits.sort keys.",
                                field_name
                            ));
                        }
                    }
                }
            }

            for value in map.values() {
                validate_top_hits_sort_fields(value, schema)?;
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                validate_top_hits_sort_fields(item, schema)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Recursively walk `json` and add every `top_hits.sort` field key to `fields`. Sort keys
/// appear as object keys inside the sort array (`{"field_name": "asc"}`), so
/// [`extract_fields_from_agg_json`] does not see them. Elasticsearch-style pseudo fields
/// (`_score`, `_doc`) are skipped since they do not resolve to schema fields.
fn collect_top_hits_sort_field_names(json: &serde_json::Value, fields: &mut HashSet<String>) {
    match json {
        serde_json::Value::Object(map) => {
            if let Some(sort) = map
                .get("top_hits")
                .and_then(|v| v.as_object())
                .and_then(|top_hits| top_hits.get("sort"))
                .and_then(|v| v.as_array())
            {
                for entry in sort {
                    let Some(sort_obj) = entry.as_object() else {
                        continue;
                    };
                    for field_name in sort_obj.keys() {
                        if field_name.starts_with('_') {
                            continue;
                        }
                        let field_name = FieldName::from(field_name.as_str());
                        fields.insert(field_name.root());
                    }
                }
            }
            for value in map.values() {
                collect_top_hits_sort_field_names(value, fields);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_top_hits_sort_field_names(item, fields);
            }
        }
        _ => {}
    }
}

fn extract_fields_from_agg_json(json: &serde_json::Value, fields: &mut HashSet<String>) {
    match json {
        serde_json::Value::Object(map) => {
            // Check for a "field" key at this level
            if let Some(serde_json::Value::String(f)) = map.get("field") {
                let field_name = FieldName::from(f);
                fields.insert(field_name.root());
            }

            // Recurse into all values
            for value in map.values() {
                extract_fields_from_agg_json(value, fields);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                extract_fields_from_agg_json(item, fields);
            }
        }
        _ => {}
    }
}

impl std::fmt::Display for AggregateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregateType::CountAny { .. } => write!(f, "COUNT(*)"),
            AggregateType::Count { .. } => write!(f, "COUNT({})", self.field_name().unwrap()),
            AggregateType::Sum { .. } => write!(f, "SUM({})", self.field_name().unwrap()),
            AggregateType::Avg { .. } => write!(f, "AVG({})", self.field_name().unwrap()),
            AggregateType::Min { .. } => write!(f, "MIN({})", self.field_name().unwrap()),
            AggregateType::Max { .. } => write!(f, "MAX({})", self.field_name().unwrap()),
            AggregateType::Custom { agg_json, .. } => write!(f, "CUSTOM_AGG({})", agg_json),
        }
    }
}

impl From<AggregateType> for AggregationVariants {
    fn from(val: AggregateType) -> Self {
        match val {
            AggregateType::CountAny { .. } => AggregationVariants::Count(CountAggregation {
                field: "ctid".to_string(),
                missing: None,
            }),
            AggregateType::Count { field, missing, .. } => {
                AggregationVariants::Count(CountAggregation { field, missing })
            }
            AggregateType::Sum { field, missing, .. } => AggregationVariants::Sum(SumAggregation {
                field,
                missing,
                none_if_no_match: Some(true),
            }),
            AggregateType::Avg { field, missing, .. } => {
                AggregationVariants::Average(AverageAggregation { field, missing })
            }
            AggregateType::Min { field, missing, .. } => {
                AggregationVariants::Min(MinAggregation { field, missing })
            }
            AggregateType::Max { field, missing, .. } => {
                AggregationVariants::Max(MaxAggregation { field, missing })
            }
            AggregateType::Custom { agg_json, .. } => {
                // For Custom aggregates, deserialize the JSON directly into AggregationVariants
                serde_json::from_value(agg_json)
                    .unwrap_or_else(|e| panic!("Failed to deserialize custom aggregate: {}", e))
            }
        }
    }
}

trait F64Lossless {
    fn to_f64_lossless(self) -> Option<f64>;
}

impl F64Lossless for u64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        if f as u64 == self { Some(f) } else { None }
    }
}

impl F64Lossless for i64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        if f as i64 == self { Some(f) } else { None }
    }
}

pub(crate) enum ParsedAggregateField {
    Direct(*mut pg_sys::Node),
    Coalesce {
        field: *mut pg_sys::Node,
        missing: Option<f64>,
    },
}

impl ParsedAggregateField {
    pub(crate) unsafe fn parse(expr: *mut pg_sys::Node) -> Result<Self, String> {
        let Some(coalesce) = nodecast!(CoalesceExpr, T_CoalesceExpr, expr) else {
            return Ok(Self::Direct(expr));
        };

        let args = PgList::<pg_sys::Node>::from_pg((*coalesce).args);
        let field = args
            .get_ptr(0)
            .ok_or("COALESCE expression missing first argument")?;
        let missing = parse_coalesce_missing_value(&args)?;

        Ok(Self::Coalesce { field, missing })
    }

    pub(crate) fn expression(&self) -> *mut pg_sys::Node {
        match self {
            Self::Direct(field) | Self::Coalesce { field, .. } => *field,
        }
    }

    pub(crate) unsafe fn field_name(&self, context: VarContext) -> Option<FieldName> {
        let expression = self.expression();
        if let Some((_, field_name)) = find_one_var_and_fieldname(context, expression) {
            return Some(field_name);
        }

        let Self::Coalesce { .. } = self else {
            return None;
        };
        let var = <*mut pg_sys::Var>::unwrap_from_expr(expression as *mut pg_sys::Expr)?;
        let (heaprelid, varattno) = context.var_relation(var);
        fieldname_from_var(heaprelid, var, varattno)
    }

    pub(crate) fn missing(&self) -> Option<f64> {
        match self {
            Self::Direct(_) => None,
            Self::Coalesce { missing, .. } => *missing,
        }
    }
}

/// Parse field name and missing value from aggregate argument
unsafe fn parse_aggregate_field(
    first_arg: *mut pg_sys::TargetEntry,
    root: *mut pg_sys::PlannerInfo,
    bm25_index: &PgSearchRelation,
    heap_rti: pg_sys::Index,
) -> Result<(String, Option<f64>), String> {
    let context = VarContext::from_planner(root);
    let aggregate_field = ParsedAggregateField::parse((*first_arg).expr.cast())?;
    let field_expr = aggregate_field.expression();

    let field = if let Ok(schema) = bm25_index.schema()
        && let Some(fast_field) = find_matching_fast_field(
            field_expr,
            &bm25_index.index_expressions(),
            schema,
            heap_rti,
        ) {
        FieldName::from(fast_field.name())
    } else {
        aggregate_field.field_name(context).ok_or(
            "argument to aggregate function is neither a direct column reference nor a COALESCE expression",
        )?
    };

    Ok((field.into_inner(), aggregate_field.missing()))
}

unsafe fn parse_coalesce_missing_value(args: &PgList<pg_sys::Node>) -> Result<Option<f64>, String> {
    let second_arg = args
        .get_ptr(1)
        .ok_or("COALESCE expression missing second argument")?;
    let const_node = ConstNode::unwrap_from_expr(second_arg as *mut pg_sys::Expr)
        .ok_or("second argument of COALESCE must resolve to a constant")?;

    let missing = match TantivyValue::try_from(const_node) {
        Ok(TantivyValue(PdbOwnedValue::U64(missing))) => missing.to_f64_lossless(),
        Ok(TantivyValue(PdbOwnedValue::I64(missing))) => missing.to_f64_lossless(),
        Ok(TantivyValue(PdbOwnedValue::F64(missing))) => Some(missing),
        Ok(TantivyValue(PdbOwnedValue::Null)) => None,
        // Handle string values from NUMERIC - parse to f64 for missing value
        Ok(TantivyValue(PdbOwnedValue::Str(s))) => s
            .parse::<f64>()
            .ok()
            .map(Some)
            .ok_or("unsupported constant type in COALESCE default value")?,
        _ => return Err("unsupported constant type in COALESCE default value".into()),
    };

    Ok(missing)
}

/// Create appropriate AggregateType from function OID
pub fn create_aggregate_from_oid(
    aggfnoid: u32,
    field: String,
    missing: Option<f64>,
    filter: Option<SearchQueryInput>,
    indexrelid: pg_sys::Oid,
) -> Option<AggregateType> {
    match aggfnoid {
        F_COUNT_ANY => Some(AggregateType::Count {
            field,
            missing,
            filter,
            indexrelid,
        }),
        F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
            Some(AggregateType::Avg {
                field,
                missing,
                filter,
                indexrelid,
            })
        }
        F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
            Some(AggregateType::Sum {
                field,
                missing,
                filter,
                indexrelid,
            })
        }
        F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
        | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
            Some(AggregateType::Max {
                field,
                missing,
                filter,
                indexrelid,
            })
        }
        F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
        | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
        | F_MIN_NUMERIC => Some(AggregateType::Min {
            field,
            missing,
            filter,
            indexrelid,
        }),
        _ => {
            pgrx::debug1!("Unknown aggregate function OID: {}", aggfnoid);
            None
        }
    }
}
