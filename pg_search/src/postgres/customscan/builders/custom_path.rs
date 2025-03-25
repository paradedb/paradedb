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

use crate::api::Cardinality;
use crate::postgres::customscan::CustomScan;
use pgrx::{pg_sys, PgList};
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Default, Copy, Clone)]
#[repr(i32)]
pub enum SortDirection {
    #[default]
    Asc = pg_sys::BTLessStrategyNumber as i32,
    Desc = pg_sys::BTGreaterStrategyNumber as i32,
    None = pg_sys::BTEqualStrategyNumber as i32,
}

impl AsRef<str> for SortDirection {
    fn as_ref(&self) -> &str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
            SortDirection::None => "<none>",
        }
    }
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl From<i32> for SortDirection {
    fn from(value: i32) -> Self {
        SortDirection::from(value as u32)
    }
}

impl From<u32> for SortDirection {
    fn from(value: u32) -> Self {
        match value {
            pg_sys::BTLessStrategyNumber => SortDirection::Asc,
            pg_sys::BTGreaterStrategyNumber => SortDirection::Desc,
            _ => panic!("unrecognized sort strategy number: {value}"),
        }
    }
}

impl From<SortDirection> for crate::index::reader::index::SortDirection {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Asc => crate::index::reader::index::SortDirection::Asc,
            SortDirection::Desc => crate::index::reader::index::SortDirection::Desc,
            SortDirection::None => crate::index::reader::index::SortDirection::None,
        }
    }
}

impl From<SortDirection> for u32 {
    fn from(value: SortDirection) -> Self {
        value as _
    }
}

pub enum OrderByStyle {
    Score(*mut pg_sys::PathKey),
    Field(*mut pg_sys::PathKey, String),
}

impl OrderByStyle {
    pub fn pathkey(&self) -> *mut pg_sys::PathKey {
        match self {
            OrderByStyle::Score(pathkey) => *pathkey,
            OrderByStyle::Field(pathkey, _) => *pathkey,
        }
    }

    pub fn direction(&self) -> SortDirection {
        unsafe {
            let pathkey = self.pathkey();
            assert!(!pathkey.is_null());

            (*self.pathkey()).pk_strategy.into()
        }
    }
}

#[derive(Debug)]
pub struct Args {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub rti: pg_sys::Index,
    pub rte: *mut pg_sys::RangeTblEntry,
}

impl Args {
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

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
#[repr(u32)]
#[allow(dead_code)]
pub enum Flags {
    /// #define CUSTOMPATH_SUPPORT_BACKWARD_SCAN	0x0001
    BackwardScan = 0x0001,

    /// #define CUSTOMPATH_SUPPORT_MARK_RESTORE		0x0002
    MarkRestore = 0x0002,

    /// #define CUSTOMPATH_SUPPORT_PROJECTION		0x0004
    Projection = 0x0004,

    /// ParadeDB custom flag for indicating we want to force the plan to be used
    Force = 0x0008,
}

pub struct CustomPathBuilder<P: Into<*mut pg_sys::List> + Default> {
    args: Args,
    flags: HashSet<Flags>,

    custom_path_node: pg_sys::CustomPath,

    custom_paths: PgList<pg_sys::Path>,

    /// `custom_private` can be used to store the custom path's private data. Private data should be
    /// stored in a form that can be handled by nodeToString, so that debugging routines that attempt
    /// to print the custom path will work as designed.
    custom_private: P,
}

#[derive(Copy, Clone, Debug)]
pub enum RestrictInfoType {
    BaseRelation,
    Join,
    None,
}

impl<P: Into<*mut pg_sys::List> + Default> CustomPathBuilder<P> {
    pub fn new<CS: CustomScan>(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        rti: pg_sys::Index,
        rte: *mut pg_sys::RangeTblEntry,
    ) -> CustomPathBuilder<P> {
        unsafe {
            Self {
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
                        pathtarget: (*rel).reltarget,
                        param_info: pg_sys::get_baserel_parampathinfo(
                            root,
                            rel,
                            pg_sys::bms_copy((*rel).lateral_relids),
                        ),
                        ..Default::default()
                    },
                    methods: CS::custom_path_methods(),
                    ..Default::default()
                },
                custom_paths: PgList::default(),
                custom_private: P::default(),
            }
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    //
    // convenience getters for type safety
    //

    pub fn restrict_info(&self) -> (PgList<pg_sys::RestrictInfo>, RestrictInfoType) {
        unsafe {
            let baseri = PgList::from_pg(self.args.rel().baserestrictinfo);
            let joinri = PgList::from_pg(self.args.rel().joininfo);

            if baseri.is_empty() && joinri.is_empty() {
                // both lists are empty, so return an empty list
                (PgList::new(), RestrictInfoType::None)
            } else if !baseri.is_empty() {
                // the baserestrictinfo has entries, so we prefer that first
                (baseri, RestrictInfoType::BaseRelation)
            } else {
                // only the joininfo has entries, so that's what we'll use
                (joinri, RestrictInfoType::Join)
            }
        }
    }

    #[allow(dead_code)]
    pub fn path_target(&self) -> *mut pg_sys::PathTarget {
        self.args.rel().reltarget
    }

    #[allow(dead_code)]
    pub fn limit(&self) -> i32 {
        unsafe { (*self.args().root).limit_tuples.round() as i32 }
    }

    //
    // public settings
    //

    #[allow(dead_code)]
    pub fn clear_flags(mut self) -> Self {
        self.flags.clear();
        self
    }

    pub fn set_flag(mut self, flag: Flags) -> Self {
        self.flags.insert(flag);
        self
    }

    #[allow(dead_code)]
    pub fn add_custom_path(mut self, path: *mut pg_sys::Path) -> Self {
        self.custom_paths.push(path);
        self
    }

    pub fn custom_private(&mut self) -> &mut P {
        &mut self.custom_private
    }

    pub fn set_rows(mut self, rows: Cardinality) -> Self {
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

    pub fn add_path_key(mut self, pathkey: &Option<OrderByStyle>) -> Self {
        unsafe {
            if let Some(style) = pathkey {
                let mut pklist =
                    PgList::<pg_sys::PathKey>::from_pg(self.custom_path_node.path.pathkeys);
                pklist.push(style.pathkey());

                self.custom_path_node.path.pathkeys = pklist.into_pg();
            }
            self
        }
    }

    pub fn set_force_path(mut self, force: bool) -> Self {
        if force {
            self.flags.insert(Flags::Force);
        } else {
            self.flags.remove(&Flags::Force);
        }
        self
    }

    pub fn set_parallel(
        mut self,
        is_topn: bool,
        row_estimate: Cardinality,
        limit: Option<Cardinality>,
        segment_count: usize,
        sorted: bool,
    ) -> Self {
        unsafe {
            let mut nworkers = segment_count.min(pg_sys::max_parallel_workers as usize);

            if limit.is_some() {
                let limit = limit.unwrap();
                if !sorted
                    && limit <= (segment_count * segment_count * segment_count) as Cardinality
                {
                    // not worth it to do a parallel scan
                    return self;
                }

                // if the limit is less than some arbitrarily large value
                // use at most half the number of parallel workers as there are segments
                // this generally seems to perform better than directly using `max_parallel_workers`
                if limit < 1_000_000.0 {
                    nworkers = (segment_count / 2).min(nworkers);
                }
            }

            #[cfg(not(any(feature = "pg14", feature = "pg15")))]
            {
                if nworkers == 0 && pg_sys::debug_parallel_query != 0 {
                    // force a parallel worker if the `debug_parallel_query` GUC is on
                    nworkers = 1;
                }
            }

            // we will try to parallelize based on the number of index segments
            if nworkers > 0 && (*self.args.rel).consider_parallel {
                self.custom_path_node.path.parallel_aware = true;
                self.custom_path_node.path.parallel_safe = true;
                self.custom_path_node.path.parallel_workers =
                    nworkers.try_into().expect("nworkers should be a valid i32");
            }

            self
        }
    }

    pub fn build(mut self) -> pg_sys::CustomPath {
        self.custom_path_node.custom_paths = self.custom_paths.into_pg();
        self.custom_path_node.custom_private = self.custom_private.into();
        self.custom_path_node.flags = self
            .flags
            .into_iter()
            .fold(0, |acc, flag| acc | flag as u32);

        self.custom_path_node
    }
}
