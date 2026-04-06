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

//! Aggregate target list extraction for join aggregates.
//!
//! Parses `output_rel.reltarget.exprs` at the `UPPERREL_GROUP_AGG` stage to
//! produce a [`JoinAggregateTargetList`] that tracks which table each GROUP BY
//! column and aggregate argument belongs to. This is the join-aware counterpart
//! of [`super::targetlist::TargetList`] (which assumes a single base relation).

use super::datafusion_build::JoinAggSource;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::var::{fieldname_from_var, find_one_aggref, find_one_var};
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AggKind {
    CountStar,
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
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
        _ if has_distinct => None,
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

    // Check for HAVING — not supported
    let parse = args.root().parse;
    if !parse.is_null() && !(*parse).havingQual.is_null() {
        return Err("HAVING clause is not supported for aggregate-on-join".into());
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
        } else if let Some(aggref) = find_one_aggref(expr as *mut pg_sys::Node) {
            // Aggregate function (possibly wrapped in COALESCE, etc.)
            let aggfnoid = (*aggref).aggfnoid.to_u32();
            let has_distinct = !(*aggref).aggdistinct.is_null();

            // Reject FILTER (WHERE ...) clauses on aggregates — DataFusion's
            // aggregate plan doesn't propagate per-aggregate filter predicates.
            if !(*aggref).aggfilter.is_null() {
                return Err(
                    "FILTER clauses on aggregates are not supported for aggregate-on-join".into(),
                );
            }

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
    let var = find_one_var(expr as *mut pg_sys::Node)
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
