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

pub mod build;
pub mod privdat;
pub mod scan_state;

use self::privdat::PrivateData;
use self::scan_state::JoinScanState;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::{CustomScan, ExecMethod, JoinPathlistHookArgs, PlainExecCapable};
use pgrx::pg_sys;
use std::ffi::CStr;

#[derive(Default)]
pub struct JoinScan;

impl CustomScan for JoinScan {
    const NAME: &'static CStr = c"ParadeDB Join Scan";
    type Args = JoinPathlistHookArgs;
    type State = JoinScanState;
    type PrivateData = PrivateData;

    fn create_custom_path(_builder: CustomPathBuilder<Self>) -> Option<pg_sys::CustomPath> {
        // Planning should never trigger for now
        None
    }

    fn plan_custom_path(builder: CustomScanBuilder<Self>) -> pg_sys::CustomScan {
        builder.build()
    }

    fn create_custom_scan_state(
        builder: CustomScanStateBuilder<Self, Self::PrivateData>,
    ) -> *mut CustomScanStateWrapper<Self> {
        builder.build()
    }

    fn explain_custom_scan(
        _state: &CustomScanStateWrapper<Self>,
        _ancestors: *mut pg_sys::List,
        _explainer: &mut Explainer,
    ) {
    }

    fn begin_custom_scan(
        _state: &mut CustomScanStateWrapper<Self>,
        _estate: *mut pg_sys::EState,
        _eflags: i32,
    ) {
    }

    fn rescan_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn exec_custom_scan(_state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        todo!("JoinScan execution not implemented")
    }

    fn shutdown_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(_state: &mut CustomScanStateWrapper<Self>) {}
}

impl ExecMethod for JoinScan {
    fn exec_methods() -> *const pg_sys::CustomExecMethods {
        <JoinScan as PlainExecCapable>::exec_methods()
    }
}

impl PlainExecCapable for JoinScan {}
