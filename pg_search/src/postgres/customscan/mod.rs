// Copyright (c) 2023-2025 ParadeDB, Inc.
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

#![allow(unused_variables)]
#![allow(clippy::tabs_in_doc_comments)]

use parking_lot::Mutex;
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgMemoryContexts};

use std::ffi::{CStr, CString};
use std::ptr::NonNull;

mod builders;
mod dsm;
mod exec;
mod explainer;
mod hook;
mod opexpr;
mod path;
pub mod pdbscan;
mod pushdown;
mod qual_inspect;
mod scan;

use crate::api::HashMap;
use crate::postgres::customscan::exec::{
    begin_custom_scan, end_custom_scan, exec_custom_scan, explain_custom_scan,
    mark_pos_custom_scan, rescan_custom_scan, restr_pos_custom_scan, shutdown_custom_scan,
};

use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::path::{plan_custom_path, reparameterize_custom_path_by_child};
use crate::postgres::customscan::scan::create_custom_scan_state;
pub use hook::register_rel_pathlist;

pub trait CustomScanState: Default {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState);
}

struct CustomPathMethodsWrapper(*const pg_sys::CustomPathMethods);

unsafe impl Send for CustomPathMethodsWrapper {}
unsafe impl Sync for CustomPathMethodsWrapper {}

struct CustomScanMethodsWrapper(*const pg_sys::CustomScanMethods);

unsafe impl Send for CustomScanMethodsWrapper {}
unsafe impl Sync for CustomScanMethodsWrapper {}

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
}

pub trait CustomScan: ExecMethod + Default + Sized {
    const NAME: &'static CStr;
    type Args;
    type State: CustomScanState;
    type PrivateData: From<*mut pg_sys::List> + Into<*mut pg_sys::List>;

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

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath>;

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
}

pub trait ExecMethod {
    fn exec_methods() -> *const pg_sys::CustomExecMethods;
}

#[allow(dead_code)]
pub trait PlainExecCapable: ExecMethod
where
    Self: CustomScan,
{
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        unsafe {
            static mut METHODS: *mut pg_sys::CustomExecMethods = std::ptr::null_mut();

            if METHODS.is_null() {
                METHODS = PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(
                    pg_sys::CustomExecMethods {
                        CustomName: Self::NAME.as_ptr(),
                        BeginCustomScan: Some(begin_custom_scan::<Self>),
                        ExecCustomScan: Some(exec_custom_scan::<Self>),
                        EndCustomScan: Some(end_custom_scan::<Self>),
                        ReScanCustomScan: Some(rescan_custom_scan::<Self>),
                        MarkPosCustomScan: None,
                        RestrPosCustomScan: None,
                        EstimateDSMCustomScan: None,
                        InitializeDSMCustomScan: None,
                        ReInitializeDSMCustomScan: None,
                        InitializeWorkerCustomScan: None,
                        ShutdownCustomScan: Some(shutdown_custom_scan::<Self>),
                        ExplainCustomScan: Some(explain_custom_scan::<Self>),
                    },
                );
            }
            METHODS
        }
    }
}

#[allow(dead_code)]
pub trait MarkRestoreCapable: ExecMethod
where
    Self: CustomScan,
{
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        unsafe {
            static mut METHODS: *mut pg_sys::CustomExecMethods = std::ptr::null_mut();

            if METHODS.is_null() {
                METHODS = PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(
                    pg_sys::CustomExecMethods {
                        CustomName: Self::NAME.as_ptr(),
                        BeginCustomScan: Some(begin_custom_scan::<Self>),
                        ExecCustomScan: Some(exec_custom_scan::<Self>),
                        EndCustomScan: Some(end_custom_scan::<Self>),
                        ReScanCustomScan: Some(rescan_custom_scan::<Self>),
                        MarkPosCustomScan: Some(mark_pos_custom_scan::<Self>),
                        RestrPosCustomScan: Some(restr_pos_custom_scan::<Self>),
                        EstimateDSMCustomScan: None,
                        InitializeDSMCustomScan: None,
                        ReInitializeDSMCustomScan: None,
                        InitializeWorkerCustomScan: None,
                        ShutdownCustomScan: Some(shutdown_custom_scan::<Self>),
                        ExplainCustomScan: Some(explain_custom_scan::<Self>),
                    },
                );
            }
            METHODS
        }
    }

    fn mark_pos_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn restr_pos_custom_scan(state: &mut CustomScanStateWrapper<Self>);
}

#[derive(Debug)]
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

pub fn score_funcoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.score(anyelement)".into_datum()],
        )
        .expect("the `paradedb.score(anyelement)` function should exist")
    }
}
