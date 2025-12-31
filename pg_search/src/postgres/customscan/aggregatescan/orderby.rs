// Copyright (C) 2023-2026 ParadeDB, Inc.
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

use crate::api::{HashSet, OrderByFeature, OrderByInfo};
use crate::customscan::builders::custom_path::{CustomPathBuilder, OrderByStyle};
use crate::customscan::CustomScan;
use crate::postgres::customscan::aggregatescan::{AggregateScan, CustomScanClause};
use crate::postgres::customscan::pdbscan::extract_pathkey_styles_with_sortability_check;
use crate::postgres::customscan::pdbscan::PathKeyInfo;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::postgres::PgSearchRelation;
use pgrx::pg_sys;
use pgrx::PgList;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderByClause {
    #[serde(skip)]
    pathkeys: Option<PathKeyInfo>,
    orderby_info: Vec<OrderByInfo>,
    has_orderby: bool,
}

impl OrderByClause {
    /// Creates an OrderByClause that indicates there is an ORDER BY clause,
    /// but we couldn't extract it for pushdown (e.g., ORDER BY on aggregates)
    pub fn unpushable() -> Self {
        Self {
            pathkeys: None,
            orderby_info: Vec::new(),
            has_orderby: true,
        }
    }

    pub fn has_orderby(&self) -> bool {
        self.has_orderby
    }

    pub fn orderby_info(&self) -> Vec<OrderByInfo> {
        self.orderby_info.clone()
    }
}

impl CustomScanClause<AggregateScan> for OrderByClause {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        mut builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        if let Some(pathkeys) = self.pathkeys.as_ref().and_then(|pki| pki.pathkeys()) {
            for pathkey_style in pathkeys {
                builder = builder.add_path_key(pathkey_style);
            }
        };

        builder
    }

    fn from_pg(
        args: &Self::Args,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<Self> {
        let parse = args.root().parse;
        let schema = index.schema().ok()?;

        let sort_clause =
            unsafe { PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause) };

        let sort_fields = unsafe {
            sort_clause
                .iter_ptr()
                .filter_map(|sort_clause| {
                    let expr = pg_sys::get_sortgroupclause_expr(sort_clause, (*parse).targetList);
                    let var_context = VarContext::from_planner(args.root);
                    if let Some((_, field_name)) = find_one_var_and_fieldname(var_context, expr) {
                        Some(field_name)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>()
        };

        let pathkeys = unsafe {
            extract_pathkey_styles_with_sortability_check(
                args.root,
                heap_rti,
                &schema,
                |f| f.is_fast(),
                |_| false,
            )
        };

        let orderby_info = OrderByStyle::extract_orderby_info(pathkeys.pathkeys())
            .into_iter()
            .filter(|info| {
                if let OrderByFeature::Field(field_name) = &info.feature {
                    sort_fields.contains(field_name)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        let has_orderby = unsafe { !parse.is_null() && !(*parse).sortClause.is_null() };

        if unsafe { !(*parse).groupClause.is_null() } && orderby_info.len() != sort_clause.len() {
            return None;
        }

        Some(Self {
            pathkeys: Some(pathkeys),
            orderby_info,
            has_orderby,
        })
    }
}
