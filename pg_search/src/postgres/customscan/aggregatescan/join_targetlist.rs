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
use crate::postgres::var::{fieldname_from_var, find_one_aggref};
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
    StddevSamp,
    StddevPop,
    VarSamp,
    VarPop,
    BoolAnd,
    BoolOr,
    ArrayAgg,
    /// STRING_AGG(col, separator) — stores the separator string.
    StringAgg(String),
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
            AggKind::BoolAnd => write!(f, "BOOL_AND"),
            AggKind::BoolOr => write!(f, "BOOL_OR"),
            AggKind::ArrayAgg => write!(f, "ARRAY_AGG"),
            AggKind::StringAgg(_) => write!(f, "STRING_AGG"),
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

/// A single ORDER BY entry within an aggregate (e.g., the `ORDER BY col2` in
/// `STRING_AGG(col, ',' ORDER BY col2)`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggOrderByEntry {
    /// Table RTI for the ORDER BY column.
    pub rti: pg_sys::Index,
    /// 1-based attribute number in the source relation's tuple descriptor.
    pub attno: pg_sys::AttrNumber,
    /// Resolved field name (from the BM25 index schema).
    pub field_name: String,
    /// Sort direction including NULLS FIRST/LAST.
    pub direction: crate::api::SortDirection,
}

/// An aggregate function in a join aggregate query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinAggregateEntry {
    /// Postgres aggregate function OID.
    pub func_oid: u32,
    /// Simplified classification.
    pub agg_kind: AggKind,
    /// Field references: (rti, attno, field_name). Empty for COUNT(*),
    /// single entry for most aggregates, multiple for COUNT(DISTINCT col1, col2).
    pub field_refs: Vec<(pg_sys::Index, pg_sys::AttrNumber, String)>,
    /// Position in the output tuple.
    pub output_index: usize,
    /// Postgres result type OID (INT8OID for COUNT, FLOAT8OID for others).
    pub result_type_oid: pg_sys::Oid,
    /// Whether this aggregate uses DISTINCT (e.g., SUM(DISTINCT col)).
    /// For CountDistinct this is implicitly true via AggKind; for other
    /// aggregates this flag drives the DataFusion `distinct` parameter.
    #[serde(default)]
    pub distinct: bool,
    /// ORDER BY within the aggregate (e.g., `STRING_AGG(col, ',' ORDER BY col2)`).
    /// Empty for aggregates without internal ordering.
    #[serde(default)]
    pub order_by: Vec<AggOrderByEntry>,
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
        "bool_and" | "every" => Some(AggKind::BoolAnd),
        "bool_or" => Some(AggKind::BoolOr),
        "array_agg" => Some(AggKind::ArrayAgg),
        // STRING_AGG separator is extracted later in extract_aggregate_targetlist
        "string_agg" => Some(AggKind::StringAgg(",".into())),
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
        } else if let Some(aggref) = find_one_aggref(expr as *mut pg_sys::Node) {
            // Aggregate function (possibly wrapped in COALESCE, etc.)
            let aggfnoid = (*aggref).aggfnoid.to_u32();
            let has_distinct = !(*aggref).aggdistinct.is_null();

            // Reject FILTER (WHERE ...) clauses on aggregates — DataFusion
            // doesn't propagate per-aggregate filter predicates and would
            // silently produce wrong results if we didn't fall back.
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

            let mut agg_kind = classify_aggregate_oid(aggfnoid, (*aggref).aggstar, has_distinct)
                .ok_or_else(|| format!("unsupported aggregate function OID: {}", aggfnoid))?;

            // For STRING_AGG, extract the separator from the second argument
            let is_string_agg = matches!(agg_kind, AggKind::StringAgg(_));
            if is_string_agg {
                let separator = extract_string_agg_separator(aggref).unwrap_or_else(|| ",".into());
                agg_kind = AggKind::StringAgg(separator);
            }

            let field_refs = extract_aggref_field_refs(aggref, sources, is_string_agg)?;
            let order_by = extract_aggref_order_by(aggref, sources)?;
            // Use the actual Postgres result type from the Aggref node,
            // not a guessed type — this avoids segfaults from type mismatches
            let result_type_oid = (*aggref).aggtype;

            aggregates.push(JoinAggregateEntry {
                func_oid: aggfnoid,
                agg_kind,
                field_refs,
                output_index: idx,
                result_type_oid,
                distinct: has_distinct,
                order_by,
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

/// Extract the separator string from a STRING_AGG's second argument.
///
/// STRING_AGG(col, separator) stores the separator as the second TargetEntry.
/// Returns `None` if the separator cannot be extracted (non-const, missing).
unsafe fn extract_string_agg_separator(aggref: *mut pg_sys::Aggref) -> Option<String> {
    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    if args.len() < 2 {
        return None;
    }
    let second_arg = args.get_ptr(1)?;
    let expr = (*second_arg).expr as *mut pg_sys::Node;
    if expr.is_null() || (*expr).type_ != pg_sys::NodeTag::T_Const {
        return None;
    }
    let konst = expr as *mut pg_sys::Const;
    if (*konst).constisnull {
        return None;
    }
    let datum = (*konst).constvalue;
    let text_ptr = datum.cast_mut_ptr::<pg_sys::varlena>();
    let cstr = pg_sys::text_to_cstring(text_ptr);
    if cstr.is_null() {
        return None;
    }
    let s = std::ffi::CStr::from_ptr(cstr).to_str().ok()?.to_owned();
    Some(s)
}

/// Extract the field reference from an `Aggref`'s arguments.
///
/// For `COUNT(*)`: returns empty (no field).
/// For `COUNT(col)`, `SUM(col)`, etc.: returns the column reference.
/// For `STRING_AGG(col, sep)`: only processes the first arg (column),
/// skipping the separator which is handled by `extract_string_agg_separator`.
unsafe fn extract_aggref_field_refs(
    aggref: *mut pg_sys::Aggref,
    sources: &[JoinAggSource],
    is_string_agg: bool,
) -> Result<Vec<(pg_sys::Index, pg_sys::AttrNumber, String)>, String> {
    // COUNT(*) has no arguments
    if (*aggref).aggstar {
        return Ok(Vec::new());
    }

    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    if args.is_empty() {
        return Err("aggregate function has no arguments".into());
    }

    // For STRING_AGG, only the first arg is the column reference;
    // the second arg is the separator constant.
    let num_field_args = if is_string_agg { 1 } else { args.len() };

    let mut refs = Vec::with_capacity(num_field_args);
    for (arg_idx, arg_ptr) in args.iter_ptr().enumerate() {
        if arg_idx >= num_field_args {
            break;
        }
        let expr = (*arg_ptr).expr;

        // The argument must be a bare Var (possibly wrapped in RelabelType).
        // Reject complex expressions like COALESCE(score, 0) — find_one_var
        // would strip the wrapper, causing DataFusion to compute e.g. SUM(score)
        // instead of the intended SUM(COALESCE(score, 0)).
        let var = unwrap_to_var(expr as *mut pg_sys::Node).ok_or(
            "aggregate argument must be a direct column reference; \
                     wrapped expressions (COALESCE, casts) are not supported for aggregate-on-join",
        )?;

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

        refs.push((rti, attno, field_name));
    }

    Ok(refs)
}

/// Extract ORDER BY entries from an aggregate's `aggorder` clause.
///
/// `aggorder` is a `List` of `SortGroupClause`. Each clause's `tleSortGroupRef`
/// matches a `TargetEntry.ressortgroupref` in the aggref's `args` list, identifying
/// which column to sort by.
///
/// Returns an empty Vec for aggregates without ORDER BY (the common case).
unsafe fn extract_aggref_order_by(
    aggref: *mut pg_sys::Aggref,
    sources: &[JoinAggSource],
) -> Result<Vec<AggOrderByEntry>, String> {
    if (*aggref).aggorder.is_null() {
        return Ok(Vec::new());
    }

    let order_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*aggref).aggorder);
    if order_clauses.is_empty() {
        return Ok(Vec::new());
    }

    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    let mut entries = Vec::with_capacity(order_clauses.len());

    for clause_ptr in order_clauses.iter_ptr() {
        let sort_ref = (*clause_ptr).tleSortGroupRef;

        // Find the TargetEntry in aggref.args whose ressortgroupref matches
        let te = args
            .iter_ptr()
            .find(|te| (*(*te)).ressortgroupref == sort_ref)
            .ok_or_else(|| {
                format!(
                    "aggorder references ressortgroupref {} but no matching arg found",
                    sort_ref
                )
            })?;

        let var = unwrap_to_var((*te).expr as *mut pg_sys::Node)
            .ok_or("ORDER BY within aggregate must reference a direct column")?;

        let rti = (*var).varno as pg_sys::Index;
        let attno = (*var).varattno;

        let source = sources.iter().find(|s| s.rti == rti).ok_or_else(|| {
            format!(
                "aggregate ORDER BY references table at RTI {} which is not in the join",
                rti
            )
        })?;

        let field_name = fieldname_from_var(source.relid, var, attno)
            .ok_or_else(|| {
                format!(
                    "could not resolve field name for aggregate ORDER BY (RTI={}, attno={})",
                    rti, attno
                )
            })?
            .into_inner();

        let direction = crate::api::SortDirection::from_sort_op(
            (*clause_ptr).sortop,
            (*clause_ptr).nulls_first,
        )
        .ok_or_else(|| {
            format!(
                "could not determine sort direction for aggregate ORDER BY (sortop={})",
                (*clause_ptr).sortop.to_u32()
            )
        })?;

        entries.push(AggOrderByEntry {
            rti,
            attno,
            field_name,
            direction,
        });
    }

    Ok(entries)
}

/// Unwrap an expression to a bare `Var`, allowing only `RelabelType` wrappers.
/// Returns `None` for anything more complex (COALESCE, FuncExpr, etc.)
/// so the caller can reject and fall back to native Postgres.
unsafe fn unwrap_to_var(mut node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    while !node.is_null() {
        match (*node).type_ {
            pg_sys::NodeTag::T_Var => return Some(node as *mut pg_sys::Var),
            pg_sys::NodeTag::T_RelabelType => {
                node = (*(node as *mut pg_sys::RelabelType)).arg as *mut pg_sys::Node;
            }
            _ => return None,
        }
    }
    None
}
