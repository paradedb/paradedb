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

use crate::postgres::customscan::scan::create_custom_scan_state;
use crate::postgres::customscan::CustomScan;
use pgrx::{node_to_string, pg_sys, PgList, PgMemoryContexts};
use std::ffi::c_void;
use std::fmt::{Debug, Formatter};

pub struct Args {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub best_path: *mut pg_sys::CustomPath,
    pub tlist: PgList<pg_sys::TargetEntry>,
    pub clauses: *mut pg_sys::List,
    pub custom_plans: *mut pg_sys::List,
}

impl Debug for Args {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Args")
            .field("root", &self.root)
            .field("rel", &self.rel)
            .field("best_path", &self.best_path)
            .field(
                "tlist",
                &self
                    .tlist
                    .iter_ptr()
                    .map(|te| unsafe { node_to_string(te.cast()) }.unwrap_or("<null>"))
                    .collect::<Vec<_>>(),
            )
            .field("clauses", &self.clauses)
            .field("custom_plans", &self.custom_plans)
            .finish()
    }
}

impl Args {
    pub fn target_list(&self) -> impl Iterator<Item = Option<&pg_sys::TargetEntry>> + '_ {
        self.tlist.iter_ptr().map(|entry| unsafe { entry.as_ref() })
    }
}

pub struct CustomScanBuilder {
    args: Args,

    custom_scan_node: pg_sys::CustomScan,
    custom_paths: PgList<c_void>,
    custom_private: PgList<pg_sys::Node>,
}

impl CustomScanBuilder {
    pub fn new<CS: CustomScan>(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        best_path: *mut pg_sys::CustomPath,
        tlist: *mut pg_sys::List,
        clauses: *mut pg_sys::List,
        custom_plans: *mut pg_sys::List,
    ) -> Self {
        let scan = pg_sys::CustomScan {
            flags: unsafe { (*best_path).flags },
            custom_private: unsafe { *best_path }.custom_private,
            custom_plans,
            methods: PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(
                pg_sys::CustomScanMethods {
                    CustomName: CS::NAME.as_ptr(),
                    CreateCustomScanState: Some(create_custom_scan_state::<CS>),
                },
            ),
            scan: pg_sys::Scan {
                plan: pg_sys::Plan {
                    type_: pg_sys::NodeTag::T_CustomScan,
                    targetlist: tlist,
                    ..Default::default()
                },
                scanrelid: unsafe { *rel }.relid,
            },
            ..Default::default()
        };

        CustomScanBuilder {
            args: Args {
                root,
                rel,
                best_path,
                tlist: unsafe { PgList::from_pg(tlist) },
                clauses,
                custom_plans,
            },
            custom_scan_node: scan,
            custom_paths: unsafe { PgList::from_pg(custom_plans) },
            custom_private: unsafe { PgList::from_pg(scan.custom_private) },
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn custom_private(&self) -> *mut pg_sys::List {
        unsafe { (*self.args.best_path).custom_private }
    }

    pub fn build(self) -> pg_sys::CustomScan {
        self.custom_scan_node
    }
}
