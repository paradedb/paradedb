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

use crate::api::{Cardinality, FieldName, HashSet, OrderByFeature, OrderByInfo, SortDirection};
use crate::index::fast_fields_helper::WhichFastField;
use crate::postgres::customscan::CustomScan;
use pgrx::{pg_sys, PgList};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum OrderByStyle {
    Score(*mut pg_sys::PathKey),
    Field(*mut pg_sys::PathKey, FieldName),
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

            match (*pathkey).pk_strategy as u32 {
                pg_sys::BTLessStrategyNumber => SortDirection::Asc,
                pg_sys::BTGreaterStrategyNumber => SortDirection::Desc,
                value => panic!("unrecognized sort strategy number: {value}"),
            }
        }
    }

    /// Extract ORDER BY information from query pathkeys
    /// In this case, we convert OrderByStyle to OrderByInfo for serialization.
    pub fn extract_order_by_info(order_pathkeys: &Option<Vec<OrderByStyle>>) -> Vec<OrderByInfo> {
        order_pathkeys
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(|style| style.into())
            .collect()
    }
}

impl From<&OrderByStyle> for OrderByInfo {
    fn from(value: &OrderByStyle) -> Self {
        let feature = match value {
            OrderByStyle::Field(_, name) => OrderByFeature::Field(name.to_owned()),
            OrderByStyle::Score(_) => OrderByFeature::Score,
        };
        OrderByInfo {
            feature,
            direction: value.direction(),
        }
    }
}

///
/// The type of ExecMethod that was chosen at planning time. We fully select an ExecMethodType at
/// planning time in order to be able to make claims about the sortedness and estimates for our
/// execution.
///
/// `which_fast_fields` lists in this enum are _all_ of the fast fields which were identified at
/// planning time: based on the join order that the planner ends up choosing, only a subset of
/// these might be used at execution time (in an order specified by the execution time target
/// list), but never a superset.
///
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum ExecMethodType {
    #[default]
    Normal,
    TopN {
        heaprelid: pg_sys::Oid,
        limit: usize,
        orderby_info: Option<Vec<OrderByInfo>>,
    },
    FastFieldMixed {
        which_fast_fields: HashSet<WhichFastField>,
        limit: Option<usize>,
    },
}

impl ExecMethodType {
    ///
    /// Returns true if this is a sorted TopN execution.
    ///
    pub fn is_sorted_topn(&self) -> bool {
        matches!(
            self,
            ExecMethodType::TopN {
                orderby_info: Some(..),
                ..
            }
        )
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

pub struct CustomPathBuilder<CS: CustomScan> {
    args: CS::Args,
    flags: HashSet<Flags>,

    custom_path_node: pg_sys::CustomPath,

    custom_paths: PgList<pg_sys::Path>,
}

#[derive(Copy, Clone, Debug)]
pub enum RestrictInfoType {
    BaseRelation,
    Join,
    None,
}

pub fn restrict_info(rel: &pg_sys::RelOptInfo) -> (PgList<pg_sys::RestrictInfo>, RestrictInfoType) {
    unsafe {
        let baseri = PgList::from_pg(rel.baserestrictinfo);
        let joinri = PgList::from_pg(rel.joininfo);

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

impl<CS: CustomScan> CustomPathBuilder<CS> {
    pub fn new(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        args: CS::Args,
    ) -> Self {
        unsafe {
            Self {
                args,
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
            }
        }
    }

    pub fn args(&self) -> &CS::Args {
        &self.args
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

    pub fn add_path_key(mut self, style: &OrderByStyle) -> Self {
        unsafe {
            let mut pklist =
                PgList::<pg_sys::PathKey>::from_pg(self.custom_path_node.path.pathkeys);
            pklist.push(style.pathkey());

            self.custom_path_node.path.pathkeys = pklist.into_pg();
        }
        self
    }

    pub fn set_force_path(mut self, force: bool) -> Self {
        if force {
            self.flags.insert(Flags::Force);
        } else {
            self.flags.remove(&Flags::Force);
        }
        self
    }

    pub fn set_parallel(mut self, nworkers: usize) -> Self {
        self.custom_path_node.path.parallel_aware = true;
        self.custom_path_node.path.parallel_safe = true;
        self.custom_path_node.path.parallel_workers =
            nworkers.try_into().expect("nworkers should be a valid i32");

        self
    }

    /// Build a CustomPath using the given private data.
    ///
    /// `custom_private` can be used to store the custom path's private data.
    pub fn build(mut self, custom_private: CS::PrivateData) -> pg_sys::CustomPath {
        self.custom_path_node.custom_paths = self.custom_paths.into_pg();
        self.custom_path_node.custom_private = custom_private.into();
        self.custom_path_node.flags = self
            .flags
            .into_iter()
            .fold(0, |acc, flag| acc | flag as u32);

        self.custom_path_node
    }
}
