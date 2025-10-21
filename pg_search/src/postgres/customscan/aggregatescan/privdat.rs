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

use crate::api::operator::anyelement_query_input_opoid;
use crate::api::AsCStr;
use crate::customscan::aggregatescan::aggregations::AggregateCSClause;
use crate::customscan::aggregatescan::CustomScanClause;
use crate::customscan::builders::custom_path::RestrictInfoType;
use crate::customscan::solve_expr::SolvePostgresExpressions;
use crate::nodecast;
use crate::postgres::customscan::explain::ExplainFormat;
use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};
use crate::postgres::types::{ConstNode, TantivyValue};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
use pgrx::pg_sys::AsPgCStr;
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
use tantivy::aggregation::metric::SingleMetricResult;
use tantivy::schema::OwnedValue;

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
}

impl ExplainFormat for AggregateType {
    fn explain_format(&self) -> String {
        todo!()
        // self.format_aggregate()
    }
}

impl SolvePostgresExpressions for AggregateType {
    fn has_heap_filters(&mut self) -> bool {
        self.filter_expr_mut()
            .as_mut()
            .is_some_and(|filter| filter.has_heap_filters())
    }

    fn has_postgres_expressions(&mut self) -> bool {
        self.filter_expr_mut()
            .as_mut()
            .is_some_and(|filter| filter.has_postgres_expressions())
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

// TODO: We should use Tantivy's native aggregate types (CountAggregation, SumAggregation, etc.)
// which implement serde, but the current fork/version produces incorrect JSON structure.
// The Tantivy types serialize to {"field": "name", "missing": null} instead of
// the expected {"aggregation_type": {"field": "name"}} format.
// https://docs.rs/tantivy/latest/tantivy/aggregation/metric/struct.CountAggregation.html
impl AggregateType {
    pub unsafe fn try_from(
        aggref: *mut pg_sys::Aggref,
        heaprelid: pg_sys::Oid,
        bm25_index: &crate::postgres::PgSearchRelation,
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
        qual_state: &mut QualExtractState,
    ) -> Option<Self> {
        let aggfnoid = (*aggref).aggfnoid.to_u32();
        let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);

        let filter_expr = if (*aggref).aggfilter.is_null() {
            None
        } else {
            extract_quals(
                root,
                heap_rti,
                (*aggref).aggfilter as *mut pg_sys::Node,
                anyelement_query_input_opoid(),
                RestrictInfoType::BaseRelation,
                bm25_index,
                false,
                qual_state,
                true,
            )
        };
        let filter_query = filter_expr.map(|qual| SearchQueryInput::from(&qual));

        if aggfnoid == F_COUNT_ && (*aggref).aggstar {
            return Some(AggregateType::CountAny {
                filter: filter_query,
                indexrelid: bm25_index.oid(),
            });
        }

        if args.is_empty() {
            return None;
        }

        let first_arg = args.get_ptr(0)?;
        let (field, missing) = parse_aggregate_field(first_arg, heaprelid)?;
        let agg_type =
            create_aggregate_from_oid(aggfnoid, field, missing, filter_query, bm25_index.oid())?;

        Some(agg_type)
    }

    pub fn can_use_doc_count(&self) -> bool {
        matches!(self, AggregateType::CountAny { .. }) && !self.has_filter()
    }

    /// Get the field name for field-based aggregates (None for COUNT)
    pub fn field_name(&self) -> Option<String> {
        match self {
            AggregateType::CountAny { .. } => None,
            AggregateType::Count { field, .. } => Some(field.clone()),
            AggregateType::Sum { field, .. } => Some(field.clone()),
            AggregateType::Avg { field, .. } => Some(field.clone()),
            AggregateType::Min { field, .. } => Some(field.clone()),
            AggregateType::Max { field, .. } => Some(field.clone()),
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
            | AggregateType::Max { .. } => SingleMetricResult { value: None },
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
        }
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
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrivateData {
    pub indexrelid: pg_sys::Oid,
    pub heap_rti: pg_sys::Index,
    pub aggregate_clause: AggregateCSClause,
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe {
            let list = PgList::<pg_sys::Node>::from_pg(list);
            let node = list.get_ptr(0).unwrap();
            let content = node
                .as_c_str()
                .unwrap()
                .to_str()
                .expect("string node should be valid utf8");
            serde_json::from_str(content).unwrap()
        }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        let content = serde_json::to_string(&value).unwrap();
        unsafe {
            let mut ser = PgList::new();
            ser.push(pg_sys::makeString(content.as_pg_cstr()).cast::<pg_sys::Node>());
            ser.into_pg()
        }
    }
}

trait F64Lossless {
    fn to_f64_lossless(self) -> Option<f64>;
}

impl F64Lossless for u64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        if f as u64 == self {
            Some(f)
        } else {
            None
        }
    }
}

impl F64Lossless for i64 {
    fn to_f64_lossless(self) -> Option<f64> {
        let f = self as f64;
        if f as i64 == self {
            Some(f)
        } else {
            None
        }
    }
}

/// Parse field name and missing value from aggregate argument
unsafe fn parse_aggregate_field(
    first_arg: *mut pg_sys::TargetEntry,
    heaprelid: pg_sys::Oid,
) -> Option<(String, Option<f64>)> {
    let (var, missing) =
        if let Some(coalesce_node) = nodecast!(CoalesceExpr, T_CoalesceExpr, (*first_arg).expr) {
            parse_coalesce_expression(coalesce_node)?
        } else if let Some(var) = nodecast!(Var, T_Var, (*first_arg).expr) {
            (var, None)
        } else {
            return None;
        };

    let field = fieldname_from_var(heaprelid, var, (*var).varattno)?.into_inner();
    Some((field, missing))
}

/// Parse COALESCE expression to extract variable and missing value
unsafe fn parse_coalesce_expression(
    coalesce_node: *mut pg_sys::CoalesceExpr,
) -> Option<(*mut pg_sys::Var, Option<f64>)> {
    let args = PgList::<pg_sys::Node>::from_pg((*coalesce_node).args);
    if args.is_empty() {
        return None;
    }

    let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
    let const_node = ConstNode::try_from(args.get_ptr(1)?)?;
    let missing = match TantivyValue::try_from(const_node) {
        Ok(TantivyValue(OwnedValue::U64(missing))) => missing.to_f64_lossless(),
        Ok(TantivyValue(OwnedValue::I64(missing))) => missing.to_f64_lossless(),
        Ok(TantivyValue(OwnedValue::F64(missing))) => Some(missing),
        Ok(TantivyValue(OwnedValue::Null)) => None,
        _ => return None,
    };

    Some((var, missing))
}

/// Create appropriate AggregateType from function OID
fn create_aggregate_from_oid(
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
