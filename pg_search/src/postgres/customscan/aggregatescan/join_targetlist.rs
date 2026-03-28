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

// These types are not yet called — they will be used by the planner integration in #4485.
#![allow(dead_code)]

//! Aggregate target list extraction for join aggregates.
//!
//! Parses `output_rel.reltarget.exprs` at the `UPPERREL_GROUP_AGG` stage to
//! produce a [`JoinAggregateTargetList`] that tracks which table each GROUP BY
//! column and aggregate argument belongs to. This is the join-aware counterpart
//! of [`super::targetlist::TargetList`] (which assumes a single base relation).

use super::datafusion_build::JoinAggSource;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::var::fieldname_from_var;
use pgrx::pg_sys;
use pgrx::pg_sys::{
    F_AVG_FLOAT4, F_AVG_FLOAT8, F_AVG_INT2, F_AVG_INT4, F_AVG_INT8, F_AVG_NUMERIC, F_COUNT_,
    F_COUNT_ANY, F_MAX_DATE, F_MAX_FLOAT4, F_MAX_FLOAT8, F_MAX_INT2, F_MAX_INT4, F_MAX_INT8,
    F_MAX_NUMERIC, F_MAX_TIME, F_MAX_TIMESTAMP, F_MAX_TIMESTAMPTZ, F_MAX_TIMETZ, F_MIN_DATE,
    F_MIN_FLOAT4, F_MIN_FLOAT8, F_MIN_INT2, F_MIN_INT4, F_MIN_INT8, F_MIN_MONEY, F_MIN_NUMERIC,
    F_MIN_TIME, F_MIN_TIMESTAMP, F_MIN_TIMESTAMPTZ, F_MIN_TIMETZ, F_SUM_FLOAT4, F_SUM_FLOAT8,
    F_SUM_INT2, F_SUM_INT4, F_SUM_INT8, F_SUM_NUMERIC,
};
use pgrx::PgList;

/// Simplified aggregate classification for the DataFusion backend.
/// Unlike [`AggregateType`] (Tantivy-oriented), this enum is lightweight and maps
/// directly to DataFusion aggregate expressions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AggKind {
    CountStar,
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
    StddevSamp,
    StddevPop,
    VarSamp,
    VarPop,
}

impl std::fmt::Display for AggKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggKind::CountStar => write!(f, "COUNT(*)"),
            AggKind::Count => write!(f, "COUNT"),
            AggKind::CountDistinct => write!(f, "COUNT(DISTINCT)"),
            AggKind::Sum => write!(f, "SUM"),
            AggKind::Avg => write!(f, "AVG"),
            AggKind::Min => write!(f, "MIN"),
            AggKind::Max => write!(f, "MAX"),
            AggKind::StddevSamp => write!(f, "STDDEV_SAMP"),
            AggKind::StddevPop => write!(f, "STDDEV_POP"),
            AggKind::VarSamp => write!(f, "VAR_SAMP"),
            AggKind::VarPop => write!(f, "VAR_POP"),
        }
    }
}

/// A GROUP BY column reference in a join aggregate query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinGroupColumn {
    /// Range table index of the table this column belongs to.
    pub rti: pg_sys::Index,
    /// Attribute number within that table.
    pub attno: pg_sys::AttrNumber,
    /// Resolved field name (from the BM25 index schema).
    pub field_name: String,
    /// Position in the output tuple (index into `output_rel.reltarget.exprs`).
    pub output_index: usize,
}

/// An aggregate function in a join aggregate query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinAggregateEntry {
    /// Postgres aggregate function OID.
    pub func_oid: u32,
    /// Simplified classification.
    pub agg_kind: AggKind,
    /// Field reference: (rti, attno, field_name). None for COUNT(*).
    pub field_ref: Option<(pg_sys::Index, pg_sys::AttrNumber, String)>,
    /// Position in the output tuple.
    pub output_index: usize,
    /// Postgres result type OID (INT8OID for COUNT, FLOAT8OID for others).
    pub result_type_oid: pg_sys::Oid,
}

/// The complete aggregate target list for a join aggregate query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinAggregateTargetList {
    pub group_columns: Vec<JoinGroupColumn>,
    pub aggregates: Vec<JoinAggregateEntry>,
}

/// Classify an aggregate function OID into an [`AggKind`].
///
/// Returns `None` for unsupported or unknown OIDs (including `pdb.agg()`).
fn classify_aggregate_oid(aggfnoid: u32, aggstar: bool, has_distinct: bool) -> Option<AggKind> {
    if aggfnoid == F_COUNT_ && aggstar {
        return Some(AggKind::CountStar);
    }

    match aggfnoid {
        F_COUNT_ANY if has_distinct => Some(AggKind::CountDistinct),
        F_COUNT_ANY => Some(AggKind::Count),
        F_AVG_INT8 | F_AVG_INT4 | F_AVG_INT2 | F_AVG_NUMERIC | F_AVG_FLOAT4 | F_AVG_FLOAT8 => {
            Some(AggKind::Avg)
        }
        F_SUM_INT8 | F_SUM_INT4 | F_SUM_INT2 | F_SUM_FLOAT4 | F_SUM_FLOAT8 | F_SUM_NUMERIC => {
            Some(AggKind::Sum)
        }
        F_MAX_INT8 | F_MAX_INT4 | F_MAX_INT2 | F_MAX_FLOAT4 | F_MAX_FLOAT8 | F_MAX_DATE
        | F_MAX_TIME | F_MAX_TIMETZ | F_MAX_TIMESTAMP | F_MAX_TIMESTAMPTZ | F_MAX_NUMERIC => {
            Some(AggKind::Max)
        }
        F_MIN_INT8 | F_MIN_INT4 | F_MIN_INT2 | F_MIN_FLOAT4 | F_MIN_FLOAT8 | F_MIN_DATE
        | F_MIN_TIME | F_MIN_TIMETZ | F_MIN_MONEY | F_MIN_TIMESTAMP | F_MIN_TIMESTAMPTZ
        | F_MIN_NUMERIC => Some(AggKind::Min),
        _ => classify_aggregate_by_name(aggfnoid),
    }
}

/// Fallback classification by looking up the function name from the catalog.
/// Handles aggregate functions whose OIDs aren't exposed as constants in pg_sys
/// (e.g., STDDEV, VARIANCE and their variants).
fn classify_aggregate_by_name(aggfnoid: u32) -> Option<AggKind> {
    let name = unsafe {
        let name_ptr = pg_sys::get_func_name(pg_sys::Oid::from(aggfnoid));
        if name_ptr.is_null() {
            return None;
        }
        std::ffi::CStr::from_ptr(name_ptr).to_str().ok()?.to_owned()
    };
    match name.as_str() {
        "stddev" | "stddev_samp" => Some(AggKind::StddevSamp),
        "stddev_pop" => Some(AggKind::StddevPop),
        "variance" | "var_samp" => Some(AggKind::VarSamp),
        "var_pop" => Some(AggKind::VarPop),
        _ => None,
    }
}

/// Extract aggregate target list from `output_rel.reltarget.exprs` for a join
/// aggregate query.
///
/// Iterates the target list and classifies each expression as either a GROUP BY
/// column (`T_Var`) or an aggregate function (`T_Aggref`). For joins, `Var.varno`
/// tells us which table the column belongs to.
///
/// # Errors
///
/// Returns an error if:
/// - An expression is neither a `Var` nor an `Aggref`
/// - An aggregate uses DISTINCT (`aggdistinct` is set)
/// - An aggregate is `pdb.agg()` (not supported on joins in M1)
/// - An aggregate OID is unknown/unsupported
/// - A `Var` references a table not in `sources`
/// - A field name cannot be resolved
pub unsafe fn extract_aggregate_targetlist(
    args: &CreateUpperPathsHookArgs,
    sources: &[JoinAggSource],
) -> Result<JoinAggregateTargetList, String> {
    let output_rel = args.output_rel();
    let target_exprs = PgList::<pg_sys::Expr>::from_pg((*output_rel.reltarget).exprs);
    if target_exprs.is_empty() {
        return Err("target list is empty".into());
    }

    let mut group_columns = Vec::new();
    let mut aggregates = Vec::new();

    for (idx, expr) in target_exprs.iter_ptr().enumerate() {
        let tag = (*(expr as *mut pg_sys::Node)).type_;

        if tag == pg_sys::NodeTag::T_Var {
            // GROUP BY column
            let var = expr as *mut pg_sys::Var;
            let rti = (*var).varno as pg_sys::Index;
            let attno = (*var).varattno;

            let source = sources.iter().find(|s| s.rti == rti).ok_or_else(|| {
                format!(
                    "GROUP BY column references table at RTI {} which is not in the join",
                    rti
                )
            })?;

            let field_name = fieldname_from_var(source.relid, var, attno)
                .ok_or_else(|| {
                    format!(
                        "could not resolve field name for column (RTI={}, attno={})",
                        rti, attno
                    )
                })?
                .into_inner();

            group_columns.push(JoinGroupColumn {
                rti,
                attno,
                field_name,
                output_index: idx,
            });
        } else if let Some(aggref) = find_aggref_in_expr(expr as *mut pg_sys::Node) {
            // Aggregate function (possibly wrapped in COALESCE, etc.)
            let aggfnoid = (*aggref).aggfnoid.to_u32();
            let has_distinct = !(*aggref).aggdistinct.is_null();

            // Reject pdb.agg()
            let pdb_agg_oid = crate::api::agg_funcoid().to_u32();
            let pdb_agg_mvcc_oid = crate::api::agg_with_solve_mvcc_funcoid().to_u32();
            if aggfnoid == pdb_agg_oid || aggfnoid == pdb_agg_mvcc_oid {
                return Err(
                    "pdb.agg() is not supported on joins — use standard SQL aggregates (COUNT, SUM, AVG, MIN, MAX)".into()
                );
            }

            let agg_kind = classify_aggregate_oid(aggfnoid, (*aggref).aggstar, has_distinct)
                .ok_or_else(|| format!("unsupported aggregate function OID: {}", aggfnoid))?;

            let field_ref = extract_aggref_field_ref(aggref, sources)?;
            // Use the actual Postgres result type from the Aggref node,
            // not a guessed type — this avoids segfaults from type mismatches
            let result_type_oid = (*aggref).aggtype;

            aggregates.push(JoinAggregateEntry {
                func_oid: aggfnoid,
                agg_kind,
                field_ref,
                output_index: idx,
                result_type_oid,
            });
        } else {
            return Err(format!(
                "expression at index {} is neither a GROUP BY column (Var) nor an aggregate (Aggref)",
                idx
            ));
        }
    }

    Ok(JoinAggregateTargetList {
        group_columns,
        aggregates,
    })
}

/// Extract the field reference from an `Aggref`'s arguments.
///
/// For `COUNT(*)`: returns `None` (no field).
/// For `COUNT(col)`, `SUM(col)`, etc.: returns `Some((rti, attno, field_name))`.
unsafe fn extract_aggref_field_ref(
    aggref: *mut pg_sys::Aggref,
    sources: &[JoinAggSource],
) -> Result<Option<(pg_sys::Index, pg_sys::AttrNumber, String)>, String> {
    // COUNT(*) has no arguments
    if (*aggref).aggstar {
        return Ok(None);
    }

    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    if args.is_empty() {
        return Err("aggregate function has no arguments".into());
    }

    let first_arg = args.get_ptr(0).ok_or("aggregate missing first argument")?;
    let expr = (*first_arg).expr;

    // The argument may be a direct Var or wrapped in COALESCE / RelabelType
    let var = find_var_in_expr(expr as *mut pg_sys::Node)
        .ok_or("aggregate argument must reference a column (Var)")?;

    let rti = (*var).varno as pg_sys::Index;
    let attno = (*var).varattno;

    let source = sources.iter().find(|s| s.rti == rti).ok_or_else(|| {
        format!(
            "aggregate argument references table at RTI {} which is not in the join",
            rti
        )
    })?;

    let field_name = fieldname_from_var(source.relid, var, attno)
        .ok_or_else(|| {
            format!(
                "could not resolve field name for aggregate argument (RTI={}, attno={})",
                rti, attno
            )
        })?
        .into_inner();

    Ok(Some((rti, attno, field_name)))
}

/// Find an `Aggref` node in an expression tree. Handles cases where the
/// aggregate is wrapped in `COALESCE`, `NULLIF`, type coercion, etc.
/// Returns the `Aggref` pointer if exactly one is found.
unsafe fn find_aggref_in_expr(expr: *mut pg_sys::Node) -> Option<*mut pg_sys::Aggref> {
    if expr.is_null() {
        return None;
    }

    let tag = (*expr).type_;

    // Direct Aggref
    if tag == pg_sys::NodeTag::T_Aggref {
        return Some(expr as *mut pg_sys::Aggref);
    }

    // Unwrap common wrappers: RelabelType, CoerceViaIO, FuncExpr (COALESCE, NULLIF)
    if tag == pg_sys::NodeTag::T_RelabelType {
        let relabel = expr as *mut pg_sys::RelabelType;
        return find_aggref_in_expr((*relabel).arg as *mut pg_sys::Node);
    }

    if tag == pg_sys::NodeTag::T_CoerceViaIO {
        let coerce = expr as *mut pg_sys::CoerceViaIO;
        return find_aggref_in_expr((*coerce).arg as *mut pg_sys::Node);
    }

    if tag == pg_sys::NodeTag::T_CoalesceExpr {
        let coalesce = expr as *mut pg_sys::CoalesceExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*coalesce).args);
        // The aggregate is the first argument of COALESCE
        if let Some(first) = args.get_ptr(0) {
            return find_aggref_in_expr(first);
        }
    }

    if tag == pg_sys::NodeTag::T_FuncExpr {
        let func = expr as *mut pg_sys::FuncExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*func).args);
        // Check each argument for an Aggref
        for arg in args.iter_ptr() {
            if let Some(aggref) = find_aggref_in_expr(arg) {
                return Some(aggref);
            }
        }
    }

    None
}

/// Find a `Var` node in an expression tree, unwrapping common wrappers.
unsafe fn find_var_in_expr(expr: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    if expr.is_null() {
        return None;
    }

    let tag = (*expr).type_;

    if tag == pg_sys::NodeTag::T_Var {
        return Some(expr as *mut pg_sys::Var);
    }

    if tag == pg_sys::NodeTag::T_RelabelType {
        let relabel = expr as *mut pg_sys::RelabelType;
        return find_var_in_expr((*relabel).arg as *mut pg_sys::Node);
    }

    if tag == pg_sys::NodeTag::T_CoerceViaIO {
        let coerce = expr as *mut pg_sys::CoerceViaIO;
        return find_var_in_expr((*coerce).arg as *mut pg_sys::Node);
    }

    if tag == pg_sys::NodeTag::T_CoalesceExpr {
        let coalesce = expr as *mut pg_sys::CoalesceExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*coalesce).args);
        if let Some(first) = args.get_ptr(0) {
            return find_var_in_expr(first);
        }
    }

    None
}
