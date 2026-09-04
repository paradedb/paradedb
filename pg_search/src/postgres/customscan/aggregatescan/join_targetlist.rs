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

use super::GroupingShape;
use super::datafusion_build::{
    FilterExprBuildContext, JoinAggSource, collect_join_agg_sources, resolve_source_field,
};
use super::pdb_agg::{PdbAggFieldRef, PdbAggRequest};
use super::privdat::FilterExpr;
use crate::api::{SortDirection, pdb_agg_spec};
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::customscan::datafusion::explain::get_attname_safe;
use crate::postgres::customscan::joinscan::build::RelationAlias;
use crate::postgres::var::{VarContext, find_one_var_and_fieldname};
use crate::schema::SearchFieldType;
use pgrx::PgList;
use pgrx::pg_guard;
use pgrx::pg_sys;
use pgrx::pg_sys::{
    F_AVG_FLOAT4, F_AVG_FLOAT8, F_AVG_INT2, F_AVG_INT4, F_AVG_INT8, F_AVG_NUMERIC, F_COUNT_,
    F_COUNT_ANY, F_MAX_DATE, F_MAX_FLOAT4, F_MAX_FLOAT8, F_MAX_INT2, F_MAX_INT4, F_MAX_INT8,
    F_MAX_NUMERIC, F_MAX_TIME, F_MAX_TIMESTAMP, F_MAX_TIMESTAMPTZ, F_MAX_TIMETZ, F_MIN_DATE,
    F_MIN_FLOAT4, F_MIN_FLOAT8, F_MIN_INT2, F_MIN_INT4, F_MIN_INT8, F_MIN_MONEY, F_MIN_NUMERIC,
    F_MIN_TIME, F_MIN_TIMESTAMP, F_MIN_TIMESTAMPTZ, F_MIN_TIMETZ, F_SUM_FLOAT4, F_SUM_FLOAT8,
    F_SUM_INT2, F_SUM_INT4, F_SUM_INT8, F_SUM_NUMERIC,
};

/// Look up a join source by RTI, returning a uniform error message that
/// names the calling context (e.g. "GROUP BY column", "aggregate argument").
///
/// The three sites that called `sources.iter().find(|s| s.rti == rti)` with
/// nearly-identical `ok_or_else(...)` formatters now share this helper so the
/// error wording stays consistent across the file.
fn find_source_by_rti<'a>(
    sources: &'a [JoinAggSource],
    rti: pg_sys::Index,
    context_label: &str,
) -> Result<&'a JoinAggSource, String> {
    sources.iter().find(|s| s.rti == rti).ok_or_else(|| {
        format!("{context_label} references table at RTI {rti} which is not in the join")
    })
}

/// Simplified aggregate classification for the DataFusion backend.
/// Unlike `AggregateType` (Tantivy-oriented), this enum is lightweight and maps
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
    /// STRING_AGG(col, separator) - stores the separator string.
    StringAgg(String),
    /// `pdb.agg(jsonb)`, lowered to grouping sets and metric expressions.
    PdbAgg(Box<PdbAggRequest>),
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
            AggKind::PdbAgg(request) => write!(f, "pdb.agg({})", request.agg_json),
        }
    }
}

/// A transformation applied to a fast-field value before DataFusion groups it.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, Default, Hash, PartialEq, Eq,
)]
pub enum GroupingTransform {
    /// Group by the fast field's stored value without changing it.
    #[default]
    Identity,

    /// Apply PostgreSQL's `date(timestamp)` semantics before grouping.
    TimestampToDate,
}

/// A GROUP BY column reference in a join aggregate query.
///
/// `plan_position` is the unique DataFusion-facing source identity (used
/// for execution-time column binding). `attno` is load-bearing for
/// fast-field projection in `populate_required_fields`. `rti` is not
/// stored - recoverable via `source_at_plan_position(plan_position).rti`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinGroupColumn {
    pub plan_position: usize,
    pub attno: pg_sys::AttrNumber,
    pub field_name: String,
    /// Position in the output tuple (index into `output_rel.reltarget.exprs`).
    pub output_index: usize,
    /// Declared scale when this is a NUMERIC field. Grouping itself works on
    /// the stored representation; the scale is needed to render group keys
    /// with the column's display scale.
    #[serde(default)]
    pub numeric_scale: Option<i16>,

    /// Transformation applied to the fast-field value before grouping.
    #[serde(default)]
    pub transform: GroupingTransform,
}

/// The NUMERIC field type an aggregate has to handle, or `None` when the
/// aggregate needs no numeric-specific treatment.
///
/// COUNT variants work on the stored representation directly, since both
/// encodings are canonical and byte-distinct means value-distinct.
/// SUM/AVG/MIN/MAX need the declared scale to render their result, so an
/// unbounded NUMERIC declines. Everything else declines as well, which lands
/// the query on Postgres instead of failing when DataFusion rejects the
/// storage type.
fn numeric_agg_field_type(
    agg_kind: &AggKind,
    field_refs: &[JoinAggColRef],
    has_distinct: bool,
) -> Result<Option<SearchFieldType>, String> {
    let Some(field_type) = field_refs.iter().find_map(|r| r.numeric) else {
        return Ok(None);
    };

    match agg_kind {
        AggKind::CountStar | AggKind::Count | AggKind::CountDistinct => Ok(None),
        AggKind::Sum | AggKind::Avg | AggKind::Min | AggKind::Max => {
            if has_distinct {
                return Err(format!(
                    "{agg_kind} with DISTINCT is not supported on NUMERIC columns"
                ));
            }
            if field_type.numeric_scale().is_none() {
                return Err(format!(
                    "{agg_kind} on an unbounded NUMERIC column is not supported; declare a \
                     precision and scale to enable aggregate pushdown"
                ));
            }
            Ok(Some(field_type))
        }
        _ => Err(format!("{agg_kind} is not supported on NUMERIC columns")),
    }
}

/// Aggregate-argument column reference. Same identity model as
/// [`JoinGroupColumn`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinAggColRef {
    pub plan_position: usize,
    pub attno: pg_sys::AttrNumber,
    pub field_name: String,
    /// Set when the referenced field is NUMERIC.
    #[serde(default)]
    pub numeric: Option<SearchFieldType>,
}

/// Aggregate ORDER BY entry (e.g. `STRING_AGG(col, ',' ORDER BY col2)`).
/// Same identity model as [`JoinGroupColumn`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggOrderByEntry {
    pub plan_position: usize,
    /// 1-based attribute number in the source relation's tuple descriptor.
    /// Load-bearing for fast-field projection.
    pub attno: pg_sys::AttrNumber,
    /// Resolved field name (from the ParadeDB index schema).
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
    /// Field references. Empty for COUNT(*), single entry for most
    /// aggregates, multiple for `COUNT(DISTINCT col1, col2)`.
    pub field_refs: Vec<JoinAggColRef>,
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
    /// Per-aggregate FILTER clause (e.g., `COUNT(*) FILTER (WHERE price > 100)`).
    /// `None` when the aggregate has no FILTER.
    #[serde(default)]
    pub filter: Option<FilterExpr>,
    /// Set for SUM/AVG/MIN/MAX over a NUMERIC field. COUNT works on the
    /// stored representation and leaves this `None`.
    #[serde(default)]
    pub numeric: Option<SearchFieldType>,
}

/// The complete aggregate target list for a join aggregate query. Each
/// targetlist ref carries a resolved `plan_position` (an opaque, unique
/// per-`RelNode::Scan` identity), so execution-time column binding is a
/// straight lookup with no ambiguity from rti aliasing across sub-
/// PlannerInfos.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinAggregateTargetList {
    pub group_columns: Vec<JoinGroupColumn>,
    pub aggregates: Vec<JoinAggregateEntry>,
}

/// Planner-only aggregate extraction result.
///
/// `runtime` is serialized in `PrivateData` for execution. `scan_tlist` and
/// the expression pointers live only during planning; they define the raw
/// tuple emitted by DataFusion and consumed by PostgreSQL's setrefs pass.
pub struct ExtractedDataFusionTarget {
    pub runtime: JoinAggregateTargetList,
    pub scan_tlist: Vec<*mut pg_sys::TargetEntry>,
    group_exprs: Vec<*mut pg_sys::Node>,
    aggrefs: Vec<*mut pg_sys::Aggref>,
}

impl ExtractedDataFusionTarget {
    pub unsafe fn aggregate_index(&self, expr: *mut pg_sys::Node) -> Option<usize> {
        self.aggrefs.iter().position(|aggref| {
            pg_sys::equal(
                (*aggref).cast::<core::ffi::c_void>(),
                expr.cast::<core::ffi::c_void>(),
            )
        })
    }

    pub unsafe fn group_index(&self, expr: *mut pg_sys::Node) -> Option<usize> {
        self.group_exprs.iter().position(|group_expr| {
            pg_sys::equal(
                (*group_expr).cast::<core::ffi::c_void>(),
                expr.cast::<core::ffi::c_void>(),
            )
        })
    }
}

impl JoinAggregateTargetList {
    /// The lowered `pdb.agg()` calls, in target-list order.
    pub fn pdb_agg_requests(&self) -> impl Iterator<Item = &PdbAggRequest> {
        self.aggregates
            .iter()
            .filter_map(|agg| match &agg.agg_kind {
                AggKind::PdbAgg(request) => Some(request.as_ref()),
                _ => None,
            })
    }

    /// Every fast field a `pdb.agg()` spec reads.
    pub fn pdb_agg_field_refs(&self) -> impl Iterator<Item = &PdbAggFieldRef> {
        self.pdb_agg_requests().flat_map(PdbAggRequest::fields)
    }
}

/// Classify an aggregate function OID into an [`AggKind`].
///
/// Returns `None` for unsupported or unknown OIDs. `pdb.agg()` is handled before
/// this is reached.
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
    let name = crate::postgres::catalog::lookup_func_name(pg_sys::Oid::from(aggfnoid))?;
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

unsafe fn extract_timestamp_to_date_var(
    expr: *mut pg_sys::Node,
) -> Result<Option<*mut pg_sys::Var>, String> {
    if expr.is_null() || (*expr).type_ != pg_sys::NodeTag::T_FuncExpr {
        return Ok(None);
    }

    let fun_expr = expr.cast::<pg_sys::FuncExpr>();

    match (*fun_expr).funcid.to_u32() {
        pg_sys::F_DATE_TIMESTAMP => {}
        pg_sys::F_DATE_TIMESTAMPTZ => {
            return Err(
                "DATE(timestamptz) grouping is not pushed down because it depends on the session TimeZone"
                    .into(),
            );
        }
        _ => return Ok(None),
    }

    let args = PgList::<pg_sys::Node>::from_pg((*fun_expr).args);

    if args.len() != 1 {
        return Err("DATE(timestamp) grouping has an unexpected argument count".into());
    }

    let inner = args
        .get_ptr(0)
        .ok_or_else(|| "DATE(timestamp) grouping has a missing argument".to_string())?;

    if inner.is_null() || (*inner).type_ != pg_sys::NodeTag::T_Var {
        return Err("DATE(timestamp) grouping requires a bare timestamp column, casts and other expressions are not supported".into());
    }

    let var = inner.cast::<pg_sys::Var>();

    if (*var).vartype != pg_sys::TIMESTAMPOID {
        return Err(
            "DATE(timestamp) grouping requires a timestamp column, found a non-timestamp column"
                .into(),
        );
    }

    Ok(Some(var))
}

/// Find output Vars which PostgreSQL may permit through functional dependency.
///
/// GROUP BY expressions and aggregate arguments are boundaries: setrefs first
/// matches the former as whole expressions, while the latter are evaluated by
/// DataFusion. Every other Var must be carried in the raw tuple.
unsafe fn find_nonaggregate_output_vars(
    expr: *mut pg_sys::Node,
    group_exprs: &[(*mut pg_sys::Node, pg_sys::Index)],
) -> Vec<*mut pg_sys::Var> {
    struct WalkerContext {
        group_exprs: *const (*mut pg_sys::Node, pg_sys::Index),
        group_exprs_len: usize,
        vars: Vec<*mut pg_sys::Var>,
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        let ctx = &mut *(context as *mut WalkerContext);
        let group_exprs = std::slice::from_raw_parts(ctx.group_exprs, ctx.group_exprs_len);
        if group_exprs
            .iter()
            .any(|(group_expr, _)| pg_sys::equal((*group_expr).cast(), node.cast()))
        {
            return false;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_Aggref => false,
            pg_sys::NodeTag::T_Var => {
                let var = node.cast::<pg_sys::Var>();
                if !ctx
                    .vars
                    .iter()
                    .any(|known| pg_sys::equal((*known).cast(), var.cast()))
                {
                    ctx.vars.push(var);
                }
                false
            }
            _ => pg_sys::expression_tree_walker(node, Some(walker), context),
        }
    }

    let mut context = WalkerContext {
        group_exprs: group_exprs.as_ptr(),
        group_exprs_len: group_exprs.len(),
        vars: Vec::new(),
    };
    walker(expr, (&mut context as *mut WalkerContext).cast());
    context.vars
}

/// Extract the DataFusion runtime metadata and raw tuple description for a
/// join aggregate query from its grouping/DISTINCT output.
///
/// Group expressions and aggregates are resolved to unique plan positions at
/// extraction time, so execution-time binding is immune to RTI aliasing across
/// sub-PlannerInfos.
///
/// # Errors
///
/// Returns an error when an expression cannot be resolved to a supported
/// grouping input or aggregate, including unsupported aggregate OIDs, missing
/// indexed fields, and ambiguous plan positions.
pub unsafe fn extract_aggregate_targetlist(
    args: &CreateUpperPathsHookArgs,
    sources: &[JoinAggSource],
    plan: &crate::postgres::customscan::joinscan::build::RelNode,
    shape: GroupingShape,
    mut pdb_route: Option<PdbAggRoute>,
) -> Result<ExtractedDataFusionTarget, String> {
    let target_exprs = shape.target_exprs();
    if target_exprs.is_empty() {
        return Err("target list is empty".into());
    }
    let parse = (*args.root).parse;
    if parse.is_null() {
        return Err("query parse tree is missing".into());
    }
    let plain_columns_only = shape.is_distinct();
    let clause = if plain_columns_only {
        "DISTINCT"
    } else {
        "GROUP BY"
    };
    let outer_root_id =
        crate::postgres::customscan::joinscan::build::PlannerRootId::from(args.root);

    let mut group_exprs: Vec<(*mut pg_sys::Node, pg_sys::Index)> = Vec::new();
    if plain_columns_only {
        for (idx, expr) in target_exprs.iter_ptr().enumerate() {
            if super::targetlist::find_aggrefs_in_expr(expr.cast()).is_empty()
                && !group_exprs
                    .iter()
                    .any(|(known, _)| pg_sys::equal((*known).cast(), expr.cast()))
            {
                group_exprs.push((expr.cast(), (idx + 1) as pg_sys::Index));
            }
        }
    } else {
        for sgc in PgList::<pg_sys::SortGroupClause>::from_pg((*parse).groupClause).iter_ptr() {
            let expr = pg_sys::get_sortgroupclause_expr(sgc, (*parse).targetList);
            if expr.is_null() {
                return Err("GROUP BY expression is missing from the query target list".into());
            }
            if !group_exprs
                .iter()
                .any(|(known, _)| pg_sys::equal((*known).cast(), expr.cast()))
            {
                group_exprs.push((expr, (*sgc).tleSortGroupRef));
            }
        }
    }

    for expr in target_exprs.iter_ptr() {
        for var in find_nonaggregate_output_vars(expr.cast(), &group_exprs) {
            if !group_exprs
                .iter()
                .any(|(known, _)| pg_sys::equal((*known).cast(), var.cast()))
            {
                group_exprs.push((var.cast(), 0));
            }
        }
    }
    if !(*parse).havingQual.is_null() {
        for var in find_nonaggregate_output_vars((*parse).havingQual, &group_exprs) {
            if !group_exprs
                .iter()
                .any(|(known, _)| pg_sys::equal((*known).cast(), var.cast()))
            {
                group_exprs.push((var.cast(), 0));
            }
        }
    }

    let mut aggrefs: Vec<(*mut pg_sys::Aggref, Option<usize>)> = Vec::new();
    let mut collect = |expr: *mut pg_sys::Node, output_index: Option<usize>| {
        for aggref in super::targetlist::find_aggrefs_in_expr(expr) {
            if !aggrefs
                .iter()
                .any(|(known, _)| pg_sys::equal((*known).cast(), aggref.cast()))
            {
                aggrefs.push((aggref, output_index));
            }
        }
    };
    for (idx, expr) in target_exprs.iter_ptr().enumerate() {
        collect(expr.cast(), Some(idx));
    }
    if !(*parse).havingQual.is_null() {
        collect((*parse).havingQual, None);
    }

    let group_context = RawGroupColumnContext {
        args,
        sources,
        plan,
        outer_root_id,
        clause,
        plain_columns_only,
    };
    let mut group_columns = Vec::with_capacity(group_exprs.len());
    for (raw_index, (expr, _)) in group_exprs.iter().enumerate() {
        group_columns.push(extract_raw_group_column(&group_context, *expr, raw_index)?);
    }
    let mut aggregates = Vec::with_capacity(aggrefs.len());
    for (offset, (aggref, output_index)) in aggrefs.iter().enumerate() {
        aggregates.push(extract_raw_aggregate_entry(
            *aggref,
            sources,
            plan,
            outer_root_id,
            group_columns.len() + offset,
            output_index.and_then(|_| {
                pdb_route
                    .as_mut()
                    .and_then(|route| route.take_request(*aggref))
            }),
        )?);
    }

    if group_columns.is_empty() && aggregates.is_empty() {
        return Err(format!(
            "{clause} target contains no supported grouping columns or aggregates"
        ));
    }
    let mut scan_tlist = Vec::with_capacity(group_columns.len() + aggregates.len());
    for (expr, sort_group_ref) in &group_exprs {
        let resno = scan_tlist.len() as pg_sys::AttrNumber + 1;
        let te = pg_sys::makeTargetEntry(
            pg_sys::copyObjectImpl((*expr).cast()).cast::<pg_sys::Expr>(),
            resno,
            std::ptr::null_mut(),
            false,
        );
        (*te).ressortgroupref = *sort_group_ref;
        scan_tlist.push(te);
    }
    for (aggref, _) in &aggrefs {
        let resno = scan_tlist.len() as pg_sys::AttrNumber + 1;
        scan_tlist.push(pg_sys::makeTargetEntry(
            pg_sys::copyObjectImpl((*aggref).cast()).cast::<pg_sys::Expr>(),
            resno,
            std::ptr::null_mut(),
            false,
        ));
    }
    Ok(ExtractedDataFusionTarget {
        runtime: JoinAggregateTargetList {
            group_columns,
            aggregates,
        },
        scan_tlist,
        group_exprs: group_exprs.into_iter().map(|(expr, _)| expr).collect(),
        aggrefs: aggrefs.into_iter().map(|(aggref, _)| aggref).collect(),
    })
}

struct RawGroupColumnContext<'a> {
    args: &'a CreateUpperPathsHookArgs,
    sources: &'a [JoinAggSource],
    plan: &'a crate::postgres::customscan::joinscan::build::RelNode,
    outer_root_id: crate::postgres::customscan::joinscan::build::PlannerRootId,
    clause: &'a str,
    plain_columns_only: bool,
}

unsafe fn extract_raw_group_column(
    context: &RawGroupColumnContext<'_>,
    expr: *mut pg_sys::Node,
    output_index: usize,
) -> Result<JoinGroupColumn, String> {
    let mut node = expr;
    let mut transform = GroupingTransform::Identity;
    if !context.plain_columns_only
        && let Some(var) = extract_timestamp_to_date_var(node)?
    {
        node = var.cast();
        transform = GroupingTransform::TimestampToDate;
    }
    let (attno, field_name, plan_position, numeric_source) = if (*node).type_
        == pg_sys::NodeTag::T_Var
    {
        let var = node.cast::<pg_sys::Var>();
        let rti = (*var).varno as pg_sys::Index;
        let attno = (*var).varattno;
        if let Some(unnest) = context.plan.find_lateral_unnest(rti) {
            let source = find_source_by_rti(context.sources, unnest.source_rti.0, context.clause)?;
            let plan_position = context.plan.plan_position(context.outer_root_id, unnest.source_rti.0, unnest.source_attno)
                .ok_or_else(|| format!("GROUP BY unnest column (RTI={rti}) does not resolve to a unique output-visible source in the plan tree"))?;
            (
                unnest.source_attno,
                unnest.field_name.clone(),
                plan_position,
                Some(source),
            )
        } else {
            let source = find_source_by_rti(context.sources, rti, context.clause)?;
            let field_name = source.column_name(attno).ok_or_else(|| {
                let alias =
                    RelationAlias::new(source.alias.as_deref()).display(source.rti as usize);
                format!(
                    "{} column {} is not columnar indexed",
                    context.clause,
                    get_attname_safe(Some(source.relid), attno, &alias)
                )
            })?;
            let plan_position = context.plan.plan_position(context.outer_root_id, rti, attno)
                .ok_or_else(|| format!("GROUP BY column (RTI={rti}, attno={attno}) does not resolve to a unique output-visible source in the plan tree"))?;
            (attno, field_name, plan_position, Some(source))
        }
    } else if !context.plain_columns_only {
        let (var, field_name) = find_one_var_and_fieldname(
            VarContext::from_planner(context.args.root),
            expr,
        )
        .ok_or_else(|| {
            "GROUP BY on this expression is not pushed down, only supported indexed expressions are"
                .to_string()
        })?;
        let rti = (*var).varno as pg_sys::Index;
        let attno = (*var).varattno;
        let plan_position = context.plan.plan_position(context.outer_root_id, rti, attno)
            .ok_or_else(|| format!("GROUP BY expression at RTI {rti} (attno={attno}) does not resolve to a unique output-visible source in the plan tree"))?;
        (
            attno,
            field_name.into_inner(),
            plan_position,
            find_source_by_rti(context.sources, rti, "GROUP BY expression").ok(),
        )
    } else {
        return Err("DISTINCT on an expression is not pushed down, only plain columns are".into());
    };
    let numeric_scale = numeric_source
        .and_then(|source| source.bm25_index.as_ref())
        .and_then(|index| index.schema().ok())
        .and_then(|schema| schema.numeric_field_type(&field_name))
        .map(|(_, scale)| scale.ok_or_else(|| format!("{} column {field_name} is an unbounded NUMERIC; declare a precision and scale to enable aggregate pushdown", context.clause)))
        .transpose()?;
    Ok(JoinGroupColumn {
        plan_position,
        attno,
        field_name,
        output_index,
        numeric_scale,
        transform,
    })
}

unsafe fn extract_raw_aggregate_entry(
    aggref: *mut pg_sys::Aggref,
    sources: &[JoinAggSource],
    plan: &crate::postgres::customscan::joinscan::build::RelNode,
    outer_root_id: crate::postgres::customscan::joinscan::build::PlannerRootId,
    output_index: usize,
    pdb_request: Option<PdbAggRequest>,
) -> Result<JoinAggregateEntry, String> {
    let aggfnoid = (*aggref).aggfnoid.to_u32();
    let has_distinct = !(*aggref).aggdistinct.is_null();
    let filter = (!(*aggref).aggfilter.is_null())
        .then(|| {
            FilterExpr::from_pg_node(
                (*aggref).aggfilter as *mut pg_sys::Node,
                &FilterExprBuildContext::Filter {
                    sources,
                    plan,
                    outer_root_id,
                },
            )
            .ok_or_else(|| {
                "aggregate FILTER cannot be translated for aggregate-on-join".to_string()
            })
        })
        .transpose()?;
    if crate::api::is_agg_funcoid(aggfnoid) {
        if has_distinct || !(*aggref).aggorder.is_null() {
            return Err("pdb.agg() does not accept DISTINCT or ORDER BY".into());
        }
        let mut request = pdb_request.unwrap_or(lower_pdb_agg(aggref, sources)?);
        request.assign_plan_positions(|field| {
            plan.plan_position(outer_root_id, field.rti, field.attno)
        })?;
        return Ok(JoinAggregateEntry {
            func_oid: aggfnoid,
            agg_kind: AggKind::PdbAgg(Box::new(request)),
            field_refs: Vec::new(),
            output_index,
            result_type_oid: (*aggref).aggtype,
            filter,
            distinct: false,
            order_by: Vec::new(),
            numeric: None,
        });
    }
    let mut agg_kind = classify_aggregate_oid(aggfnoid, (*aggref).aggstar, has_distinct)
        .ok_or_else(|| {
            crate::postgres::catalog::lookup_fully_qualified_func_name(pg_sys::Oid::from(aggfnoid))
                .map(|name| format!("unsupported aggregate function: {name}"))
                .unwrap_or_else(|| format!("unsupported aggregate function OID: {aggfnoid}"))
        })?;
    let is_string_agg = matches!(agg_kind, AggKind::StringAgg(_));
    if is_string_agg {
        agg_kind =
            AggKind::StringAgg(extract_string_agg_separator(aggref).unwrap_or_else(|| ",".into()));
    }
    let field_refs =
        extract_aggref_field_refs(aggref, sources, is_string_agg, plan, outer_root_id)?;
    let order_by = extract_aggref_order_by(aggref, sources, plan, outer_root_id)?;
    let numeric = numeric_agg_field_type(&agg_kind, &field_refs, has_distinct)?;
    Ok(JoinAggregateEntry {
        func_oid: aggfnoid,
        agg_kind,
        field_refs,
        output_index,
        result_type_oid: (*aggref).aggtype,
        filter,
        distinct: has_distinct,
        order_by,
        numeric,
    })
}

/// The `pdb.agg()` calls of the grouping output, lowered to decide the route.
///
/// One visible target expression can contain multiple aggregate calls, so each
/// request is associated with its complete `Aggref`, not a target-list index.
pub struct PdbAggRoute {
    requests: Vec<(*mut pg_sys::Aggref, PdbAggRequest)>,
}

impl PdbAggRoute {
    /// A spec reads a NUMERIC field, which only the DataFusion backend can
    /// aggregate.
    pub fn references_numeric(&self) -> bool {
        self.requests
            .iter()
            .flat_map(|(_, request)| request.fields())
            .any(|field| field.field_type.is_numeric())
    }

    /// Take the pre-lowered request for this aggregate expression.
    ///
    /// The route is planner-only and this is consumed exactly once while the
    /// runtime target list is built. Structural equality also covers copied
    /// planner nodes without relying on pointer identity.
    unsafe fn take_request(&mut self, aggref: *mut pg_sys::Aggref) -> Option<PdbAggRequest> {
        let index = self.requests.iter().position(|(known, _)| {
            pg_sys::equal(
                (*known).cast::<core::ffi::c_void>(),
                aggref.cast::<core::ffi::c_void>(),
            )
        })?;
        Some(self.requests.swap_remove(index).1)
    }
}

/// Lower every `pdb.agg()` in the grouping output, fields included, to decide
/// the route. `None` when a spec does not lower: a single-table query then stays
/// on the Tantivy path, which runs every Tantivy aggregation, so the user never
/// sees an error for a query the index can answer.
pub unsafe fn pdb_agg_route(
    root: *mut pg_sys::PlannerInfo,
    input_rel: &pg_sys::RelOptInfo,
    shape: GroupingShape,
) -> Option<PdbAggRoute> {
    let sources = collect_join_agg_sources(root, input_rel);
    let mut requests: Vec<(*mut pg_sys::Aggref, PdbAggRequest)> = Vec::new();
    for expr in shape.target_exprs().iter_ptr() {
        for aggref in super::targetlist::find_aggrefs_in_expr(expr.cast()) {
            if crate::api::is_agg_funcoid((*aggref).aggfnoid.to_u32())
                && !requests.iter().any(|(known, _)| {
                    pg_sys::equal(
                        (*known).cast::<core::ffi::c_void>(),
                        aggref.cast::<core::ffi::c_void>(),
                    )
                })
            {
                requests.push((aggref, lower_pdb_agg(aggref, &sources).ok()?));
            }
        }
    }
    Some(PdbAggRoute { requests })
}

/// Lower a `pdb.agg()` call into its DataFusion request. The spec must be a
/// constant: field validation and the grouping-set layout both need it at plan
/// time, the same as on the Tantivy backend.
unsafe fn lower_pdb_agg(
    aggref: *mut pg_sys::Aggref,
    sources: &[JoinAggSource],
) -> Result<PdbAggRequest, String> {
    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    let arg_expr = |i: usize| args.get_ptr(i).map(|arg| (*arg).expr as *mut pg_sys::Node);
    let (spec, visibility) = arg_expr(0)
        .and_then(|spec_arg| pdb_agg_spec((*aggref).aggfnoid.to_u32(), spec_arg, arg_expr(1)))
        .ok_or("pdb.agg argument must be a constant for aggregate pushdown")?;
    PdbAggRequest::lower(spec, visibility, &|field| {
        let resolved = resolve_source_field(sources, field)?;
        Ok(PdbAggFieldRef {
            rti: resolved.source.rti,
            attno: resolved.attno,
            field_name: resolved.field_name,
            field_type: resolved.field_type,
            plan_position: 0,
        })
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
    plan: &crate::postgres::customscan::joinscan::build::RelNode,
    outer_root_id: crate::postgres::customscan::joinscan::build::PlannerRootId,
) -> Result<Vec<JoinAggColRef>, String> {
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
        // Reject complex expressions like COALESCE(score, 0) - find_one_var
        // would strip the wrapper, causing DataFusion to compute e.g. SUM(score)
        // instead of the intended SUM(COALESCE(score, 0)).
        let var = unwrap_to_var(expr as *mut pg_sys::Node).ok_or(
            "aggregate argument must be a direct column reference; \
                     wrapped expressions (COALESCE, casts) are not supported for aggregate-on-join",
        )?;

        let rti = (*var).varno as pg_sys::Index;
        let attno = (*var).varattno;

        let (source, attno, field_name, plan_position) = if let Some(unnest_info) =
            plan.find_lateral_unnest(rti)
        {
            let source =
                find_source_by_rti(sources, unnest_info.source_rti.0, "aggregate argument")?;
            let fn_name = unnest_info.field_name.clone();
            let pp = plan
                .plan_position(
                    outer_root_id,
                    unnest_info.source_rti.0,
                    unnest_info.source_attno,
                )
                .ok_or_else(|| {
                    format!(
                        "aggregate argument (RTI={rti}) does not resolve to a unique \
                             output-visible source in the plan tree"
                    )
                })?;
            (source, unnest_info.source_attno, fn_name, pp)
        } else {
            let source = find_source_by_rti(sources, rti, "aggregate argument")?;
            let fn_name = source.column_name(attno).ok_or_else(|| {
                let alias =
                    RelationAlias::new(source.alias.as_deref()).display(source.rti as usize);
                format!(
                    "aggregate argument {} is not columnar indexed",
                    get_attname_safe(Some(source.relid), attno, &alias)
                )
            })?;
            let pp = plan
                    .plan_position(outer_root_id, rti, attno)
                    .ok_or_else(|| {
                        format!(
                            "aggregate argument (RTI={rti}, attno={attno}) does not resolve to a unique \
                             output-visible source in the plan tree"
                        )
                    })?;
            (source, attno, fn_name, pp)
        };

        let numeric = source
            .bm25_index
            .as_ref()
            .and_then(|i| i.schema().ok())
            .and_then(|s| s.numeric_field_type(&field_name))
            .map(|(field_type, _)| field_type);

        refs.push(JoinAggColRef {
            plan_position,
            attno,
            field_name,
            numeric,
        });
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
    plan: &crate::postgres::customscan::joinscan::build::RelNode,
    outer_root_id: crate::postgres::customscan::joinscan::build::PlannerRootId,
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

        let source = find_source_by_rti(sources, rti, "aggregate ORDER BY")?;

        let field_name = source.column_name(attno).ok_or_else(|| {
            format!(
                "could not resolve field name for aggregate ORDER BY (RTI={}, attno={})",
                rti, attno
            )
        })?;

        let direction =
            SortDirection::from_sort_op((*clause_ptr).sortop, (*clause_ptr).nulls_first)
                .ok_or_else(|| {
                    format!(
                        "could not determine sort direction for aggregate ORDER BY (sortop={})",
                        (*clause_ptr).sortop.to_u32()
                    )
                })?;

        let plan_position = plan
            .plan_position(outer_root_id, rti, attno)
            .ok_or_else(|| {
                format!(
                    "aggregate ORDER BY column (RTI={rti}, attno={attno}) does not resolve to a \
                     unique output-visible source in the plan tree"
                )
            })?;

        entries.push(AggOrderByEntry {
            plan_position,
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
pub(in crate::postgres::customscan) unsafe fn unwrap_to_var(
    mut node: *mut pg_sys::Node,
) -> Option<*mut pg_sys::Var> {
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
