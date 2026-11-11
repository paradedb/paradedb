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
#![allow(
    clippy::missing_safety_doc,
    clippy::tabs_in_doc_comments,
    unsafe_op_in_unsafe_fn
)]

pub use index_read::postgres::customscan::*;

use crate::api::HashMap;
use crate::api::agg_funcoids;
use crate::nodecast;
use crate::postgres::node::NodeExt;
use parking_lot::Mutex;
use pgrx::{PgList, PgMemoryContexts, pg_sys};
use std::ffi::CStr;
use std::ptr::NonNull;

pub mod aggregatescan;
pub mod basescan;
pub mod bitmap_intersection;
pub mod builders;
pub mod collation_semantics;
pub mod dsm;
pub mod exec;
pub mod explainer;
pub mod joinscan;
pub mod node;
pub mod opexpr;
pub mod orderby;
pub mod parallel;
pub mod path;
pub mod projections;
pub mod pullup;
pub mod pushdown;
pub mod qual_inspect;
pub mod range_table;
pub mod scan;

use crate::api::window_aggregate::window_agg_oid;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
pub use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::path::{plan_custom_path, reparameterize_custom_path_by_child};
use crate::postgres::customscan::scan::create_custom_scan_state;

pub trait CustomScanState: Default {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState);
}

struct CustomPathMethodsWrapper(*const pg_sys::CustomPathMethods);

unsafe impl Send for CustomPathMethodsWrapper {}
unsafe impl Sync for CustomPathMethodsWrapper {}

struct CustomScanMethodsWrapper(*const pg_sys::CustomScanMethods);

unsafe impl Send for CustomScanMethodsWrapper {}
unsafe impl Sync for CustomScanMethodsWrapper {}

struct CustomExecMethodsWrapper(*const pg_sys::CustomExecMethods);

unsafe impl Send for CustomExecMethodsWrapper {}
unsafe impl Sync for CustomExecMethodsWrapper {}

lazy_static::lazy_static! {
    static ref PATH_METHODS: Mutex<HashMap<&'static CStr, CustomPathMethodsWrapper>> = Mutex::default();
    static ref SCAN_METHODS: Mutex<HashMap<&'static CStr, CustomScanMethodsWrapper>> = Mutex::default();
    static ref EXEC_METHODS: Mutex<HashMap<&'static CStr, CustomExecMethodsWrapper>> = Mutex::default();
}

pub trait CustomScan: Default + Sized {
    const NAME: &'static CStr;
    type Args;
    type State: CustomScanState;
    type PrivateData: From<*mut pg_sys::List> + Into<*mut pg_sys::List>;

    fn exec_methods() -> pg_sys::CustomExecMethods;

    fn custom_exec_methods() -> *const pg_sys::CustomExecMethods {
        EXEC_METHODS
            .lock()
            .entry(Self::NAME)
            .or_insert_with(|| {
                CustomExecMethodsWrapper(
                    PgMemoryContexts::TopMemoryContext
                        .leak_and_drop_on_delete(Self::exec_methods()),
                )
            })
            .0
    }

    fn custom_path_methods() -> *const pg_sys::CustomPathMethods {
        PATH_METHODS
            .lock()
            .entry(Self::NAME)
            .or_insert_with(|| {
                CustomPathMethodsWrapper(
                    PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(
                        pg_sys::CustomPathMethods {
                            CustomName: Self::NAME.as_ptr(),
                            PlanCustomPath: Some(plan_custom_path::<Self>),
                            ReparameterizeCustomPathByChild: Some(
                                reparameterize_custom_path_by_child::<Self>,
                            ),
                        },
                    ),
                )
            })
            .0
    }

    fn custom_scan_methods() -> *const pg_sys::CustomScanMethods {
        SCAN_METHODS
            .lock()
            .entry(Self::NAME)
            .or_insert_with(|| {
                CustomScanMethodsWrapper(
                    PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(
                        pg_sys::CustomScanMethods {
                            CustomName: Self::NAME.as_ptr(),
                            CreateCustomScanState: Some(create_custom_scan_state::<Self>),
                        },
                    ),
                )
            })
            .0
    }

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Vec<pg_sys::CustomPath>;

    fn plan_custom_path(builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan;

    fn create_custom_scan_state(
        builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self>;

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    );

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    );

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot;

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn add_planner_warning<
        S: Into<String>,
        C: crate::postgres::planner_warnings::ToWarningContexts,
    >(
        message: S,
        contexts: C,
    ) {
        crate::postgres::planner_warnings::add_planner_warning(
            Self::NAME
                .to_str()
                .expect("CustomScan name should be valid UTF-8"),
            message,
            contexts,
        )
    }

    fn add_detailed_planner_warning<
        S: Into<String>,
        C: crate::postgres::planner_warnings::ToWarningContexts,
        D: IntoIterator<Item = String>,
    >(
        message: S,
        contexts: C,
        details: D,
    ) {
        crate::postgres::planner_warnings::add_detailed_planner_warning(
            Self::NAME
                .to_str()
                .expect("CustomScan name should be valid UTF-8"),
            message,
            contexts,
            details,
        )
    }

    fn mark_contexts_successful<C: crate::postgres::planner_warnings::ToWarningContexts>(
        contexts: C,
    ) {
        crate::postgres::planner_warnings::mark_contexts_successful(contexts)
    }
}

#[allow(dead_code)]
pub trait MarkRestoreCapable
where
    Self: CustomScan,
{
    fn mark_pos_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn restr_pos_custom_scan(state: &mut CustomScanStateWrapper<Self>);
}

#[derive(Debug, Clone, Copy)]
pub struct RelPathlistHookArgs {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub rti: pg_sys::Index,
    pub rte: *mut pg_sys::RangeTblEntry,
}

impl RelPathlistHookArgs {
    pub fn rel(&self) -> &pg_sys::RelOptInfo {
        unsafe { self.rel.as_ref().expect("Args::rel should not be null") }
    }

    pub fn rte(&self) -> &pg_sys::RangeTblEntry {
        unsafe { self.rte.as_ref().expect("Args::rte should not be null") }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CreateUpperPathsHookArgs {
    pub root: *mut pg_sys::PlannerInfo,
    pub stage: pg_sys::UpperRelationKind::Type,
    pub input_rel: *mut pg_sys::RelOptInfo,
    pub output_rel: *mut pg_sys::RelOptInfo,
    #[allow(dead_code)]
    pub extra: *mut ::std::os::raw::c_void,
}

impl CreateUpperPathsHookArgs {
    pub fn root(&self) -> &pg_sys::PlannerInfo {
        unsafe { self.root.as_ref().expect("Args::root should not be null") }
    }

    pub fn input_rel(&self) -> &pg_sys::RelOptInfo {
        unsafe {
            self.input_rel
                .as_ref()
                .expect("Args::input_rel should not be null")
        }
    }

    pub fn output_rel(&self) -> &pg_sys::RelOptInfo {
        unsafe {
            self.output_rel
                .as_ref()
                .expect("Args::output_rel should not be null")
        }
    }

    pub unsafe fn scan_join_rel(&self) -> Option<&pg_sys::RelOptInfo> {
        if self.stage == pg_sys::UpperRelationKind::UPPERREL_FINAL {
            joinscan::planning::find_final_rel(self.root).as_ref()
        } else {
            self.input_rel.as_ref()
        }
    }

    pub fn group_by_pathkeys(&self) -> Vec<*mut pg_sys::PathKey> {
        let pathkeys = unsafe { PgList::<pg_sys::PathKey>::from_pg(self.root().group_pathkeys) };
        #[cfg(feature = "pg15")]
        let count = pathkeys.len();
        #[cfg(not(feature = "pg15"))]
        let count = self.root().num_groupby_pathkeys as usize;
        pathkeys.iter_ptr().take(count).collect()
    }

    pub unsafe fn estimate_group_count(&self) -> f64 {
        let parse = self.root().parse;
        if parse.is_null() || (*parse).groupClause.is_null() {
            return 1.0;
        }

        let group_exprs =
            pg_sys::get_sortgrouplist_exprs((*parse).groupClause, (*parse).targetList);
        if group_exprs.is_null() {
            return 1.0;
        }

        pg_sys::estimate_num_groups(
            self.root,
            group_exprs,
            self.input_rel().rows,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }

    pub fn grouping_limit_offset(&self) -> Option<LimitOffset> {
        let parse = self.root().parse;
        if parse.is_null() {
            return None;
        }
        let has_stage_above = self.stage == pg_sys::UpperRelationKind::UPPERREL_GROUP_AGG
            && unsafe { !(*parse).distinctClause.is_null() || (*parse).hasWindowFuncs };
        if has_stage_above {
            return None;
        }
        LimitOffset::from_parse(parse)
    }

    pub fn limit_plus_offset(&self) -> Option<usize> {
        self.grouping_limit_offset()
            .and_then(|lo| lo.static_fetch())
    }

    pub unsafe fn is_single_grouping_column(&self) -> bool {
        let parse = self.root().parse;
        !parse.is_null()
            && !(*parse).groupClause.is_null()
            && PgList::<pg_sys::SortGroupClause>::from_pg((*parse).groupClause).len() == 1
    }

    pub unsafe fn orders_by_grouping_key(&self) -> bool {
        let parse = self.root().parse;
        if parse.is_null() || (*parse).sortClause.is_null() || (*parse).groupClause.is_null() {
            return false;
        }
        let sort_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause);
        let group_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).groupClause);
        if sort_clauses.len() != 1 || group_clauses.len() != 1 {
            return false;
        }
        match (sort_clauses.get_ptr(0), group_clauses.get_ptr(0)) {
            (Some(sort), Some(group)) => (*sort).tleSortGroupRef == (*group).tleSortGroupRef,
            _ => false,
        }
    }

    pub unsafe fn has_numeric_aggregate(&self) -> bool {
        let parse = self.root().parse;
        if parse.is_null() || (*parse).targetList.is_null() {
            return false;
        }

        unsafe fn is_direct_numeric_var(expr: *mut pg_sys::Node) -> bool {
            aggregatescan::join_targetlist::unwrap_to_var(expr)
                .is_some_and(|var| (*var).vartype == pg_sys::NUMERICOID)
        }

        if (*parse).targetList.any(|node| {
            crate::nodecast!(Aggref, node).is_some_and(|aggref| {
                PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args)
                    .iter_ptr()
                    .any(|arg| is_direct_numeric_var((*arg).expr.cast()))
            })
        }) {
            return true;
        }

        if !(*parse).groupClause.is_null() {
            let group_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).groupClause);
            for gc in group_clauses.iter_ptr() {
                let expr = pg_sys::get_sortgroupclause_expr(gc, (*parse).targetList);
                if !expr.is_null() && is_direct_numeric_var(expr) {
                    return true;
                }
            }
        }

        false
    }

    pub unsafe fn has_int8_sum_aggregate(&self) -> bool {
        let parse = self.root().parse;
        if parse.is_null() {
            return false;
        }

        let is_int8_sum = |node: *mut pg_sys::Node| {
            crate::nodecast!(Aggref, node).is_some_and(|aggref| {
                (*aggref).aggfnoid.to_u32() == pg_sys::F_SUM_INT8
                    && PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args)
                        .iter_ptr()
                        .any(|arg| {
                            aggregatescan::join_targetlist::unwrap_to_var((*arg).expr.cast())
                                .is_some()
                        })
            })
        };

        (!(*parse).targetList.is_null() && (*parse).targetList.any(is_int8_sum))
            || (!(*parse).havingQual.is_null() && (*parse).havingQual.any(is_int8_sum))
    }

    pub unsafe fn has_date_group(&self) -> bool {
        let parse = self.root().parse;

        if parse.is_null() || (*parse).groupClause.is_null() || (*parse).targetList.is_null() {
            return false;
        }

        let group_clauses = PgList::<pg_sys::SortGroupClause>::from_pg((*parse).groupClause);

        for gc in group_clauses.iter_ptr() {
            let expr = pg_sys::get_sortgroupclause_expr(gc, (*parse).targetList);
            if expr.is_null() {
                continue;
            }

            let Some(func_expr) = nodecast!(FuncExpr, expr) else {
                continue;
            };

            if matches!(
                (*func_expr).funcid.to_u32(),
                pg_sys::F_DATE_TIMESTAMP | pg_sys::F_DATE_TIMESTAMPTZ
            ) {
                return true;
            }
        }
        false
    }
}

pub fn wrap_custom_scan_state<CS: CustomScan>(
    node: *mut pg_sys::CustomScanState,
) -> NonNull<CustomScanStateWrapper<CS>> {
    NonNull::<CustomScanStateWrapper<CS>>::new(node.cast())
        .expect("`CustomScanState` node should not be null")
}

pub unsafe fn query_has_paradedb_agg(parse: *mut pg_sys::Query, recursive: bool) -> bool {
    let paradedb_agg_oids = agg_funcoids();
    let window_agg_proc_oid = window_agg_oid();

    let contains_agg = |node: *mut pg_sys::Node| {
        node.any(|node| {
            nodecast!(WindowFunc, node)
                .is_some_and(|expr| paradedb_agg_oids.contains(&(*expr).winfnoid.to_u32()))
                || nodecast!(Aggref, node)
                    .is_some_and(|expr| paradedb_agg_oids.contains(&(*expr).aggfnoid.to_u32()))
                || (window_agg_proc_oid != pg_sys::InvalidOid
                    && nodecast!(FuncExpr, node)
                        .is_some_and(|expr| (*expr).funcid == window_agg_proc_oid))
        })
    };
    let target_list = PgList::<pg_sys::TargetEntry>::from_pg((*parse).targetList);
    if target_list
        .iter_ptr()
        .any(|te| contains_agg((*te).expr.cast()))
        || contains_agg((*parse).havingQual)
    {
        return true;
    }

    if recursive {
        if !(*parse).rtable.is_null() {
            let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*parse).rtable);
            for rte in rtable.iter_ptr() {
                if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY
                    && !(*rte).subquery.is_null()
                    && query_has_paradedb_agg((*rte).subquery, recursive)
                {
                    return true;
                }
            }
        }

        if !(*parse).cteList.is_null() {
            let ctelist = PgList::<pg_sys::CommonTableExpr>::from_pg((*parse).cteList);
            for cte in ctelist.iter_ptr() {
                if !(*cte).ctequery.is_null()
                    && query_has_paradedb_agg((*cte).ctequery.cast(), recursive)
                {
                    return true;
                }
            }
        }
    }

    false
}
