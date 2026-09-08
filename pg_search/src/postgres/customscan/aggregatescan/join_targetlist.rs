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
/// `runtime` is serialized in `PrivateData` for execution. The expression
/// pointers live only during planning; [`Self::into_parts`] wraps them into
/// the raw tuple's `TargetEntry`s for PostgreSQL's setrefs pass.
pub struct ExtractedDataFusionTarget {
    runtime: JoinAggregateTargetList,
    group_exprs: Vec<*mut pg_sys::Node>,
    aggrefs: Vec<*mut pg_sys::Aggref>,
}

impl ExtractedDataFusionTarget {
    pub fn targetlist(&self) -> &JoinAggregateTargetList {
        &self.runtime
    }

    /// The runtime metadata and the raw `TargetEntry`s it describes, both
    /// listing groups first and aggregates second. The entries wrap the
    /// planner's own nodes; the caller copies them into the plan.
    #[must_use]
    pub unsafe fn into_parts(self) -> (JoinAggregateTargetList, PgList<pg_sys::TargetEntry>) {
        let exprs = self
            .group_exprs
            .iter()
            .copied()
            .chain(self.aggrefs.iter().map(|aggref| aggref.cast()));
        let mut scan_tlist = PgList::new();
        for (offset, expr) in exprs.enumerate() {
            scan_tlist.push(pg_sys::makeTargetEntry(
                expr.cast(),
                offset as pg_sys::AttrNumber + 1,
                std::ptr::null_mut(),
                false,
            ));
        }
        (self.runtime, scan_tlist)
    }

    /// Raw aggregate column whose `Aggref` is structurally equal to `expr`.
    pub unsafe fn aggregate_index(&self, expr: *mut pg_sys::Node) -> Option<usize> {
        position_equal(&self.aggrefs, expr)
    }

    /// Raw group column whose expression is structurally equal to `expr`.
    pub unsafe fn group_index(&self, expr: *mut pg_sys::Node) -> Option<usize> {
        position_equal(&self.group_exprs, expr)
    }
}

unsafe fn position_equal<T>(nodes: &[*mut T], node: *mut pg_sys::Node) -> Option<usize> {
    nodes
        .iter()
        .position(|known| pg_sys::equal((*known).cast(), node.cast()))
}

unsafe fn push_unique<T>(nodes: &mut Vec<*mut T>, node: *mut T) {
    if position_equal(nodes, node.cast()).is_none() {
        nodes.push(node);
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
                "DATE(timestamptz) grouping is not pushed down because it depends on the \
                 session TimeZone"
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
        return Err(
            "DATE(timestamp) grouping requires a bare timestamp column, casts and other \
             expressions are not supported"
                .into(),
        );
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

/// Collect the output Vars and Aggrefs of `expr`.
///
/// GROUP BY expressions and aggregate arguments are boundaries: setrefs first
/// matches the former as whole expressions, while the latter are evaluated by
/// DataFusion. Every other Var is one PostgreSQL admitted through functional
/// dependency and must be carried in the raw tuple as a group column.
unsafe fn collect_output_nodes(
    expr: *mut pg_sys::Node,
    group_exprs: &mut Vec<*mut pg_sys::Node>,
    aggrefs: &mut Vec<*mut pg_sys::Aggref>,
) {
    struct WalkerContext<'a> {
        group_exprs: &'a mut Vec<*mut pg_sys::Node>,
        aggrefs: &'a mut Vec<*mut pg_sys::Aggref>,
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        let ctx = &mut *(context as *mut WalkerContext<'_>);
        if position_equal(ctx.group_exprs, node).is_some() {
            return false;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_Aggref => {
                push_unique(ctx.aggrefs, node.cast());
                false
            }
            pg_sys::NodeTag::T_Var => {
                ctx.group_exprs.push(node);
                false
            }
            _ => pg_sys::expression_tree_walker(node, Some(walker), context),
        }
    }

    let mut context = WalkerContext {
        group_exprs,
        aggrefs,
    };
    walker(expr, (&mut context as *mut WalkerContext).cast());
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
    let outer_root_id =
        crate::postgres::customscan::joinscan::build::PlannerRootId::from(args.root);

    let mut group_exprs: Vec<*mut pg_sys::Node> = Vec::new();
    if shape.is_distinct() {
        for (idx, expr) in target_exprs.iter_ptr().enumerate() {
            if super::expr_contains_aggref(expr.cast()) {
                continue;
            }
            if (*expr).type_ != pg_sys::NodeTag::T_Var {
                return Err(format!(
                    "DISTINCT column {} is an expression; only plain columns are pushed down",
                    idx + 1
                ));
            }
            push_unique(&mut group_exprs, expr.cast());
        }
    } else {
        let exprs = pg_sys::get_sortgrouplist_exprs((*parse).groupClause, (*parse).targetList);
        for expr in PgList::<pg_sys::Node>::from_pg(exprs).iter_ptr() {
            push_unique(&mut group_exprs, expr);
        }
    }

    let mut aggrefs: Vec<*mut pg_sys::Aggref> = Vec::new();
    let having = (*parse).havingQual;
    let output_exprs = target_exprs
        .iter_ptr()
        .map(|expr| expr.cast::<pg_sys::Node>())
        .chain((!having.is_null()).then_some(having));
    for expr in output_exprs {
        collect_output_nodes(expr, &mut group_exprs, &mut aggrefs);
    }

    let context = RawColumnContext {
        root: args.root,
        sources,
        plan,
        outer_root_id,
        shape,
        pdb_agg_funcoids: crate::api::agg_funcoids(),
    };
    let mut group_columns = Vec::with_capacity(group_exprs.len());
    for (idx, expr) in group_exprs.iter().enumerate() {
        group_columns.push(extract_raw_group_column(&context, *expr, idx + 1)?);
    }
    let mut aggregates = Vec::with_capacity(aggrefs.len());
    for aggref in &aggrefs {
        let pdb_request = pdb_route
            .as_mut()
            .and_then(|route| route.take_request(*aggref));
        aggregates.push(extract_raw_aggregate_entry(&context, *aggref, pdb_request)?);
    }

    if group_columns.is_empty() && aggregates.is_empty() {
        return Err(format!(
            "{} target contains no supported grouping columns or aggregates",
            shape.clause_name()
        ));
    }
    Ok(ExtractedDataFusionTarget {
        runtime: JoinAggregateTargetList {
            group_columns,
            aggregates,
        },
        group_exprs,
        aggrefs,
    })
}

struct RawColumnContext<'a> {
    root: *mut pg_sys::PlannerInfo,
    sources: &'a [JoinAggSource],
    plan: &'a crate::postgres::customscan::joinscan::build::RelNode,
    outer_root_id: crate::postgres::customscan::joinscan::build::PlannerRootId,
    shape: GroupingShape,
    pdb_agg_funcoids: [u32; 3],
}

struct ResolvedVar<'a> {
    source: &'a JoinAggSource,
    attno: pg_sys::AttrNumber,
    field_name: String,
    plan_position: usize,
}

/// Resolve a Var, or the source column behind a lateral `unnest()` Var, to the
/// join source and plan position DataFusion binds it to.
unsafe fn resolve_var_source<'a>(
    context: &RawColumnContext<'a>,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
    what: &str,
) -> Result<ResolvedVar<'a>, String> {
    let (source_rti, attno, unnest_field) = match context.plan.find_lateral_unnest(rti) {
        Some(unnest) => (
            unnest.source_rti.0,
            unnest.source_attno,
            Some(unnest.field_name.clone()),
        ),
        None => (rti, attno, None),
    };
    let source = find_source_by_rti(context.sources, source_rti, what)?;
    let field_name = match unnest_field {
        Some(field_name) => field_name,
        None => source.column_name(attno).ok_or_else(|| {
            let alias = RelationAlias::new(source.alias.as_deref()).display(source.rti as usize);
            format!(
                "{what} {} is not columnar indexed",
                get_attname_safe(Some(source.relid), attno, &alias)
            )
        })?,
    };
    let plan_position = context
        .plan
        .plan_position(context.outer_root_id, source_rti, attno)
        .ok_or_else(|| {
            format!(
                "{what} (RTI={rti}, attno={attno}) does not resolve to a unique \
                 output-visible source in the plan tree"
            )
        })?;
    Ok(ResolvedVar {
        source,
        attno,
        field_name,
        plan_position,
    })
}

/// `position` is the 1-based GROUP BY item. Group expressions come first in
/// GROUP BY order and the functionally dependent Vars appended after them are
/// plain columns, so only real GROUP BY items can reach the expression error.
unsafe fn extract_raw_group_column(
    context: &RawColumnContext<'_>,
    expr: *mut pg_sys::Node,
    position: usize,
) -> Result<JoinGroupColumn, String> {
    let clause = context.shape.clause_name();
    let mut node = expr;
    let mut transform = GroupingTransform::Identity;
    if !context.shape.is_distinct()
        && let Some(var) = extract_timestamp_to_date_var(node)?
    {
        node = var.cast();
        transform = GroupingTransform::TimestampToDate;
    }
    let (attno, field_name, plan_position, numeric_source) = if (*node).type_
        == pg_sys::NodeTag::T_Var
    {
        let var = node.cast::<pg_sys::Var>();
        let resolved = resolve_var_source(
            context,
            (*var).varno as pg_sys::Index,
            (*var).varattno,
            &format!("{clause} column"),
        )?;
        (
            resolved.attno,
            resolved.field_name,
            resolved.plan_position,
            Some(resolved.source),
        )
    } else {
        assert!(
            !context.shape.is_distinct(),
            "DISTINCT group expressions are plain columns by construction"
        );
        let (var, field_name) =
            find_one_var_and_fieldname(VarContext::from_planner(context.root), expr).ok_or_else(
                || {
                    format!(
                        "GROUP BY item {position} is an unsupported expression; only plain columns \
                 and indexed expressions are pushed down"
                    )
                },
            )?;
        let rti = (*var).varno as pg_sys::Index;
        let attno = (*var).varattno;
        let plan_position = context
            .plan
            .plan_position(context.outer_root_id, rti, attno)
            .ok_or_else(|| {
                format!(
                    "GROUP BY expression at RTI {rti} (attno={attno}) does not resolve to a \
                     unique output-visible source in the plan tree"
                )
            })?;
        (
            attno,
            field_name.into_inner(),
            plan_position,
            find_source_by_rti(context.sources, rti, "GROUP BY expression").ok(),
        )
    };
    let numeric_scale = numeric_source
        .and_then(|source| source.bm25_index.as_ref())
        .and_then(|index| index.schema().ok())
        .and_then(|schema| schema.numeric_field_type(&field_name))
        .map(|(_, scale)| {
            scale.ok_or_else(|| {
                format!(
                    "{clause} column {field_name} is an unbounded NUMERIC; declare a precision \
                     and scale to enable aggregate pushdown"
                )
            })
        })
        .transpose()?;
    Ok(JoinGroupColumn {
        plan_position,
        attno,
        field_name,
        numeric_scale,
        transform,
    })
}

unsafe fn extract_raw_aggregate_entry(
    context: &RawColumnContext<'_>,
    aggref: *mut pg_sys::Aggref,
    pdb_request: Option<PdbAggRequest>,
) -> Result<JoinAggregateEntry, String> {
    let aggfnoid = (*aggref).aggfnoid.to_u32();
    let has_distinct = !(*aggref).aggdistinct.is_null();
    let filter = (!(*aggref).aggfilter.is_null())
        .then(|| {
            FilterExpr::from_pg_node(
                (*aggref).aggfilter as *mut pg_sys::Node,
                &FilterExprBuildContext::Filter {
                    sources: context.sources,
                    plan: context.plan,
                    outer_root_id: context.outer_root_id,
                },
            )
            .ok_or_else(|| {
                "aggregate FILTER cannot be translated for aggregate-on-join".to_string()
            })
        })
        .transpose()?;
    if context.pdb_agg_funcoids.contains(&aggfnoid) {
        if has_distinct || !(*aggref).aggorder.is_null() {
            return Err("pdb.agg() does not accept DISTINCT or ORDER BY".into());
        }
        let mut request = match pdb_request {
            Some(request) => request,
            None => lower_pdb_agg(aggref, context.sources)?,
        };
        request.assign_plan_positions(|field| {
            context
                .plan
                .plan_position(context.outer_root_id, field.rti, field.attno)
        })?;
        return Ok(JoinAggregateEntry {
            func_oid: aggfnoid,
            agg_kind: AggKind::PdbAgg(Box::new(request)),
            field_refs: Vec::new(),
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
    let field_refs = extract_aggref_field_refs(context, aggref, is_string_agg)?;
    let order_by =
        extract_aggref_order_by(aggref, context.sources, context.plan, context.outer_root_id)?;
    let numeric = numeric_agg_field_type(&agg_kind, &field_refs, has_distinct)?;
    Ok(JoinAggregateEntry {
        func_oid: aggfnoid,
        agg_kind,
        field_refs,
        result_type_oid: (*aggref).aggtype,
        filter,
        distinct: has_distinct,
        order_by,
        numeric,
    })
}

/// The `pdb.agg()` calls of the grouping output, lowered to decide the route.
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

    /// Matched by `equal`, since planner copies break pointer identity.
    unsafe fn position(&self, aggref: *mut pg_sys::Aggref) -> Option<usize> {
        self.requests
            .iter()
            .position(|(known, _)| pg_sys::equal((*known).cast(), aggref.cast()))
    }

    unsafe fn take_request(&mut self, aggref: *mut pg_sys::Aggref) -> Option<PdbAggRequest> {
        let index = self.position(aggref)?;
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
    let agg_funcoids = crate::api::agg_funcoids();
    let mut route = PdbAggRoute {
        requests: Vec::new(),
    };
    for expr in shape.target_exprs().iter_ptr() {
        for aggref in super::targetlist::find_aggrefs_in_expr(expr.cast()) {
            if agg_funcoids.contains(&(*aggref).aggfnoid.to_u32())
                && route.position(aggref).is_none()
            {
                route
                    .requests
                    .push((aggref, lower_pdb_agg(aggref, &sources).ok()?));
            }
        }
    }
    Some(route)
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
    context: &RawColumnContext<'_>,
    aggref: *mut pg_sys::Aggref,
    is_string_agg: bool,
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

        let resolved = resolve_var_source(
            context,
            (*var).varno as pg_sys::Index,
            (*var).varattno,
            "aggregate argument",
        )?;
        let numeric = resolved
            .source
            .bm25_index
            .as_ref()
            .and_then(|i| i.schema().ok())
            .and_then(|s| s.numeric_field_type(&resolved.field_name))
            .map(|(field_type, _)| field_type);

        refs.push(JoinAggColRef {
            plan_position: resolved.plan_position,
            attno: resolved.attno,
            field_name: resolved.field_name,
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
