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

pub mod privdat;

use std::ffi::CStr;

use crate::postgres::customscan::aggregatescan::privdat::PrivateData;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::{
    CreateUpperPathsHookArgs, CustomScan, CustomScanState, ExecMethod,
};

use pgrx::pg_sys;

#[derive(Default)]
pub struct AggregateScan;

impl CustomScan for AggregateScan {
    const NAME: &'static CStr = c"ParadeDB Aggregate Scan";
    type Args = CreateUpperPathsHookArgs;
    type State = AggregateScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        todo!("TODO: create_custom_path")
    }

    fn plan_custom_path(builder: CustomScanBuilder<Self::PrivateData>) -> pg_sys::CustomScan {
        todo!("TODO: plan_custom_path")
    }

    fn create_custom_scan_state(
        builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        todo!("TODO: plan_custom_path")
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pg_sys::List,
        explainer: &mut Explainer,
    ) {
        todo!("TODO: explain_custom_scan")
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        todo!("TODO: begin_custom_scan")
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        todo!("TODO: rescan_custom_scan")
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        todo!("TODO: exec_custom_scan")
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        todo!("TODO: shutdown_custom_scan")
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        todo!("TODO: end_custom_scan")
    }
}

impl ExecMethod for AggregateScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        todo!("TODO: exec_methods")
    }
}

#[derive(Default)]
pub struct AggregateScanState;

impl CustomScanState for AggregateScanState {
    fn init_exec_method(&mut self, cstate: *mut pg_sys::CustomScanState) {
        todo!("TODO: init_exec_method")
    }
}
