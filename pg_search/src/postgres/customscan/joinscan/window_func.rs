// Copyright (c) 2023-2026 ParadeDB, Inc.
// TODO: Add window outputs here
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

use crate::postgres::customscan::joinscan::scan_state::{
    null_if_source_exists, resolve_var_to_df_col,
};
use crate::schema::SearchFieldType;
use datafusion::common::internal_datafusion_err;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::expr::WindowFunction;
use datafusion::logical_expr::{Expr, Literal, WindowFunctionDefinition};
use pgrx::pg_sys::{FRAMEOPTION_NONDEFAULT, Query, WindowFunc};
use pgrx::{PgList, pg_sys};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::nodecast;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, classify_aggregate_oid, unwrap_to_var,
};
use crate::postgres::customscan::joinscan::planning::resolve_fast_field_from_join_sources;

use super::build::{JoinCSClause, JoinSource};

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
    pub agg_type: SupportedWindowAggType,
    pub col_info: Option<ColumnInfo>,
    pub result_type: ResultType,
    pub id: WindowAggId,
}
impl WindowAgg {
    pub fn arg_field_type(&self) -> Option<&SearchFieldType> {
        self.col_info.as_ref().and_then(|ci| ci.field_type.as_ref())
    }

    /// True when `other` computes the same aggregate over the same input
    /// column — identity (`id`) excluded. Comparing `(rti, attno)` suffices
    /// for the column: `field_type` is derived from them at extraction.
    pub fn same_spec(&self, other: &Self) -> bool {
        self.agg_type == other.agg_type
            && self.result_type.0 == other.result_type.0
            && match (&self.col_info, &other.col_info) {
                (None, None) => true,
                (Some(a), Some(b)) => a.rti == b.rti && a.attno == b.attno,
                _ => false,
            }
    }

    pub fn as_window_expr(&self, join_clause: &JoinCSClause) -> Result<Expr> {
        use crate::customscan::datafusion::numeric_agg;
        use datafusion::functions_aggregate::{average, count, min_max, sum};

        let col_expr = match &self.col_info {
            Some(ci) => match resolve_var_to_df_col(join_clause, ci.rti, ci.attno) {
                Some(ce) => Some(ce),
                None => {
                    // The argument's relation participates in the join but was
                    // pruned from the output (e.g. the inner side of an Anti
                    // Join): its values are identically NULL in every output
                    // row, so the aggregate is a plan-time constant — 0 for
                    // COUNT, NULL otherwise. This is the window-input analogue
                    // of the `null_if_source_exists` fallback every other
                    // pruned-column consumer applies.
                    if null_if_source_exists(join_clause, ci.rti).is_some() {
                        return Ok(match self.agg_type {
                            SupportedWindowAggType::Count => datafusion::logical_expr::lit(0_i64),
                            _ => {
                                datafusion::logical_expr::lit(datafusion::common::ScalarValue::Null)
                            }
                        });
                    }
                    return Err(internal_datafusion_err!(
                        "Failed to map column to fast field and column expr. rti: {}, attno: {}",
                        ci.rti,
                        ci.attno
                    ));
                }
            },
            None => None,
        };
        let numeric_field = numeric_window_field(self.agg_type, self.arg_field_type())?;

        // Match only basic aggregate functions. Missing and filters are not supported in global window
        // functions
        //
        // Numeric fields require special handling for SUM/AVG. They route to scaled-Int64 or
        // decimal-bytes UDAFs. The Numeric64 UDAFs take the scale as a plan literal so it survives
        // plan serialization for parallel and MPP execution; decimal-bytes values are self-describing.
        match self.agg_type {
            SupportedWindowAggType::Sum => {
                let ce = col_expr.expect("should always have a column expression for SUM");
                match numeric_field {
                    None => Ok(Expr::from(WindowFunction::new(
                        WindowFunctionDefinition::AggregateUDF(sum::sum_udaf()),
                        vec![ce],
                    ))),
                    Some(SearchFieldType::Numeric64(_, scale)) => {
                        Ok(Expr::from(WindowFunction::new(
                            WindowFunctionDefinition::AggregateUDF(
                                numeric_agg::numeric64_sum_udaf(),
                            ),
                            vec![ce, scale.lit()],
                        )))
                    }
                    Some(_) => Ok(Expr::from(WindowFunction::new(
                        WindowFunctionDefinition::AggregateUDF(
                            numeric_agg::numeric_bytes_sum_udaf(),
                        ),
                        vec![ce],
                    ))),
                }
            }
            SupportedWindowAggType::Avg => {
                let ce = col_expr.expect("should always have a column expression for AVG");
                match numeric_field {
                    None => Ok(Expr::from(WindowFunction::new(
                        WindowFunctionDefinition::AggregateUDF(average::avg_udaf()),
                        vec![ce],
                    ))),
                    Some(SearchFieldType::Numeric64(_, scale)) => {
                        Ok(Expr::from(WindowFunction::new(
                            WindowFunctionDefinition::AggregateUDF(
                                numeric_agg::numeric64_avg_udaf(),
                            ),
                            vec![ce, scale.lit()],
                        )))
                    }
                    Some(_) => Ok(Expr::from(WindowFunction::new(
                        WindowFunctionDefinition::AggregateUDF(
                            numeric_agg::numeric_bytes_avg_udaf(),
                        ),
                        vec![ce],
                    ))),
                }
            }
            SupportedWindowAggType::Min => Ok(Expr::from(WindowFunction::new(
                WindowFunctionDefinition::AggregateUDF(min_max::min_udaf()),
                vec![col_expr.expect("should always have a column expression for MIN")],
            ))),
            SupportedWindowAggType::Max => Ok(Expr::from(WindowFunction::new(
                WindowFunctionDefinition::AggregateUDF(min_max::max_udaf()),
                vec![col_expr.expect("should always have a column expression for MAX")],
            ))),
            SupportedWindowAggType::Count => Ok(Expr::from(WindowFunction::new(
                WindowFunctionDefinition::AggregateUDF(count::count_udaf()),
                vec![col_expr.expect("should always have a column expression for COUNT")],
            ))),
            SupportedWindowAggType::CountStar => Ok(count::count_all_window()),
        }
    }

    pub fn as_aggregate_expr(&self, join_clause: &JoinCSClause) -> Result<Expr> {
        use crate::customscan::datafusion::numeric_agg;
        use datafusion::functions_aggregate::{average, count, min_max, sum};

        let col_expr = match &self.col_info {
            Some(ci) => match resolve_var_to_df_col(join_clause, ci.rti, ci.attno) {
                Some(ce) => Some(ce),
                None => {
                    // The argument's relation participates in the join but was
                    // pruned from the output (e.g. the inner side of an Anti
                    // Join): its values are identically NULL in every output
                    // row, so the aggregate is a plan-time constant — 0 for
                    // COUNT, NULL otherwise. This is the window-input analogue
                    // of the `null_if_source_exists` fallback every other
                    // pruned-column consumer applies.
                    if null_if_source_exists(join_clause, ci.rti).is_some() {
                        return Ok(match self.agg_type {
                            SupportedWindowAggType::Count => datafusion::logical_expr::lit(0_i64),
                            _ => {
                                datafusion::logical_expr::lit(datafusion::common::ScalarValue::Null)
                            }
                        });
                    }
                    return Err(internal_datafusion_err!(
                        "Failed to map column to fast field and column expr. rti: {}, attno: {}",
                        ci.rti,
                        ci.attno
                    ));
                }
            },
            None => None,
        };
        let numeric_field = numeric_window_field(self.agg_type, self.arg_field_type())?;

        // Match only basic aggregate functions. Missing and filters are not supported in global window
        // functions
        //
        // Numeric fields require special handling for SUM/AVG. They route to scaled-Int64 or
        // decimal-bytes UDAFs. The Numeric64 UDAFs take the scale as a plan literal so it survives
        // plan serialization for parallel and MPP execution; decimal-bytes values are self-describing.
        match self.agg_type {
            SupportedWindowAggType::Sum => {
                let ce = col_expr.expect("should always have a column expression for SUM");
                match numeric_field {
                    None => Ok(sum::sum_udaf().call(vec![ce])),
                    Some(SearchFieldType::Numeric64(_, scale)) => {
                        Ok(numeric_agg::numeric64_sum_udaf().call(vec![ce, scale.lit()]))
                    }
                    Some(_) => Ok(numeric_agg::numeric_bytes_sum_udaf().call(vec![ce])),
                }
            }
            SupportedWindowAggType::Avg => {
                let ce = col_expr.expect("should always have a column expression for AVG");
                match numeric_field {
                    None => Ok(average::avg_udaf().call(vec![ce])),
                    Some(SearchFieldType::Numeric64(_, scale)) => {
                        Ok(numeric_agg::numeric64_avg_udaf().call(vec![ce, scale.lit()]))
                    }
                    Some(_) => Ok(numeric_agg::numeric_bytes_avg_udaf().call(vec![ce])),
                }
            }
            SupportedWindowAggType::Min => Ok(min_max::min_udaf().call(vec![
                col_expr.expect("should always have a column expression for MIN"),
            ])),
            SupportedWindowAggType::Max => Ok(min_max::max_udaf().call(vec![
                col_expr.expect("should always have a column expression for MAX"),
            ])),
            SupportedWindowAggType::Count => Ok(count::count_udaf().call(vec![
                col_expr.expect("should always have a column expression for COUNT"),
            ])),
            SupportedWindowAggType::CountStar => Ok(count::count_all()),
        }
    }
}

/// Sentinel `varno` for window-aggregate inputs in serialized projection
/// expressions (see [`rewrite_window_funcs_to_sentinels`]). Real range-table
/// indexes are 1-based, so 0 cannot collide with a source relation.
pub const WINDOW_SENTINEL_VARNO: pg_sys::Index = 0;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    #[allow(dead_code)]
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
        let index = col_name
            .strip_prefix(Self::PREFIX)
            .ok_or(())?
            .parse::<usize>()
            .map_err(|_| ())?;
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

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &WindowAgg> {
        self.0.iter()
    }

    pub fn iter_indexed(&self) -> impl Iterator<Item = (WindowAggIndex, &WindowAgg)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, wa)| (WindowAggIndex(i), wa))
    }

    /// The index of the first entry computing the same aggregate as
    /// `index`'s entry. Identical window aggregates (e.g. `COUNT(*) OVER ()`
    /// in several target entries) share one window column: only canonical
    /// entries are materialized by the window step, and every reference
    /// resolves to the canonical column name. Duplicate window expressions
    /// in one Window node would otherwise be extracted into a projection by
    /// DataFusion's common-subexpression elimination, where a window
    /// expression cannot be physically planned.
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

pub fn extract_window_agg(
    wf: *const WindowFunc,
    sources: &[&JoinSource],
    parse: &Query,
    id: WindowAggId,
) -> Result<WindowAgg, String> {
    assert!(!wf.is_null());
    let wf = unsafe { &*wf };

    if !wf.aggfilter.is_null() {
        return Err("window function filter clause is not supported".to_string());
    }

    if !wf.winagg {
        return Err(
            "only simple (sum/min/max/avg/count) window functions are supported".to_string(),
        );
    }

    let clause = unsafe {
        PgList::<pg_sys::WindowClause>::from_pg(parse.windowClause)
            .iter_ptr()
            .find(|wc| (**wc).winref == wf.winref)
            .expect("WindowFunc.winref should always match a clause")
    };
    assert!(!clause.is_null());
    let clause = unsafe { *clause };

    if !clause.partitionClause.is_null()
        || !clause.orderClause.is_null()
        || clause.frameOptions & FRAMEOPTION_NONDEFAULT as i32 != 0
    {
        return Err(
            "only bare window functions of the style 'agg OVER ()' are supported".to_string(),
        );
    }

    let Some(agg_type) = SupportedWindowAggType::from_funcoid(wf.winfnoid, wf.winstar) else {
        return Err("unsupported window function was provided".to_string());
    };

    let col_info = {
        let args = unsafe { PgList::<pg_sys::Node>::from_pg(wf.args) };
        match args.len() {
            0 => {
                assert!(wf.winstar); // count(*)
                None
            }
            1 => {
                let arg = args.get_ptr(0).unwrap();

                let var = unwrap_to_var(arg).ok_or_else(|| {
                    "window aggregate argument must be a direct column reference".to_string()
                })?;

                assert!(!var.is_null());
                let var = unsafe { *var };

                let Some(ff) = resolve_fast_field_from_join_sources(sources, &var) else {
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

                Some(ColumnInfo::new(
                    var.varno as pg_sys::Index,
                    var.varattno,
                    ff.field_type().cloned(),
                ))
            }
            _ => {
                return Err("multi-argument window aggregates are not supported".to_string());
            }
        }
    };

    Ok(WindowAgg {
        agg_type,
        col_info,
        result_type: ResultType(wf.wintype),
        id,
    })
}

pub fn is_supported_window_agg_node(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }
    if let Some(wf) = unsafe { nodecast!(WindowFunc, T_WindowFunc, node) } {
        let wf = unsafe { &*wf };
        return SupportedWindowAggType::from_funcoid(wf.winfnoid, wf.winstar).is_some();
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

/// Copy `expr`, replacing each embedded `WindowFunc` with a
/// `paradedb.window_agg('<description>')` placeholder `FuncExpr` whose
/// node-level `funcresulttype` is the aggregate's wintype.
///
/// PG18's EXPLAIN requires a `WindowAgg` plan node to deparse a `WindowFunc`
/// (commit 8b1b342544b6 removed the `OVER (?)` fallback), but the scan
/// absorbs the window so no such node exists — a raw `WindowFunc` left in
/// the plan's target lists makes every `EXPLAIN VERBOSE` fail with "could
/// not find window clause". The placeholder deparses as an ordinary function
/// call. It is never executed: `plan_custom_path` applies this rewrite
/// identically to `scan.plan.targetlist` and `custom_scan_tlist`, so
/// setrefs' whole-entry equality match still rewrites the outer occurrence
/// to an INDEX_VAR into the scan output. The text argument is cosmetic
/// EXPLAIN output only — it is never parsed.
pub unsafe fn rewrite_window_funcs_to_placeholders(
    expr: *mut pg_sys::Node,
    root: *mut pg_sys::PlannerInfo,
) -> *mut pg_sys::Node {
    let mut ctx = PlaceholderRewriteCtx { root };
    placeholder_mutator(expr, std::ptr::addr_of_mut!(ctx).cast())
}

struct PlaceholderRewriteCtx {
    root: *mut pg_sys::PlannerInfo,
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
        let ctx = context.cast::<PlaceholderRewriteCtx>();

        // A ruleutils deparse of the WindowFunc hits the same PG18 error
        // this rewrite exists to avoid (deparse_planner_expr catches it and
        // returns None on PG18; on older versions it renders `OVER (?)`),
        // so fall back to a description built from the node itself.
        let description = crate::postgres::deparse::deparse_planner_expr((*ctx).root, node)
            .unwrap_or_else(|| describe_window_func(&*wf, (*ctx).root));

        match crate::api::window_aggregate::make_window_agg_placeholder(
            &description,
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
    if wf.winstar {
        return format!("{func_name}(*) OVER ()");
    }
    let args = PgList::<pg_sys::Node>::from_pg(wf.args);
    let arg_name = args
        .get_ptr(0)
        .and_then(|arg| {
            let var = unwrap_to_var(arg)?;
            var_column_name(&*var, root)
        })
        .unwrap_or_else(|| "...".to_string());
    format!("{func_name}({arg_name}) OVER ()")
}

/// The column name a parse-tree Var refers to, resolved through the query's
/// range table; `None` when anything along the way is unresolvable.
fn var_column_name(var: &pg_sys::Var, root: *mut pg_sys::PlannerInfo) -> Option<String> {
    if root.is_null() || unsafe { (*root).parse.is_null() } || var.varno < 1 {
        return None;
    }
    let rtable = unsafe { PgList::<pg_sys::RangeTblEntry>::from_pg((*(*root).parse).rtable) };
    let rte = rtable.get_ptr(var.varno as usize - 1)?;
    let rte = unsafe { &*rte };
    if rte.rtekind != pg_sys::RTEKind::RTE_RELATION {
        return None;
    }
    let name_ptr = unsafe { pg_sys::get_attname(rte.relid, var.varattno, true) };
    if name_ptr.is_null() {
        return None;
    }
    Some(
        unsafe { std::ffi::CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// Determines if the field for this aggregate is a numeric and is supported for pushing
/// down this aggregate.
///
/// Returns:
/// - `Ok(None)` if the aggregate has no field requirements or this field is not numeric.
/// - `Ok(Some(_))` if the field is a supported numeric field
/// - `Err(_)` if the field is an unsupported numeric
pub fn numeric_window_field(
    agg_type: SupportedWindowAggType,
    field_type: Option<&SearchFieldType>,
) -> Result<Option<&SearchFieldType>> {
    match (agg_type, field_type) {
        (SupportedWindowAggType::Count | SupportedWindowAggType::CountStar, _) => Ok(None),
        (_, Some(ft)) => {
            let field_type = if ft.is_numeric() {
                ft
            } else {
                return Ok(None);
            };

            if field_type.numeric_scale().is_none() {
                return Err(DataFusionError::Plan(
                    "Non-count window aggregation on an unbounded NUMERIC column is not supported; declare a \
                     precision and scale to enable aggregate pushdown".to_string(),
                ));
            }

            Ok(Some(ft))
        }
        _ => Ok(None),
    }
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
