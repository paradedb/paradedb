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

use crate::customscan::path::{plan_custom_path, reparameterize_custom_path_by_child};
use crate::customscan::CustomScan;
use pgrx::list::List;
use pgrx::memcx::MemCx;
use pgrx::{pg_sys, PgMemoryContexts};
use std::collections::HashSet;
use std::ffi::c_void;
use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub struct Args {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub rti: pg_sys::Index,
    pub rte: *mut pg_sys::RangeTblEntry,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
#[repr(u32)]
pub enum Flags {
    /// #define CUSTOMPATH_SUPPORT_BACKWARD_SCAN	0x0001
    BackwardScan = 0x0001,

    /// #define CUSTOMPATH_SUPPORT_MARK_RESTORE		0x0002
    MarkRestore = 0x0002,

    /// #define CUSTOMPATH_SUPPORT_PROJECTION		0x0004
    Projection = 0x0004,
}

pub struct CustomPathBuilder<'mcx> {
    mcx: &'mcx MemCx<'mcx>,
    args: Args,
    flags: HashSet<Flags>,

    custom_path_node: pg_sys::CustomPath,

    custom_paths: List<'mcx, *mut c_void>,
    custom_private: List<'mcx, *mut c_void>,
}

impl Debug for CustomPathBuilder<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomPathBuilder")
            .field("args", &self.args)
            .field("flags", &self.flags)
            .field("path", &self.flags)
            .field("custom_paths", &self.custom_paths)
            .field("custom_private", &self.custom_private)
            .finish()
    }
}

impl<'mcx> CustomPathBuilder<'mcx> {
    pub fn new<CS: CustomScan>(
        mcx: &'mcx MemCx<'mcx>,
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        rti: pg_sys::Index,
        rte: *mut pg_sys::RangeTblEntry,
    ) -> CustomPathBuilder<'mcx> {
        Self {
            mcx,
            args: Args {
                root,
                rel,
                rti,
                rte,
            },
            flags: Default::default(),

            custom_path_node: pg_sys::CustomPath {
                path: pg_sys::Path {
                    type_: pg_sys::NodeTag::T_CustomPath,
                    pathtype: pg_sys::NodeTag::T_CustomScan,
                    parent: rel,
                    pathtarget: unsafe { *rel }.reltarget,
                    param_info: std::ptr::null_mut(),
                    parallel_aware: false,
                    parallel_safe: false,
                    parallel_workers: 0,
                    rows: 0.0,
                    startup_cost: 0.0,
                    total_cost: 0.0,
                    pathkeys: std::ptr::null_mut(),
                },
                flags: 0,
                custom_paths: std::ptr::null_mut(),
                custom_private: std::ptr::null_mut(),
                methods: PgMemoryContexts::For(unsafe { *root }.planner_cxt)
                    .leak_and_drop_on_delete(pg_sys::CustomPathMethods {
                        CustomName: CS::NAME.as_ptr(),
                        PlanCustomPath: Some(plan_custom_path::<CS>),
                        ReparameterizeCustomPathByChild: Some(
                            reparameterize_custom_path_by_child::<CS>,
                        ),
                    }),
            },
            custom_paths: List::Nil,
            custom_private: List::Nil,
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn clear_flags(mut self) -> Self {
        self.flags.clear();
        self
    }

    pub fn set_flag(mut self, flag: Flags) -> Self {
        self.flags.insert(flag);
        self
    }

    pub fn add_custom_path(mut self, path: *mut pg_sys::Path) -> Self {
        self.custom_paths
            .unstable_push_in_context(path.cast(), self.mcx);
        self
    }

    pub fn add_private_data(mut self, data: *mut pg_sys::ValUnion) -> Self {
        self.custom_private
            .unstable_push_in_context(data.cast(), self.mcx);
        self
    }

    pub fn set_rows(mut self, rows: pg_sys::Cardinality) -> Self {
        self.custom_path_node.path.rows = rows;
        self
    }

    pub fn set_startup_cost(mut self, cost: pg_sys::Cost) -> Self {
        self.custom_path_node.path.startup_cost = cost;
        self
    }

    pub fn set_total_cost(mut self, cost: pg_sys::Cost) -> Self {
        self.custom_path_node.path.total_cost = cost;
        self
    }

    pub fn build(mut self) -> pg_sys::CustomPath {
        self.custom_path_node.custom_paths = self.custom_paths.as_mut_ptr();
        self.custom_path_node.custom_private = self.custom_private.as_mut_ptr();
        self.custom_path_node.flags = self
            .flags
            .into_iter()
            .fold(0, |acc, flag| acc | flag as u32);

        self.custom_path_node
    }
}
