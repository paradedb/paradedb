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
use pgrx::memcx::MemCx;
use pgrx::{is_a, pg_sys, PgMemoryContexts};
use std::ffi::{c_void, CStr};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ptr::NonNull;

mod builders;
mod dsm;
mod exec;
mod hook;
mod path;
mod scan;

pub mod example;
mod explainer;
mod port;

use crate::customscan::dsm::{
    estimate_dsm_custom_scan, initialize_dsm_custom_scan, initialize_worker_custom_scan,
    reinitialize_dsm_custom_scan,
};
use crate::customscan::exec::{
    begin_custom_scan, end_custom_scan, exec_custom_scan, explain_custom_scan,
    mark_pos_custom_scan, rescan_custom_scan, restr_pos_custom_scan, shutdown_custom_scan,
};
use crate::customscan::scan::create_custom_scan_state;

use crate::customscan::builders::custom_path::CustomPathBuilder;
use crate::customscan::builders::custom_scan::CustomScanBuilder;
use crate::customscan::builders::custom_state::{CustomScanStateBuilder, CustomScanStateWrapper};
use crate::customscan::explainer::Explainer;
pub use hook::register_rel_pathlist;

pub trait CustomScanState: Default {}

pub trait CustomScan: Default + Sized {
    const NAME: &'static CStr;
    type State: CustomScanState;

    fn exec_methods() -> pg_sys::CustomExecMethods {
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
        }
    }

    fn callback(builder: CustomPathBuilder) -> Option<pg_sys::CustomPath>;

    fn plan_custom_path(builder: CustomScanBuilder) -> pg_sys::CustomScan;

    fn create_custom_scan_state(
        builder: CustomScanStateBuilder<Self>,
    ) -> CustomScanStateWrapper<Self>;

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

trait AsMemCx {
    unsafe fn as_memcx<'mcx>(&self) -> MemCx<'mcx>;
}

impl AsMemCx for PgMemoryContexts {
    /// Wrap the provided [`pg_sys::MemoryContext`]
    ///
    /// # Safety
    /// Assumes the provided [`pg_sys::MemoryContext`] is valid and properly initialized.
    /// This method does check to ensure the pointer is non-null, but that is the only sanity
    /// check that is performed.
    unsafe fn as_memcx<'mcx>(&self) -> MemCx<'mcx> {
        // SAFETY:  this must look exactly like `pgrx::memcx::MemCx`
        struct FakeMemCx<'mcx> {
            ptr: NonNull<pg_sys::MemoryContextData>,
            _marker: PhantomData<&'mcx pg_sys::MemoryContextData>,
        }
        let ptr = NonNull::new(self.value()).expect("memory context must be non-null");
        unsafe {
            // SAFETY:  `FakeMemCx` looks exactly like `pgrx::memcx::MemCx`
            std::mem::transmute(FakeMemCx {
                ptr,
                _marker: PhantomData,
            })
        }
    }
}

#[track_caller]
unsafe fn list<'a, T>(mcxt: &'a MemCx, list: *mut pg_sys::List) -> pgrx::list::List<'a, *mut T> {
    unsafe {
        let list = pgrx::list::List::<*mut c_void>::downcast_ptr_in_memcx(list, &mcxt)
            .expect("`list` must be a `pg_sys::List` as described by its `Node::tag`");
        std::mem::transmute(list)
    }
}

#[track_caller]
#[inline(always)]
unsafe fn node<T>(void: *mut c_void, tag: pg_sys::NodeTag) -> Option<*mut T> {
    let node: *mut T = void.cast();
    if !is_a(node.cast(), tag) {
        return None;
    }
    Some(node)
}
