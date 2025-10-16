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

//! Parser logic for extracting AggregateType from PostgreSQL structures

use super::privdat::AggregateType;
use crate::nodecast;
use crate::postgres::types::{ConstNode, TantivyValue};
use crate::postgres::var::fieldname_from_var;
use crate::query::SearchQueryInput;
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
use tantivy::schema::OwnedValue;

/// Lossless conversion trait for numeric types to f64
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

/// Parses PostgreSQL aggregate references into AggregateType
pub struct AggregateParser;

impl AggregateParser {
    /// Parse an aggregate reference from PostgreSQL planner structures
    ///
    /// Returns Some((aggregate_type, uses_search_operator)) or None if parsing fails
    pub unsafe fn from_postgres_aggref(
        aggref: *mut pg_sys::Aggref,
        heaprelid: pg_sys::Oid,
        bm25_index: &crate::postgres::PgSearchRelation,
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
    ) -> Option<(AggregateType, bool)> {
        let aggfnoid = (*aggref).aggfnoid.to_u32();
        let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);

        // Extract filter clause if present
        let (filter_expr, filter_uses_search_operator) =
            Self::extract_filter_clause(aggref, bm25_index, root, heap_rti);

        // Handle COUNT(*)
        if aggfnoid == F_COUNT_ && (*aggref).aggstar {
            return Some((
                AggregateType::CountAny {
                    filter: filter_expr,
                },
                filter_uses_search_operator,
            ));
        }

        // Parse field-based aggregates
        if args.is_empty() {
            return None;
        }

        let first_arg = args.get_ptr(0)?;
        let (field, missing) = Self::parse_aggregate_field(first_arg, heaprelid)?;
        let agg_type = Self::create_aggregate_from_oid(aggfnoid, field, missing, filter_expr)?;

        Some((agg_type, filter_uses_search_operator))
    }

    /// Extract filter clause from aggregate if present
    unsafe fn extract_filter_clause(
        aggref: *mut pg_sys::Aggref,
        bm25_index: &crate::postgres::PgSearchRelation,
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
    ) -> (Option<SearchQueryInput>, bool) {
        if (*aggref).aggfilter.is_null() {
            return (None, false);
        }

        let mut filter_qual_state =
            crate::postgres::customscan::qual_inspect::QualExtractState::default();
        let filter_result = crate::postgres::customscan::aggregatescan::extract_filter_clause(
            (*aggref).aggfilter,
            bm25_index,
            root,
            heap_rti,
            &mut filter_qual_state,
        );
        (filter_result, filter_qual_state.uses_our_operator)
    }

    /// Parse field name and missing value from aggregate argument
    unsafe fn parse_aggregate_field(
        first_arg: *mut pg_sys::TargetEntry,
        heaprelid: pg_sys::Oid,
    ) -> Option<(String, Option<f64>)> {
        let (var, missing) = if let Some(coalesce_node) =
            nodecast!(CoalesceExpr, T_CoalesceExpr, (*first_arg).expr)
        {
            Self::parse_coalesce_expression(coalesce_node)?
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
    ) -> Option<AggregateType> {
        match aggfnoid {
            F_COUNT_ANY => Some(AggregateType::Count {
                field,
                missing,
                filter,
            }),
            F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
                Some(AggregateType::Avg {
                    field,
                    missing,
                    filter,
                })
            }
            F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
                Some(AggregateType::Sum {
                    field,
                    missing,
                    filter,
                })
            }
            F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
            | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
                Some(AggregateType::Max {
                    field,
                    missing,
                    filter,
                })
            }
            F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
            | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
            | F_MIN_NUMERIC => Some(AggregateType::Min {
                field,
                missing,
                filter,
            }),
            _ => {
                pgrx::debug1!("Unknown aggregate function OID: {}", aggfnoid);
                None
            }
        }
    }
}
