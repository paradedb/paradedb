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

use crate::customscan::builders::custom_path::CustomPathBuilder;
use crate::customscan::builders::custom_scan::CustomScanBuilder;
use crate::customscan::builders::custom_state::{CustomScanStateBuilder, CustomScanStateWrapper};
use crate::customscan::explainer::Explainer;
use crate::customscan::{CustomScan, CustomScanState};
use pgrx::list::List;
use pgrx::pg_sys;
use pgrx::pg_sys::{
    AsPgCStr, CustomPath, EState, ExplainState, Node, PlannerInfo, RelOptInfo, TupleTableSlot,
    ValUnion,
};
use std::ffi::CStr;

pub struct Example;

#[derive(Debug, Default)]
pub struct ExampleScanState {
    my_data: String,
}

impl CustomScanState for ExampleScanState {}

impl CustomScan for Example {
    const NAME: &'static CStr = c"ParadeDB Example Scan";
    type State = ExampleScanState;

    fn callback(builder: CustomPathBuilder) -> Option<CustomPath> {
        pgrx::warning!("root={:#?}", unsafe { *(*builder.args().root).parse });
        pgrx::warning!("rel={:#?}", unsafe { &*builder.args().rel });
        pgrx::warning!("rte={:#?}", unsafe { &*builder.args().rte });
        Some(builder.build())
    }

    fn plan_custom_path(builder: CustomScanBuilder) -> pgrx::pg_sys::CustomScan {
        builder.build()
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self>,
    ) -> CustomScanStateWrapper<Self> {
        builder.custom_state().my_data = String::from("Hello, world");
        builder.build()
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pgrx::pg_sys::List,
        explainer: &mut Explainer,
    ) {
        explainer.add_text("my_data", &state.custom_state.my_data);
        explainer.add_integer("A Random Integer", rand::random(), None);
        explainer.add_unsigned_integer(
            "A Random Unsigned Integer",
            rand::random::<u64>() / 1024,
            Some("kB"),
        );
        explainer.add_bool("A boolean", true);
        explainer.add_float("A random float", rand::random(), None, 4);
        explainer.add_float("A random float", rand::random(), Some("minutes"), 12);
    }

    fn begin_custom_scan(state: &mut CustomScanStateWrapper<Self>, estate: &EState, eflags: i32) {}

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut TupleTableSlot {
        todo!()
    }

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}
}
