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

use pgrx::{is_a, pg_sys};
use std::ffi::{c_void, CStr};

mod builders;
mod dsm;
mod exec;
mod hook;
mod path;
mod scan;

pub mod example;
mod explainer;
mod port;

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

#[macro_export]
macro_rules! nodecast {
    ($type_:ident, $kind:ident, $node:expr) => {
        node::<pg_sys::$type_>($node.cast(), pg_sys::NodeTag::$kind)
    };
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
