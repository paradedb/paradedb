// Copyright (c) 2023-2026 ParadeDB, Inc.
// Copyright (c) 2023-2026 ParadeDB, Inc.
//
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

use crate::api::pdb_agg_spec;
use crate::postgres::customscan::aggregatescan::datafusion_build::{
    ResolutionSource, resolve_source_field,
};
use crate::postgres::customscan::aggregatescan::pdb_agg::{PdbAggFieldRef, PdbAggRequest};
use crate::schema::SearchFieldType;
use pgrx::pg_sys::{FRAMEOPTION_NONDEFAULT, Query, WindowFunc};
use pgrx::{PgList, pg_sys};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::api::aggregate::is_agg_funcoid;
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, classify_aggregate_oid, unwrap_to_var,
};
use crate::postgres::customscan::joinscan::planning::resolve_fast_field_from_join_sources;

use super::build::JoinSource;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SqlWindowAggDef(pub SqlWindowAggType, pub Option<ColumnInfo>);
impl SqlWindowAggDef {
    pub fn agg_type(&self) -> SqlWindowAggType {
        self.0
    }

    pub fn col_info(&self) -> Option<&ColumnInfo> {
        self.1.as_ref()
    }

    pub fn arg_field_type(&self) -> Option<&SearchFieldType> {
        self.1.as_ref().and_then(|ci| ci.field_type.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowAggDef {
    Sql(SqlWindowAggDef),
    /// Contains (agg spec, visibility)
    ///
    /// NOTE: PartialEq involves comparing the entire json blob right now. We should probably find a
    /// better way of doing that
    PdbAgg(Box<PdbAggRequest>),
}
impl WindowAggDef {
    pub fn new_sql(wa_type: SqlWindowAggType, ci: Option<ColumnInfo>) -> Self {
        if ci.is_none() {
            assert!(
                matches!(wa_type, SqlWindowAggType::CountStar),
                "A ColumnInfo is required for all sql window function types except CountStar"
            );
        }
        Self::Sql(SqlWindowAggDef(wa_type, ci))
    }

    pub fn try_pdb_agg_from_window_func(
        wf: &WindowFunc,
        sources: &[&JoinSource],
    ) -> Result<Self, String> {
        assert!(
            is_agg_funcoid(wf.winfnoid.into()),
            "try_pdb_agg_from_window_func should only be called on pdb.agg window functions",
        );

        let args = unsafe { PgList::<pg_sys::Node>::from_pg(wf.args) };
        let spec_arg = args.get_ptr(0).ok_or("pdb.agg() spec must exist")?;
        if spec_arg.is_null() {
            return Err("spec arg was null".to_string());
        }
        let visibility_arg = args.get_ptr(1);
        if let Some(v) = visibility_arg
            && v.is_null()
        {
            return Err("invalid visibility argument".to_string());
        }
        let Some((spec, visibility)) =
            (unsafe { pdb_agg_spec(wf.winfnoid.into(), spec_arg, visibility_arg) })
        else {
            return Err("failed to build pdb agg spec".to_string());
        };
        let resolution_sources = sources.iter().map(|s| ResolutionSource::from(*s));
        let agg_req = PdbAggRequest::lower(spec, visibility, &|field| {
            let resolved = resolve_source_field(resolution_sources.clone(), field)?;
            Ok(PdbAggFieldRef {
                rti: resolved.source_rti,
                attno: resolved.attno,
                field_name: resolved.field_name,
                field_type: resolved.field_type,
                plan_position: 0,
                is_array: resolved.is_array,
            })
        })?;

        Ok(Self::PdbAgg(Box::new(agg_req)))
    }

    /// Returns the contents of the this if this is a Sql variant
    pub fn sql(&self) -> Option<&SqlWindowAggDef> {
        match self {
            Self::Sql(sql) => Some(sql),
            _ => None,
        }
    }

    /// Returns the contents of the this if this is a PdbAgg variant
    pub fn pdb(&self) -> Option<&PdbAggRequest> {
        match self {
            Self::PdbAgg(pdb) => Some(pdb),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum SqlWindowAggType {
    Count,
    CountStar,
    Sum,
    Avg,
    Min,
    Max,
}
impl SqlWindowAggType {
    pub fn from_funcoid(oid: pg_sys::Oid, aggstar: bool) -> Option<Self> {
        match classify_aggregate_oid(oid.to_u32(), aggstar, false) {
            Some(AggKind::Count) => Some(SqlWindowAggType::Count),
            Some(AggKind::CountStar) => Some(SqlWindowAggType::CountStar),
            Some(AggKind::Sum) => Some(SqlWindowAggType::Sum),
            Some(AggKind::Avg) => Some(SqlWindowAggType::Avg),
            Some(AggKind::Min) => Some(SqlWindowAggType::Min),
            Some(AggKind::Max) => Some(SqlWindowAggType::Max),
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
impl PartialEq for ColumnInfo {
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
    pub result_type: ResultType,
    pub id: WindowAggId,
}
impl WindowAgg {
    /// True when `other` computes the same aggregate over the same input
    /// column — identity (`id`) excluded. Comparing `(rti, attno)` suffices
    /// for the column: `field_type` is derived from them at extraction.
    pub fn same_spec(&self, other: &Self) -> bool {
        self.agg_def == other.agg_def && self.result_type.0 == other.result_type.0
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

    pub fn as_int(&self) -> usize {
        self.0
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
        write!(f, "{}{}", Self::PREFIX, self.0.0)
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

    let agg_def = match SqlWindowAggType::from_funcoid(wf.winfnoid, wf.winstar) {
        Some(agg_type) => {
            let ci = extract_single_arg_column_info(wf, sources, agg_type)?;
            WindowAggDef::new_sql(agg_type, ci)
        }
        None => {
            if is_agg_funcoid(wf.winfnoid.into()) {
                WindowAggDef::try_pdb_agg_from_window_func(wf, sources)?
            } else {
                return Err("unsupported window function was provided".to_string());
            }
        }
    };

    Ok(WindowAgg {
        agg_def,
        result_type: ResultType(wf.wintype),
        id,
    })
}

fn extract_single_arg_column_info(
    wf: &pg_sys::WindowFunc,
    sources: &[&JoinSource],
    agg_type: SqlWindowAggType,
) -> Result<Option<ColumnInfo>, String> {
    let args = unsafe { PgList::<pg_sys::Node>::from_pg(wf.args) };
    match args.len() {
        0 => {
            // count(*)
            assert!(wf.winstar);
            assert!(matches!(agg_type, SqlWindowAggType::CountStar));
            Ok(None)
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
                SqlWindowAggType::Count | SqlWindowAggType::CountStar
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
        return SqlWindowAggType::from_funcoid(wf.winfnoid, wf.winstar).is_some()
            || is_agg_funcoid(wf.winfnoid.into());
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
