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

use crate::postgres::customscan::CustomScan;
use pgrx::{pg_sys, PgList, PgMemoryContexts};
use std::fmt::{Debug, Formatter};
use std::ptr::addr_of_mut;

pub struct Args {
    pub cscan: *mut pg_sys::CustomScan,
}

#[repr(C)]
pub struct CustomScanStateWrapper<CS: CustomScan> {
    pub csstate: pg_sys::CustomScanState,
    pub custom_state: CS::State,
}

impl<CS: CustomScan> Debug for CustomScanStateWrapper<CS>
where
    CS::State: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!(
            "CustomScanStateWrapper<{}>",
            std::any::type_name::<CS>()
        ))
        .field("state_type", &std::any::type_name::<CS::State>())
        .field("csstate", &self.csstate)
        .field("custom_state", &self.custom_state)
        .finish()
    }
}

impl<CS: CustomScan> CustomScanStateWrapper<CS> {
    #[inline(always)]
    pub fn planstate(&mut self) -> *mut pg_sys::PlanState {
        addr_of_mut!(self.csstate.ss.ps)
    }

    #[inline(always)]
    pub fn custom_state(&mut self) -> &mut CS::State {
        &mut self.custom_state
    }

    #[inline(always)]
    pub fn expr_context(&self) -> *mut pg_sys::ExprContext {
        self.csstate.ss.ps.ps_ExprContext
    }

    #[inline(always)]
    pub fn scanslot(&self) -> *mut pg_sys::TupleTableSlot {
        self.csstate.ss.ss_ScanTupleSlot
    }

    #[inline(always)]
    pub fn projection_info(&self) -> *mut pg_sys::ProjectionInfo {
        self.csstate.ss.ps.ps_ProjInfo
    }

    #[inline(always)]
    pub fn projection_tupdesc(&self) -> pg_sys::TupleDesc {
        self.csstate.ss.ps.ps_ResultTupleDesc
    }

    #[inline(always)]
    pub fn set_projection_scanslot(&mut self, slot: *mut pg_sys::TupleTableSlot) {
        unsafe { (*(*self.csstate.ss.ps.ps_ProjInfo).pi_exprContext).ecxt_scantuple = slot }
    }
}

pub struct CustomScanStateBuilder<CS: CustomScan> {
    args: Args,

    custom_state: CS::State,
}

impl<CS: CustomScan> CustomScanStateBuilder<CS> {
    pub fn new(cscan: *mut pg_sys::CustomScan) -> Self {
        Self {
            args: Args { cscan },

            custom_state: CS::State::default(),
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn private_data(&self) -> PgList<pg_sys::Node> {
        unsafe { PgList::from_pg((*self.args.cscan).custom_private) }
    }

    pub fn custom_state(&mut self) -> &mut CS::State {
        &mut self.custom_state
    }

    pub fn target_list(&self) -> PgList<pg_sys::TargetEntry> {
        unsafe { PgList::from_pg((*self.args.cscan).scan.plan.targetlist) }
    }

    pub fn build(self) -> *mut CustomScanStateWrapper<CS> {
        let scan_state = pg_sys::ScanState {
            ps: pg_sys::PlanState {
                type_: pg_sys::NodeTag::T_CustomScanState,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            let cssw = pg_sys::palloc(size_of::<CustomScanStateWrapper<CS>>())
                .cast::<CustomScanStateWrapper<CS>>();
            (*cssw).csstate = pg_sys::CustomScanState {
                ss: scan_state,
                flags: 0,
                custom_ps: std::ptr::null_mut(),
                pscan_len: 0,
                methods: PgMemoryContexts::CurrentMemoryContext
                    .leak_and_drop_on_delete(CS::exec_methods()),
                slotOps: std::ptr::null_mut(),
            };

            addr_of_mut!((*cssw).custom_state).write(self.custom_state);

            cssw
        }
    }
}
