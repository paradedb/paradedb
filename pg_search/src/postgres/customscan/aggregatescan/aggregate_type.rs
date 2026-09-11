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

use crate::api::{is_agg_funcoid, pdb_agg_spec, FieldName, HashSet, MvccVisibility};
use crate::customscan::builders::custom_path::RestrictInfoType;
use crate::customscan::solve_expr::SolvePostgresExpressions;
use crate::nodecast;
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::joinscan::build::lookup_base_rel_info;
use crate::postgres::customscan::opexpr::UnwrapFromExpr;
use crate::postgres::customscan::qual_inspect::{extract_quals, PlannerContext, QualExtractState};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::types::{ConstNode, TantivyValue};
use crate::postgres::var::{fieldname_from_var, find_one_var_and_fieldname, VarContext};
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::schema::SearchIndexSchema;
use anyhow::{bail, Context};
use pgrx::pg_sys::{
    F_AVG_FLOAT4, F_AVG_FLOAT8, F_AVG_INT2, F_AVG_INT4, F_AVG_INT8, F_AVG_NUMERIC, F_COUNT_,
    F_COUNT_ANY, F_MAX_DATE, F_MAX_FLOAT4, F_MAX_FLOAT8, F_MAX_INT2, F_MAX_INT4, F_MAX_INT8,
    F_MAX_NUMERIC, F_MAX_TIME, F_MAX_TIMESTAMP, F_MAX_TIMESTAMPTZ, F_MAX_TIMETZ, F_MIN_DATE,
    F_MIN_FLOAT4, F_MIN_FLOAT8, F_MIN_INT2, F_MIN_INT4, F_MIN_INT8, F_MIN_MONEY, F_MIN_NUMERIC,
    F_MIN_TIME, F_MIN_TIMESTAMP, F_MIN_TIMESTAMPTZ, F_MIN_TIMETZ, F_SUM_FLOAT4, F_SUM_FLOAT8,
    F_SUM_INT2, F_SUM_INT4, F_SUM_INT8, F_SUM_NUMERIC,
};
use pgrx::prelude::*;
use pgrx::PgList;
use tantivy::aggregation::agg_req::{Aggregation, AggregationVariants};
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
    ) -> anyhow::Result<Self> {
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
            // Without the spec the scan declines, and Postgres runs the
            // aggregate itself.
            let arg = args.get_ptr(0).expect("pdb.agg missing argument");
            let (mut json_value, mvcc_visibility) = pdb_agg_spec(
                aggfnoid,
                (*arg).expr as *mut pg_sys::Node,
                args.get_ptr(1).map(|arg| (*arg).expr as *mut pg_sys::Node),
            )
            .context("pdb.agg argument must be a constant for aggregate pushdown")?;
            let schema = bm25_index.schema().expect("could not get index schema");

            // A spec written for a join names its fields `alias.field`, and the
            // planner can reduce that join to this one relation. A qualifier
            // naming it is dropped so the spec runs the same on either path.
            if let Some((_, Some(alias), _)) = lookup_base_rel_info(root, heap_rti) {
                strip_relation_qualifier(&mut json_value, &alias, &schema);
            }

            // Check if any existing fields in the custom aggregate are NUMERIC
            // NUMERIC fields do not support aggregate pushdown
            // Note: Non-existent fields are caught by validate_fields() with proper error
            let mut fields = HashSet::default();
            extract_fields_from_agg_json(&json_value, &mut fields);
            for field_name in &fields {
                // Only check NUMERIC support if field exists in schema
                if schema.search_field(field_name).is_some()
                    && !schema.supports_tantivy_aggregate(field_name)
                {
                    bail!(
                        "field '{}' does not support aggregate pushdown (NUMERIC)",
                        field_name
                    );
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
            bail!("aggregate missing arguments");
        }

        let first_arg = args.get_ptr(0).context("aggregate missing argument")?;
        let aggregate_field = ParsedAggregateField::from_index(
            (*first_arg).expr.cast(),
            VarContext::from_planner(root),
            bm25_index,
            heap_rti,
        )?;
        let field = aggregate_field.field_name().clone();
        let missing = aggregate_field.missing()?;

        // Check if aggregate pushdown is supported for this field type on the
        // Tantivy backend. NUMERIC fields are not supported here; standard SQL
        // aggregates over them route to the DataFusion backend at path
        // creation time and never reach this classifier.
        if !bm25_index
            .supports_tantivy_aggregate(&field)
            .unwrap_or(false)
        {
            bail!("field '{}' does not support aggregate pushdown", field);
        }

        let agg_type = Self::from_oid(aggfnoid, field, missing, filter_query, bm25_index.oid())
            .with_context(|| {
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

    pub fn from_oid(
        aggfnoid: u32,
        field: FieldName,
        missing: Option<f64>,
        filter: Option<SearchQueryInput>,
        indexrelid: pg_sys::Oid,
    ) -> Option<Self> {
        let field = field.into_inner();

        match aggfnoid {
            F_COUNT_ANY => Some(Self::Count {
                field,
                missing,
                filter,
                indexrelid,
            }),
            F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
                Some(Self::Avg {
                    field,
                    missing,
                    filter,
                    indexrelid,
                })
            }
            F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
                Some(Self::Sum {
                    field,
                    missing,
                    filter,
                    indexrelid,
                })
            }
            F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
            | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
                Some(Self::Max {
                    field,
                    missing,
                    filter,
                    indexrelid,
                })
            }
            F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
            | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
            | F_MIN_NUMERIC => Some(Self::Min {
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

    /// Get the MVCC visibility setting for this aggregate.
    /// Only Custom aggregates (pdb.agg) can have non-default MVCC settings.
    /// All standard SQL aggregates (COUNT, SUM, etc.) use the default (Enabled).
    pub fn mvcc_visibility(&self) -> MvccVisibility {
        match self {
            AggregateType::Custom {
                mvcc_visibility, ..
            } => *mvcc_visibility,
            // Standard SQL aggregates always use default MVCC behavior
            _ => MvccVisibility::default(),
        }
    }

    /// Determines if MVCC filtering should be enabled for a group of aggregates.
    /// Standard SQL aggregates carry no setting of their own.
    pub fn resolve_mvcc_enabled<'a>(aggregates: impl Iterator<Item = &'a AggregateType>) -> bool {
        MvccVisibility::resolve_shared(aggregates.filter_map(|agg_type| match agg_type {
            AggregateType::Custom {
                mvcc_visibility, ..
            } => Some(*mvcc_visibility),
            _ => None,
        }))
        .should_filter()
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
        if let Some(field) = self.field_name() {
            if !schema.supports_tantivy_aggregate(&field) {
                return Err(format!(
                    "Aggregate on NUMERIC field '{}' cannot be pushed down. \
                     NUMERIC columns do not support aggregate pushdown.",
                    field
                ));
            }
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

/// Rewrite every `"field": "alias.name"` to `"name"` when `alias.name` is no
/// index field itself and `name` is.
fn strip_relation_qualifier(json: &mut serde_json::Value, alias: &str, schema: &SearchIndexSchema) {
    match json {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(field)) = map.get_mut("field") {
                let unqualified = schema
                    .search_field(FieldName::from(field.as_str()).root())
                    .is_none()
                    .then(|| field.split_once('.'))
                    .flatten()
                    .filter(|(prefix, rest)| {
                        *prefix == alias
                            && schema.search_field(FieldName::from(*rest).root()).is_some()
                    })
                    .map(|(_, rest)| rest.to_string());
                if let Some(rest) = unqualified {
                    *field = rest;
                }
            }
            for value in map.values_mut() {
                strip_relation_qualifier(value, alias, schema);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                strip_relation_qualifier(item, alias, schema);
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

/// The request node of an aggregate. A `pdb.agg()` spec carries its own `aggs`,
/// which only survive here and not in a bare [`AggregationVariants`].
impl From<AggregateType> for Aggregation {
    fn from(val: AggregateType) -> Self {
        match val {
            AggregateType::Custom { agg_json, .. } => serde_json::from_value(agg_json)
                .unwrap_or_else(|e| panic!("Failed to deserialize custom aggregate: {}", e)),
            other => Aggregation {
                agg: other.into(),
                sub_aggregation: Default::default(),
            },
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
        (f as u128 == u128::from(self)).then_some(f)
    }
}

impl F64Lossless for i64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        (f as i128 == i128::from(self)).then_some(f)
    }
}

/// A supported aggregate argument together with the Tantivy field it resolves to.
/// The resolved name may come from a direct column, a JSON subpath, or a matching indexed
/// expression, while the original expression is retained to derive `COALESCE` semantics.
pub(crate) struct ParsedAggregateField {
    expression: AggregateFieldExpression,
    field_name: FieldName,
}

impl ParsedAggregateField {
    pub(crate) unsafe fn from_query(
        expr: *mut pg_sys::Node,
        context: VarContext,
    ) -> anyhow::Result<Self> {
        let expression = AggregateFieldExpression::from_node(expr)?;
        let field_name = expression.field_name(context)?;

        Ok(Self {
            expression,
            field_name,
        })
    }

    pub(crate) unsafe fn from_index(
        expr: *mut pg_sys::Node,
        context: VarContext,
        bm25_index: &PgSearchRelation,
        heap_rti: pg_sys::Index,
    ) -> anyhow::Result<Self> {
        let expression = AggregateFieldExpression::from_node(expr)?;
        let field_expr = expression.field_expression();

        let fast_field = bm25_index.schema().ok().and_then(|schema| {
            find_matching_fast_field(
                field_expr,
                &bm25_index.index_expressions(),
                schema,
                heap_rti,
            )
        });
        let field_name = if let Some(fast_field) = fast_field {
            FieldName::from(fast_field.name())
        } else {
            expression.field_name(context)?
        };

        Ok(Self {
            expression,
            field_name,
        })
    }

    pub(crate) fn field_name(&self) -> &FieldName {
        &self.field_name
    }

    /// Returns the Tantivy `missing` value for `COALESCE(field, default)` aggregates.
    /// Tantivy substitutes this value when a document has no value for the field, preserving the
    /// SQL `COALESCE` behavior during aggregate pushdown. Direct fields and `COALESCE(..., NULL)`
    /// do not need a substitution and return `None`.
    pub(crate) unsafe fn missing(&self) -> anyhow::Result<Option<f64>> {
        let AggregateFieldExpression::Coalesce { default, .. } = &self.expression else {
            return Ok(None);
        };
        let const_node = ConstNode::unwrap_from_expr(*default as *mut pg_sys::Expr)
            .context("second argument of COALESCE must resolve to a constant")?;

        Ok(match TantivyValue::try_from(const_node) {
            Ok(TantivyValue(PdbOwnedValue::U64(missing))) => Some(
                missing
                    .to_f64_lossless()
                    .context("COALESCE default value cannot be represented losslessly as f64")?,
            ),
            Ok(TantivyValue(PdbOwnedValue::I64(missing))) => Some(
                missing
                    .to_f64_lossless()
                    .context("COALESCE default value cannot be represented losslessly as f64")?,
            ),
            Ok(TantivyValue(PdbOwnedValue::F64(missing))) => Some(missing),
            Ok(TantivyValue(PdbOwnedValue::Null)) => None,
            Ok(TantivyValue(PdbOwnedValue::Str(s))) => Some(
                s.parse::<f64>()
                    .context("unsupported constant type in COALESCE default value")?,
            ),
            _ => bail!("unsupported constant type in COALESCE default value"),
        })
    }
}

/// The SQL expression shapes supported as aggregate arguments.
/// `Coalesce` keeps the field and constant default separate so they can become the Tantivy field
/// and `missing` value independently.
enum AggregateFieldExpression {
    Direct(*mut pg_sys::Node),
    Coalesce {
        field: *mut pg_sys::Node,
        default: *mut pg_sys::Node,
    },
}

impl AggregateFieldExpression {
    unsafe fn from_node(expr: *mut pg_sys::Node) -> anyhow::Result<Self> {
        let Some(coalesce) = nodecast!(CoalesceExpr, T_CoalesceExpr, expr) else {
            return Ok(Self::Direct(expr));
        };

        let args = PgList::<pg_sys::Node>::from_pg((*coalesce).args);
        let field = args
            .get_ptr(0)
            .context("COALESCE expression missing first argument")?;
        let default = args
            .get_ptr(1)
            .context("COALESCE expression missing second argument")?;

        Ok(Self::Coalesce { field, default })
    }

    fn field_expression(&self) -> *mut pg_sys::Node {
        match self {
            Self::Direct(field) | Self::Coalesce { field, .. } => *field,
        }
    }

    unsafe fn field_name(&self, context: VarContext) -> anyhow::Result<FieldName> {
        let expression = self.field_expression();
        if let Some((_, field_name)) = find_one_var_and_fieldname(context, expression) {
            return Ok(field_name);
        }

        let Self::Coalesce { .. } = self else {
            bail!(
                "argument to aggregate function is neither a direct column reference nor a COALESCE expression"
            );
        };
        let var = <*mut pg_sys::Var>::unwrap_from_expr(expression as *mut pg_sys::Expr)
            .context("first argument of COALESCE must resolve to a field")?;
        let (heaprelid, varattno) = context.var_relation(var);
        fieldname_from_var(heaprelid, var, varattno)
            .context("first argument of COALESCE must resolve to a field")
    }
}

#[cfg(test)]
mod tests {
    use super::F64Lossless;

    #[test]
    fn test_f64_lossless_integer_boundaries() {
        for value in [0_u64, 1 << 53, (1 << 53) + 2, 1 << 63] {
            assert_eq!(value.to_f64_lossless(), Some(value as f64));
        }
        for value in [(1_u64 << 53) + 1, u64::MAX] {
            assert_eq!(value.to_f64_lossless(), None);
        }
        for value in [0_i64, 1 << 53, -(1 << 53), i64::MIN] {
            assert_eq!(value.to_f64_lossless(), Some(value as f64));
        }
        for value in [(1_i64 << 53) + 1, -((1 << 53) + 1), i64::MAX] {
            assert_eq!(value.to_f64_lossless(), None);
        }
    }
}
