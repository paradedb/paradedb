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

//! https://www.postgresql.org/docs/current/custom-scan.html

#![allow(clippy::tabs_in_doc_comments)]

use parking_lot::Mutex;
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgMemoryContexts};

use std::ffi::{CStr, CString};
use std::ptr::NonNull;

pub mod aggregatescan;
pub mod basescan;
mod builders;
pub mod dsm;
pub mod exec;
pub mod explain;
mod explainer;
mod hook;
pub mod joinscan;
pub mod opexpr;
pub mod orderby;
pub mod parallel;
mod path;
pub mod projections;
pub mod pullup;
mod pushdown;
pub mod qual_inspect;
mod range_table;
mod scan;
pub mod solve_expr;

use crate::api::HashMap;

use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::path::{plan_custom_path, reparameterize_custom_path_by_child};
use crate::postgres::customscan::scan::create_custom_scan_state;
pub use hook::{
    register_join_pathlist, register_rel_pathlist, register_upper_path,
    register_window_aggregate_hook,
};

// TODO: This trait should be expanded to include a `reset` method, which would become the
// default/only implementation of `rescan_custom_scan`.
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
    // We need to allocate the structs to define functions once, however
    // all the methods are generic over this trait ([`CustomScan]).  Because Rust
    // monomorphizes these functions, they're actually at different addresses per CustomScan
    // impl. As such, we allocate them once, in Postgres "TopMemoryContext", which is **never**
    // freed. This ensures we don't waste any more memory than we need and more importantly,
    // ensures the returned pointer holding the function pointers lives for the life of the
    // process, which Postgres requires of these.
    static ref PATH_METHODS: Mutex<HashMap<&'static CStr, CustomPathMethodsWrapper>> = Mutex::default();
    static ref SCAN_METHODS: Mutex<HashMap<&'static CStr, CustomScanMethodsWrapper>> = Mutex::default();
    static ref EXEC_METHODS: Mutex<HashMap<&'static CStr, CustomExecMethodsWrapper>> = Mutex::default();
}

pub trait CustomScan: Default + Sized {
    const NAME: &'static CStr;
    type Args;
    type State: CustomScanState;
    type PrivateData: From<*mut pg_sys::List> + Into<*mut pg_sys::List>;

    /// Returns the execution methods for this custom scan.
    ///
    /// This method is called exactly once per custom scan type and the result is memoized
    /// in `TopMemoryContext`. Implementations should return a [`pg_sys::CustomExecMethods`]
    /// struct populated with the appropriate callback functions.
    ///
    /// Common callback wrappers are available in the [`exec`] and [`dsm`] modules.
    /// Scanners that implement capability traits (like [`ParallelQueryCapable` or [`MarkRestoreCapable`])
    /// can use the generic versions of these callbacks by passing themselves as the generic
    /// argument (e.g. `Some(exec::begin_custom_scan::<Self>)`).
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

    /// Add a planner warning associated with this CustomScan type.
    ///
    /// The warning will be deduplicated and emitted at the end of the planning phase.
    /// The category is automatically set to `Self::NAME`.
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

    /// Add a detailed planner warning associated with this CustomScan type.
    ///
    /// The warning will be deduplicated and emitted at the end of the planning phase.
    /// The category is automatically set to `Self::NAME`.
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

    /// Clear planner warnings for the specified contexts (e.g., table aliases).
    ///
    /// This should be called when a CustomScan is successfully planned for a set of tables,
    /// to suppress any "failure" warnings that might have been generated during the
    /// exploration of alternative (rejected) paths for these tables.
    /// The category is automatically set to `Self::NAME`.
    fn mark_contexts_successful<C: crate::postgres::planner_warnings::ToWarningContexts>(
        contexts: C,
    ) {
        crate::postgres::planner_warnings::mark_contexts_successful(
            Self::NAME
                .to_str()
                .expect("CustomScan name should be valid UTF-8"),
            contexts,
        )
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
    #[allow(dead_code)]
    pub fn root(&self) -> &pg_sys::PlannerInfo {
        unsafe { self.root.as_ref().expect("Args::root should not be null") }
    }

    pub fn rel(&self) -> &pg_sys::RelOptInfo {
        unsafe { self.rel.as_ref().expect("Args::rel should not be null") }
    }

    pub fn rte(&self) -> &pg_sys::RangeTblEntry {
        unsafe { self.rte.as_ref().expect("Args::rte should not be null") }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JoinPathlistHookArgs {
    pub root: *mut pg_sys::PlannerInfo,
    #[allow(dead_code)]
    pub joinrel: *mut pg_sys::RelOptInfo,
    #[allow(dead_code)]
    pub outerrel: *mut pg_sys::RelOptInfo,
    #[allow(dead_code)]
    pub innerrel: *mut pg_sys::RelOptInfo,
    #[allow(dead_code)]
    pub jointype: pg_sys::JoinType::Type,
    #[allow(dead_code)]
    pub extra: *mut pg_sys::JoinPathExtraData,
}

impl JoinPathlistHookArgs {
    #[allow(dead_code)]
    pub fn root(&self) -> &pg_sys::PlannerInfo {
        unsafe { self.root.as_ref().expect("Args::root should not be null") }
    }

    #[allow(dead_code)]
    pub fn joinrel(&self) -> &pg_sys::RelOptInfo {
        unsafe {
            self.joinrel
                .as_ref()
                .expect("Args::joinrel should not be null")
        }
    }

    #[allow(dead_code)]
    pub fn outerrel(&self) -> &pg_sys::RelOptInfo {
        unsafe {
            self.outerrel
                .as_ref()
                .expect("Args::outerrel should not be null")
        }
    }

    #[allow(dead_code)]
    pub fn innerrel(&self) -> &pg_sys::RelOptInfo {
        unsafe {
            self.innerrel
                .as_ref()
                .expect("Args::innerrel should not be null")
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CreateUpperPathsHookArgs {
    pub root: *mut pg_sys::PlannerInfo,
    #[allow(dead_code)]
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
}

/// Helper function for wrapping a raw [`pg_sys::CustomScanState`] pointer with something more
/// usable by implementers
fn wrap_custom_scan_state<CS: CustomScan>(
    node: *mut pg_sys::CustomScanState,
) -> NonNull<CustomScanStateWrapper<CS>> {
    NonNull::<CustomScanStateWrapper<CS>>::new(node.cast())
        .expect("`CustomScanState` node should not be null")
}

pub unsafe fn operator_oid(signature: &str) -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(
        pg_sys::regoperatorin,
        &[CString::new(signature).into_datum()],
    )
    .expect("should be able to lookup operator signature")
}

pub fn score_funcoids() -> [pg_sys::Oid; 2] {
    [
        unsafe {
            direct_function_call::<pg_sys::Oid>(
                pg_sys::regprocedurein,
                &[c"pdb.score(anyelement)".into_datum()],
            )
            .expect("the `pdb.score(anyelement)` function should exist")
        },
        unsafe {
            direct_function_call::<pg_sys::Oid>(
                pg_sys::regprocedurein,
                &[c"paradedb.score(anyelement)".into_datum()],
            )
            .expect("the `paradedb.score(anyelement)` function should exist")
        },
    ]
}
