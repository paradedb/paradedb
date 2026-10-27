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

use crate::api::window_aggregate::{PlaceholderId, make_window_agg_placeholder};
use crate::api::{MvccVisibility, is_agg_funcoid, is_combine_agg_funcoid, pdb_agg_spec};
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::datafusion_build::{
    FilterExprBuildContext, JoinAggSource, ResolutionSource,
};
use crate::postgres::customscan::aggregatescan::datafusion_exec::{
    numeric_avg_func_with_args, numeric_sum_func_with_args,
};
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, classify_aggregate_oid, lower_pdb_agg_call, unwrap_to_var,
};
use crate::postgres::customscan::aggregatescan::pdb_agg::PdbAggRequest;
use crate::postgres::customscan::aggregatescan::privdat::FilterExpr;
use crate::postgres::customscan::collation_semantics::{CollationOperation, collation_supports};
use crate::postgres::customscan::joinscan::planning::resolve_fast_field_from_join_sources;
use crate::postgres::deparse::deparse_planner_expr;
use crate::postgres::utils::strip_relabel;
use crate::schema::SearchFieldType;
use datafusion::common::DataFusionError;
use datafusion::common::utils::expr::COUNT_STAR_EXPANSION;
use datafusion::functions_aggregate::{average, count, min_max, sum};
use datafusion::logical_expr::{AggregateUDF, Expr, lit};
use pgrx::pg_sys::{FRAMEOPTION_NONDEFAULT, PlannerInfo, WindowFunc};
use pgrx::{PgList, pg_sys};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

use super::build::{JoinSource, PlannerRootId, RelNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportedWindowAggType {
    Count,
    CountStar,
    Sum,
    Avg,
    Min,
    Max,
}
impl SupportedWindowAggType {
    pub fn from_funcoid(oid: pg_sys::Oid, aggstar: bool) -> Option<Self> {
        match classify_aggregate_oid(oid.to_u32(), aggstar, false) {
            Some(AggKind::Count) => Some(SupportedWindowAggType::Count),
            Some(AggKind::CountStar) => Some(SupportedWindowAggType::CountStar),
            Some(AggKind::Sum) => Some(SupportedWindowAggType::Sum),
            Some(AggKind::Avg) => Some(SupportedWindowAggType::Avg),
            Some(AggKind::Min) => Some(SupportedWindowAggType::Min),
            Some(AggKind::Max) => Some(SupportedWindowAggType::Max),
            _ => None,
        }
    }

    /// Lower a SQL window aggregate to its DataFusion
    /// [`AggregateUDF`] and call
    /// arguments, routing `NUMERIC` `SUM` and `AVG` to the decimal accumulators.
    pub fn udaf_with_args(
        self,
        input: Option<Expr>,
        numeric: Option<&SearchFieldType>,
    ) -> datafusion::common::Result<(Arc<AggregateUDF>, Vec<Expr>)> {
        let require_input = || {
            input.ok_or_else(|| {
                DataFusionError::Internal(format!("{self} OVER () must have an input expression"))
            })
        };
        Ok(match self {
            Self::CountStar => (count::count_udaf(), vec![lit(COUNT_STAR_EXPANSION)]),
            Self::Count => (count::count_udaf(), vec![require_input()?]),
            Self::Sum => {
                let ce = require_input()?;
                match numeric {
                    Some(field_type) => numeric_sum_func_with_args(ce, field_type),
                    None => (sum::sum_udaf(), vec![ce]),
                }
            }
            Self::Avg => {
                let ce = require_input()?;
                match numeric {
                    Some(field_type) => numeric_avg_func_with_args(ce, field_type),
                    None => (average::avg_udaf(), vec![ce]),
                }
            }
            Self::Min => (min_max::min_udaf(), vec![require_input()?]),
            Self::Max => (min_max::max_udaf(), vec![require_input()?]),
        })
    }
}

impl fmt::Display for SupportedWindowAggType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountStar => write!(f, "COUNT(*)"),
            Self::Count => write!(f, "COUNT"),
            Self::Sum => write!(f, "SUM"),
            Self::Avg => write!(f, "AVG"),
            Self::Min => write!(f, "MIN"),
            Self::Max => write!(f, "MAX"),
        }
    }
}

/// A SQL aggregate and the column it reads, `None` for `COUNT(*)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlWindowAggDef {
    agg_type: SupportedWindowAggType,
    col_info: Option<ColumnInfo>,
}
impl SqlWindowAggDef {
    pub fn agg_type(&self) -> SupportedWindowAggType {
        self.agg_type
    }

    pub fn col_info(&self) -> Option<&ColumnInfo> {
        self.col_info.as_ref()
    }

    pub fn arg_field_type(&self) -> Option<&SearchFieldType> {
        self.col_info.as_ref().and_then(|ci| ci.field_type.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowAggDef {
    Sql(SqlWindowAggDef),
    /// `pdb.agg()`, lowered to the request the aggregate scan runs.
    PdbAgg(Box<PdbAggRequest>),
}
impl WindowAggDef {
    /// Lower a `pdb.agg(...) OVER ()` under the constraints the aggregate scan
    /// puts on the spec. One thing it accepts is turned down here: a visibility
    /// other than the default, since the rows the aggregate reads are the rows
    /// the scan returns and those have to be visible.
    fn try_pdb_agg_from_window_func(
        wf: &WindowFunc,
        sources: &[&JoinSource],
    ) -> Result<Self, String> {
        let args = unsafe { PgList::<pg_sys::Node>::from_pg(wf.args) };
        let sources: Vec<ResolutionSource> =
            sources.iter().map(|s| ResolutionSource::from(*s)).collect();
        let request = unsafe {
            lower_pdb_agg_call(
                wf.winfnoid.to_u32(),
                args.get_ptr(0),
                args.get_ptr(1),
                &sources,
            )?
        };
        if request.visibility != MvccVisibility::default() {
            return Err(format!(
                "pdb.agg() as a window function over a join only supports visibility '{}'",
                MvccVisibility::default().as_sql_value()
            ));
        }
        Ok(Self::PdbAgg(Box::new(request)))
    }

    /// The SQL aggregate, if this is one.
    pub fn sql(&self) -> Option<&SqlWindowAggDef> {
        match self {
            Self::Sql(sql) => Some(sql),
            Self::PdbAgg(_) => None,
        }
    }

    /// The `pdb.agg()` request, if this is one.
    pub fn pdb_agg(&self) -> Option<&PdbAggRequest> {
        match self {
            Self::Sql(_) => None,
            Self::PdbAgg(request) => Some(request),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub rti: pg_sys::Index,
    pub attno: pg_sys::AttrNumber,
    pub field_type: Option<SearchFieldType>,
}
impl ColumnInfo {
    pub fn new(
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
        field_type: Option<SearchFieldType>,
    ) -> Self {
        Self {
            rti,
            attno,
            field_type,
        }
    }
}

impl PartialEq for ColumnInfo {
    /// `field_type` is derived from the column at extraction.
    fn eq(&self, other: &Self) -> bool {
        self.rti == other.rti && self.attno == other.attno
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResultType(pub pg_sys::Oid);

/// Identity of a window function within the query's target list: the owning
/// target entry's resno plus the window function's position within that entry
/// (in expression-walker visit order — an expression entry can embed several).
/// Extraction runs twice — once in `validate_and_build_clause` and again when
/// `plan_custom_path` re-resolves the scan target list — and this identity is
/// what re-associates the two passes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowAggId {
    resno: pg_sys::AttrNumber,
    ordinal: usize,
}
impl WindowAggId {
    /// The single window function in a bare `agg OVER ()` target entry.
    pub fn bare(resno: pg_sys::AttrNumber) -> Self {
        Self { resno, ordinal: 0 }
    }

    /// The `ordinal`-th window function embedded in an expression entry.
    pub fn nested(resno: pg_sys::AttrNumber, ordinal: usize) -> Self {
        Self { resno, ordinal }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAgg {
    pub agg_def: WindowAggDef,
    pub filter: Option<FilterExpr>,
    pub result_type: ResultType,
    pub id: WindowAggId,
}
impl WindowAgg {
    /// True when `other` computes the same aggregate over the same input
    /// — identity (`id`) excluded.
    pub fn same_spec(&self, other: &Self) -> bool {
        self.agg_def == other.agg_def
            && self.result_type.0 == other.result_type.0
            && self.filter == other.filter
    }
}

/// Sentinel `varno` for window-aggregate inputs in serialized projection
/// expressions (see [`rewrite_window_funcs_to_sentinels`]). Real range-table
/// indexes are 1-based, so 0 cannot collide with a source relation.
pub const WINDOW_SENTINEL_VARNO: pg_sys::Index = 0;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct WindowAggIndex(usize);
impl WindowAggIndex {
    pub fn as_col_name(&self) -> String {
        WindowAggColumn::new(*self).to_string()
    }

    /// The `varattno` a sentinel Var carries for this window aggregate
    /// (1-based; 0 is reserved so sentinel attnos stay positive).
    pub fn to_sentinel_attno(self) -> pg_sys::AttrNumber {
        (self.0 + 1) as pg_sys::AttrNumber
    }

    /// Decode a sentinel Var's `varattno` back into the aggregate's index.
    pub fn from_sentinel_attno(attno: pg_sys::AttrNumber) -> Option<Self> {
        (attno > 0).then(|| Self((attno - 1) as usize))
    }
}

pub struct WindowAggColumn(WindowAggIndex);
impl WindowAggColumn {
    const PREFIX: &'static str = "window_agg_";

    pub fn new(index: WindowAggIndex) -> Self {
        WindowAggColumn(index)
    }

    pub fn index(&self) -> WindowAggIndex {
        self.0
    }
}
impl fmt::Display for WindowAggColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.0.0 + 1)
    }
}
impl TryFrom<&str> for WindowAggColumn {
    type Error = ();

    fn try_from(col_name: &str) -> Result<Self, Self::Error> {
        // The name is 1-based (see `Display`).
        let index = col_name
            .strip_prefix(Self::PREFIX)
            .ok_or(())?
            .parse::<usize>()
            .map_err(|_| ())?
            .checked_sub(1)
            .ok_or(())?;
        Ok(Self::new(WindowAggIndex(index)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WindowAggList(Vec<WindowAgg>);
impl WindowAggList {
    pub fn new(aggs: Vec<WindowAgg>) -> Self {
        Self(aggs)
    }

    pub fn get(&self, index: WindowAggIndex) -> Option<&WindowAgg> {
        self.0.get(index.0)
    }

    pub fn find_index(&self, id: WindowAggId) -> Option<WindowAggIndex> {
        self.0.iter().position(|wa| wa.id == id).map(WindowAggIndex)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &WindowAgg> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WindowAgg> {
        self.0.iter_mut()
    }

    pub fn iter_indexed(&self) -> impl Iterator<Item = (WindowAggIndex, &WindowAgg)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, wa)| (WindowAggIndex(i), wa))
    }

    /// The index of the first entry computing the same aggregate as
    /// `index`'s entry. Identical window aggregates (e.g. `COUNT(*) OVER ()`
    /// in several target entries) share one window column and we only need to compute the aggregate
    /// once
    pub fn canonical_index(&self, index: WindowAggIndex) -> WindowAggIndex {
        let Some(wa) = self.get(index) else {
            return index;
        };
        self.0
            .iter()
            .position(|other| other.same_spec(wa))
            .map(WindowAggIndex)
            .unwrap_or(index)
    }
}

/// Returns `true` iff `winref` resolves to a `WindowClause` in `parse.windowClause` with an
/// empty `OVER ()` specification (no `PARTITION BY`, no `ORDER BY`, and default framing).
pub unsafe fn has_empty_over_clause(parse: *const pg_sys::Query, winref: pg_sys::Index) -> bool {
    if parse.is_null() {
        return false;
    }
    let Some(clause) = PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause)
        .iter_ptr()
        .find(|wc| (**wc).winref == winref)
    else {
        return false;
    };
    (*clause).partitionClause.is_null()
        && (*clause).orderClause.is_null()
        && ((*clause).frameOptions & FRAMEOPTION_NONDEFAULT as i32) == 0
}

/// Extracts an `OVER ()` window aggregate for an **ungrouped** query (`JoinScan`),
/// where arguments resolve directly to base-table `Var`s or `pdb.agg(...) OVER ()`.
///
/// For the **post-`GROUP BY`** counterpart (`AggregateScan`), where Stage 2
/// `OVER ()` window aggregates resolve against Stage 1 `GROUP BY` columns,
/// per-group `Aggref`s, or `pdb.combine_agg(pdb.agg(...)) OVER ()`, see
/// `aggregatescan::join_targetlist::extract_raw_window_entry`.
pub fn extract_window_agg(
    wf: *const WindowFunc,
    root: *mut PlannerInfo,
    plan: &RelNode,
    id: WindowAggId,
) -> Result<WindowAgg, String> {
    assert!(!wf.is_null());
    assert!(!root.is_null());
    let wf = unsafe { &*wf };
    let parse = unsafe { &*((*root).parse) };

    if !wf.winagg {
        return Err(
            "only simple (sum/min/max/avg/count) window functions are supported".to_string(),
        );
    }

    if !unsafe { has_empty_over_clause(parse, wf.winref) } {
        return Err(
            "only bare window functions of the style 'agg OVER ()' are supported".to_string(),
        );
    }

    let all_sources = plan.sources();

    let filter = if !wf.aggfilter.is_null() {
        let outer_root_id = PlannerRootId::from(root);
        let join_agg_sources: Vec<_> = plan
            .output_sources()
            .into_iter()
            .filter(|s| s.root_id == Some(outer_root_id))
            .map(JoinAggSource::from)
            .collect();
        let filter_ctx = FilterExprBuildContext::Filter {
            root,
            sources: &join_agg_sources,
            plan,
            outer_root_id,
        };

        let f = match FilterExpr::from_pg_node(wf.aggfilter as *mut pg_sys::Node, &filter_ctx) {
            Some(filter) => filter,
            None => {
                return Err("unsupported window function FILTER clause was provided".to_string());
            }
        };
        // verify each field the filter needs is columnar
        if f.collect_column_refs().iter().any(|cr| {
            resolve_fast_field_from_join_sources(
                &all_sources,
                cr.rti as pg_sys::Index,
                cr.attno as i32,
            )
            .is_none()
        }) {
            return Err("window function FILTER clause depends on non-columnar field".to_string());
        }
        Some(f)
    } else {
        None
    };

    let agg_def = match SupportedWindowAggType::from_funcoid(wf.winfnoid, wf.winstar) {
        Some(agg_type) => WindowAggDef::Sql(SqlWindowAggDef {
            agg_type,
            col_info: extract_column_info(wf, &all_sources, agg_type)?,
        }),
        None if is_agg_funcoid(wf.winfnoid.to_u32()) => {
            WindowAggDef::try_pdb_agg_from_window_func(wf, &all_sources)?
        }
        None => return Err("unsupported window function was provided".to_string()),
    };

    Ok(WindowAgg {
        agg_def,
        filter,
        result_type: ResultType(wf.wintype),
        id,
    })
}

/// The column a SQL window aggregate reads, `None` for `COUNT(*)`.
fn extract_column_info(
    wf: &WindowFunc,
    sources: &[&JoinSource],
    agg_type: SupportedWindowAggType,
) -> Result<Option<ColumnInfo>, String> {
    let args = unsafe { PgList::<pg_sys::Node>::from_pg(wf.args) };
    match args.len() {
        0 => {
            assert!(wf.winstar); // count(*)
            Ok(None)
        }
        1 => {
            let arg = args.get_ptr(0).unwrap();

            let var = unwrap_to_var(arg).ok_or_else(|| {
                "window aggregate argument must be a direct column reference".to_string()
            })?;

            assert!(!var.is_null());
            let var = unsafe { *var };

            let Some(ff) = resolve_fast_field_from_join_sources(
                sources,
                var.varno as pg_sys::Index,
                var.varattno as i32,
            ) else {
                return Err("arguments to window aggregate must be fast fields".to_string());
            };

            // Unbounded NUMERIC has no declared scale to decode the
            // storage encoding with; reject at planning rather than
            // letting the DataFusion plan bake fail. COUNT never reads
            // the value, so it stays absorbable.
            if !matches!(
                agg_type,
                SupportedWindowAggType::Count | SupportedWindowAggType::CountStar
            ) && ff
                .field_type()
                .is_some_and(|ft| ft.is_numeric() && ft.numeric_scale().is_none())
            {
                return Err(
                    "window aggregates on an unbounded NUMERIC column are not supported; \
                     declare a precision and scale"
                        .to_string(),
                );
            }

            if matches!(
                agg_type,
                SupportedWindowAggType::Min | SupportedWindowAggType::Max
            ) && !unsafe { collation_supports(wf.inputcollid, CollationOperation::Ordering) }
            {
                return Err(format!(
                    "{agg_type} OVER () uses a collation whose ordering cannot be pushed down"
                ));
            }

            Ok(Some(ColumnInfo::new(
                var.varno as pg_sys::Index,
                var.varattno,
                ff.field_type().cloned(),
            )))
        }
        _ => Err("multi-argument window aggregates are not supported".to_string()),
    }
}

pub fn is_supported_window_agg_node(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }
    if let Some(wf) = unsafe { nodecast!(WindowFunc, T_WindowFunc, node) } {
        let wf = unsafe { &*wf };
        return SupportedWindowAggType::from_funcoid(wf.winfnoid, wf.winstar).is_some()
            || is_agg_funcoid(wf.winfnoid.to_u32());
    }
    false
}

struct SentinelRewriteCtx<'a> {
    resno: pg_sys::AttrNumber,
    next_ordinal: usize,
    window_aggs: &'a WindowAggList,
}

/// Copy `expr`, replacing each embedded `WindowFunc` (in expression-walker
/// visit order, matching the ordinals assigned during extraction) with a
/// sentinel Var: `varno = WINDOW_SENTINEL_VARNO`, `varattno` encoding the
/// aggregate's [`WindowAggIndex`], `vartype` the aggregate's wintype. The
/// executor never sees a WindowFunc — downstream translation resolves the
/// sentinel to the window step's output column (`resolve_var_to_df_col`) and
/// `PgExprUdf` treats it as an ordinary input. The original tree is not
/// modified.
pub unsafe fn rewrite_window_funcs_to_sentinels(
    expr: *mut pg_sys::Node,
    resno: pg_sys::AttrNumber,
    window_aggs: &WindowAggList,
) -> *mut pg_sys::Node {
    let mut ctx = SentinelRewriteCtx {
        resno,
        next_ordinal: 0,
        window_aggs,
    };
    sentinel_mutator(expr, std::ptr::addr_of_mut!(ctx).cast())
}

/// Rewrites embedded `WindowFunc` nodes into `paradedb.window_agg('<description>')`
/// placeholder `FuncExpr`s whose node-level `funcresulttype` is the aggregate's
/// `wintype` and whose argument `Const.consttypmod` encodes the `WindowFunc`'s
/// deduplicated [`PlaceholderId`].
///
/// PG18's EXPLAIN requires a `WindowAgg` plan node to deparse a `WindowFunc`
/// (commit 8b1b342544b6 removed the `OVER (?)` fallback), but the scan
/// absorbs the window so no such node exists — a raw `WindowFunc` left in
/// the plan's target lists makes every `EXPLAIN VERBOSE` fail with "could
/// not find window clause". The placeholder deparses as an ordinary function
/// call. It is never executed: `plan_custom_path` applies this rewriter
/// across `scan.plan.targetlist` and `custom_scan_tlist`, so two placeholders
/// are `equal()` in `setrefs.c` if and only if their original `WindowFunc`
/// AST nodes were `equal()`. The text argument is cosmetic EXPLAIN output
/// only — it is never parsed.
pub struct PlaceholderRewriter {
    root: *mut pg_sys::PlannerInfo,
    seen: Vec<*mut pg_sys::WindowFunc>,
}

impl PlaceholderRewriter {
    pub fn new(root: *mut pg_sys::PlannerInfo) -> Self {
        Self {
            root,
            seen: Vec::new(),
        }
    }

    pub unsafe fn rewrite_expr(&mut self, expr: *mut pg_sys::Node) -> *mut pg_sys::Node {
        placeholder_mutator(expr, std::ptr::from_mut(self).cast())
    }

    unsafe fn placeholder_id(&mut self, wf: *mut pg_sys::WindowFunc) -> PlaceholderId {
        if let Some(idx) = self
            .seen
            .iter()
            .position(|known| pg_sys::equal((*known).cast(), wf.cast()))
        {
            PlaceholderId(idx)
        } else {
            let idx = self.seen.len();
            self.seen.push(wf);
            PlaceholderId(idx)
        }
    }
}

#[pgrx::pg_guard]
unsafe extern "C-unwind" fn placeholder_mutator(
    node: *mut pg_sys::Node,
    context: *mut core::ffi::c_void,
) -> *mut pg_sys::Node {
    if node.is_null() {
        return std::ptr::null_mut();
    }

    if let Some(wf) = nodecast!(WindowFunc, T_WindowFunc, node) {
        let ctx = &mut *context.cast::<PlaceholderRewriter>();
        let placeholder_id = ctx.placeholder_id(wf);

        // A ruleutils deparse of the WindowFunc hits the same PG18 error
        // this rewrite exists to avoid (deparse_planner_expr catches it and
        // returns None on PG18; on older versions it renders `OVER (?)`),
        // so fall back to a description built from the node itself.
        let description = deparse_planner_expr(ctx.root, node)
            .unwrap_or_else(|| describe_window_func(&*wf, ctx.root));

        match make_window_agg_placeholder(
            &description,
            Some(placeholder_id),
            (*wf).wintype,
            (*wf).wincollid,
        ) {
            Some(placeholder) => return placeholder.cast(),
            // Should not happen once the extension is installed; leave the
            // node for PostgreSQL to complain about rather than panicking.
            None => return node,
        }
    }

    #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
    {
        let fnptr = placeholder_mutator as *const ();
        let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
            std::mem::transmute(fnptr);
        pg_sys::expression_tree_mutator(node, Some(mutator), context)
    }

    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    {
        pg_sys::expression_tree_mutator_impl(node, Some(placeholder_mutator), context)
    }
}

/// Human-readable description of a window aggregate for the EXPLAIN
/// placeholder, e.g. `count(*) OVER ()` or `sum(price) OVER ()`.
///
/// This string is strictly cosmetic for `EXPLAIN VERBOSE`; `setrefs.c`
/// identity is carried by [`PlaceholderId`] in
/// the placeholder argument's `Const.consttypmod`.
unsafe fn describe_window_func(wf: &pg_sys::WindowFunc, root: *mut pg_sys::PlannerInfo) -> String {
    let func_name = {
        let name_ptr = pg_sys::get_func_name(wf.winfnoid);
        if name_ptr.is_null() {
            "window".to_string()
        } else {
            std::ffi::CStr::from_ptr(name_ptr)
                .to_string_lossy()
                .into_owned()
        }
    };
    let filter_suffix = if wf.aggfilter.is_null() {
        String::new()
    } else {
        let filter_str =
            deparse_planner_expr(root, wf.aggfilter.cast()).unwrap_or_else(|| "...".to_string());
        format!(" FILTER (WHERE {filter_str})")
    };
    if wf.winstar {
        return format!("{func_name}(*){filter_suffix} OVER ()");
    }
    let args = PgList::<pg_sys::Node>::from_pg(wf.args);
    if is_combine_agg_funcoid(wf.winfnoid.to_u32()) {
        let spec = args
            .get_ptr(0)
            .and_then(|arg| nodecast!(Aggref, T_Aggref, strip_relabel(arg)))
            .and_then(|aggref| {
                let agg_args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
                let first_te = agg_args.get_ptr(0)?;
                pdb_agg_spec((*aggref).aggfnoid.to_u32(), (*first_te).expr.cast(), None)
            })
            .map(|(spec, _)| spec.to_string())
            .unwrap_or_else(|| "...".to_string());
        return format!("pdb.{func_name}(pdb.agg('{spec}')){filter_suffix} OVER ()");
    }
    if is_agg_funcoid(wf.winfnoid.to_u32()) {
        let spec = args
            .get_ptr(0)
            .and_then(|spec| pdb_agg_spec(wf.winfnoid.to_u32(), spec, None))
            .map(|(spec, _)| spec.to_string())
            .unwrap_or_else(|| "...".to_string());
        return format!("pdb.{func_name}('{spec}'){filter_suffix} OVER ()");
    }
    let arg_name = args
        .get_ptr(0)
        .and_then(|arg| {
            if let Some(var) = unwrap_to_var(arg)
                && let Some(col_name) = var_column_name(&*var, root)
            {
                return Some(col_name);
            }
            deparse_planner_expr(root, arg)
        })
        .unwrap_or_else(|| "...".to_string());
    format!("{func_name}({arg_name}){filter_suffix} OVER ()")
}

/// The unqualified column name a parse-tree Var refers to, resolved through the
/// query's range table; returns `None` when unresolvable or when another relation
/// in the range table has a column with the same name (so the caller falls back
/// to relation-qualified deparsing).
fn var_column_name(var: &pg_sys::Var, root: *mut pg_sys::PlannerInfo) -> Option<String> {
    if root.is_null() || unsafe { (*root).parse.is_null() } || var.varno < 1 {
        return None;
    }
    let rtable = unsafe { PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable) };
    let var_rte_idx = var.varno as usize - 1;
    let rte = rtable.get_ptr(var_rte_idx)?;
    let rte = unsafe { &*rte };
    if rte.rtekind != pg_sys::RTEKind::RTE_RELATION {
        return None;
    }
    let name_ptr = unsafe { pg_sys::get_attname(rte.relid, var.varattno, true) };
    if name_ptr.is_null() {
        return None;
    }
    for (idx, other_ptr) in rtable.iter_ptr().enumerate() {
        if idx == var_rte_idx {
            continue;
        }
        let other = unsafe { &*other_ptr };
        if other.rtekind == pg_sys::RTEKind::RTE_RELATION
            && other.relid != pg_sys::InvalidOid
            && unsafe { pg_sys::get_attnum(other.relid, name_ptr) }
                != pg_sys::InvalidAttrNumber as pg_sys::AttrNumber
        {
            return None;
        }
    }
    Some(
        unsafe { std::ffi::CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[pgrx::pg_guard]
unsafe extern "C-unwind" fn sentinel_mutator(
    node: *mut pg_sys::Node,
    context: *mut core::ffi::c_void,
) -> *mut pg_sys::Node {
    if node.is_null() {
        return std::ptr::null_mut();
    }

    if let Some(wf) = nodecast!(WindowFunc, T_WindowFunc, node) {
        let ctx = context.cast::<SentinelRewriteCtx>();
        let ordinal = (*ctx).next_ordinal;
        (*ctx).next_ordinal += 1;
        let id = WindowAggId::nested((*ctx).resno, ordinal);
        let index = (*ctx).window_aggs.find_index(id).unwrap_or_else(|| {
            panic!(
                "BUG: window function (resno={}, ordinal={ordinal}) was not \
                 extracted during path validation",
                (*ctx).resno
            )
        });
        return pg_sys::makeVar(
            WINDOW_SENTINEL_VARNO as _,
            index.to_sentinel_attno(),
            (*wf).wintype,
            -1,
            (*wf).wincollid,
            0,
        )
        .cast();
    }

    #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
    {
        let fnptr = sentinel_mutator as *const ();
        let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
            std::mem::transmute(fnptr);
        pg_sys::expression_tree_mutator(node, Some(mutator), context)
    }

    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    {
        pg_sys::expression_tree_mutator_impl(node, Some(sentinel_mutator), context)
    }
}
