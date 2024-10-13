// Copyright (c) 2023-2024 Retake, Inc.
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
#![allow(dead_code)]
#![allow(clippy::tabs_in_doc_comments)]

use pgrx::{pg_sys, PgMemoryContexts};
use std::ffi::CStr;

mod builders;
mod dsm;
mod exec;
mod hook;
mod path;
mod scan;

mod explainer;
pub mod pdbscan;

use crate::postgres::customscan::dsm::{
    estimate_dsm_custom_scan, initialize_dsm_custom_scan, initialize_worker_custom_scan,
    reinitialize_dsm_custom_scan,
};
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

pub trait CustomScanState: Default {}

pub trait CustomScan: Default + Sized {
    const NAME: &'static CStr;
    type State: CustomScanState;

    //
    // SAFETY:  We need to allocate the struct to define the functions once, however
    // all the methods are generic over this trait ([`CustomScan]).  Because Rust
    // monomorphizes these functions, they're actually at different addresses per CustomScan
    // impl.  As such, we allocate them once, in Postgres "TopMemoryContext", which is **never**
    // freed.  This ensures we don't waste any more memory than we need and more importantly,
    // ensures the returned pointer holding the function pointers lives for the life of the
    // process, which Postgres requires of these.
    //

    fn custom_path_methods() -> *const pg_sys::CustomPathMethods {
        unsafe {
            static mut METHODS: *mut pg_sys::CustomPathMethods = std::ptr::null_mut();

            if METHODS.is_null() {
                METHODS = PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(
                    pg_sys::CustomPathMethods {
                        CustomName: Self::NAME.as_ptr(),
                        PlanCustomPath: Some(plan_custom_path::<Self>),
                        ReparameterizeCustomPathByChild: Some(
                            reparameterize_custom_path_by_child::<Self>,
                        ),
                    },
                );
            }
            METHODS
        }
    }

    fn custom_scan_methods() -> *const pg_sys::CustomScanMethods {
        unsafe {
            static mut METHODS: *mut pg_sys::CustomScanMethods = std::ptr::null_mut();

            METHODS = PgMemoryContexts::TopMemoryContext.leak_and_drop_on_delete(
                pg_sys::CustomScanMethods {
                    CustomName: Self::NAME.as_ptr(),
                    CreateCustomScanState: Some(create_custom_scan_state::<Self>),
                },
            );
            METHODS
        }
    }

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

    fn callback(builder: CustomPathBuilder) -> Option<pg_sys::CustomPath>;

    fn plan_custom_path(builder: CustomScanBuilder) -> pg_sys::CustomScan;

    fn create_custom_scan_state(
        builder: CustomScanStateBuilder<Self>,
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

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot;

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>);

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>);
}

pub trait MarkRestoreCapable: CustomScan {
    fn exec_methods() -> pg_sys::CustomExecMethods {
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
        }
    }
}

pub trait ParallelQueryCapable: CustomScan {
    fn exec_methods() -> pg_sys::CustomExecMethods {
        pg_sys::CustomExecMethods {
            CustomName: Self::NAME.as_ptr(),
            BeginCustomScan: Some(begin_custom_scan::<Self>),
            ExecCustomScan: Some(exec_custom_scan::<Self>),
            EndCustomScan: Some(end_custom_scan::<Self>),
            ReScanCustomScan: Some(rescan_custom_scan::<Self>),
            MarkPosCustomScan: None,
            RestrPosCustomScan: None,
            EstimateDSMCustomScan: Some(estimate_dsm_custom_scan::<Self>),
            InitializeDSMCustomScan: Some(initialize_dsm_custom_scan::<Self>),
            ReInitializeDSMCustomScan: Some(reinitialize_dsm_custom_scan::<Self>),
            InitializeWorkerCustomScan: Some(initialize_worker_custom_scan::<Self>),
            ShutdownCustomScan: Some(shutdown_custom_scan::<Self>),
            ExplainCustomScan: Some(explain_custom_scan::<Self>),
        }
    }
}

pub trait ParallelQueryAndMarkRestoreCapable:
    CustomScan + ParallelQueryCapable + MarkRestoreCapable
{
    fn exec_methods() -> pg_sys::CustomExecMethods {
        pg_sys::CustomExecMethods {
            CustomName: Self::NAME.as_ptr(),
            BeginCustomScan: Some(begin_custom_scan::<Self>),
            ExecCustomScan: Some(exec_custom_scan::<Self>),
            EndCustomScan: Some(end_custom_scan::<Self>),
            ReScanCustomScan: Some(rescan_custom_scan::<Self>),
            MarkPosCustomScan: Some(mark_pos_custom_scan::<Self>),
            RestrPosCustomScan: Some(restr_pos_custom_scan::<Self>),
            EstimateDSMCustomScan: Some(estimate_dsm_custom_scan::<Self>),
            InitializeDSMCustomScan: Some(initialize_dsm_custom_scan::<Self>),
            ReInitializeDSMCustomScan: Some(reinitialize_dsm_custom_scan::<Self>),
            InitializeWorkerCustomScan: Some(initialize_worker_custom_scan::<Self>),
            ShutdownCustomScan: Some(shutdown_custom_scan::<Self>),
            ExplainCustomScan: Some(explain_custom_scan::<Self>),
        }
    }
}
