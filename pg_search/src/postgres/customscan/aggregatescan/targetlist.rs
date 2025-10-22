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

use crate::customscan::aggregatescan::{GroupByClause, GroupingColumn};
use crate::nodecast;
use crate::postgres::customscan::aggregatescan::aggregate_type::AggregateType;
use crate::postgres::customscan::aggregatescan::{AggregateScan, CustomScanClause};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::qual_inspect::QualExtractState;
use crate::postgres::customscan::CustomScan;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::postgres::PgSearchRelation;
use pgrx::pg_sys;
use pgrx::PgList;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TargetListEntry {
    // the grouping columns are not guaranteed to the in the same order in the GROUP BY vs target list,
    // so we store the index of the grouping column in the GROUP BY list
    // todo (@rebasedming): we should sort the grouping columns so they match the order in the target list
    GroupingColumn(usize),
    Aggregate(AggregateType),
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TargetList {
    entries: Vec<TargetListEntry>,
    groupby: GroupByClause,
    uses_our_operator: bool,
}

impl TargetList {
    pub fn aggregates(&self) -> impl Iterator<Item = &AggregateType> {
        self.entries.iter().filter_map(|entry| match entry {
            TargetListEntry::Aggregate(aggregate) => Some(aggregate),
            TargetListEntry::GroupingColumn(_) => None,
        })
    }

    pub fn aggregates_mut(&mut self) -> impl Iterator<Item = &mut AggregateType> {
        self.entries.iter_mut().filter_map(|entry| match entry {
            TargetListEntry::Aggregate(aggregate) => Some(aggregate),
            TargetListEntry::GroupingColumn(_) => None,
        })
    }

    pub fn entries(&self) -> impl Iterator<Item = &TargetListEntry> {
        self.entries.iter()
    }

    pub fn groupby(&self) -> &GroupByClause {
        &self.groupby
    }

    pub fn grouping_columns(&self) -> Vec<GroupingColumn> {
        self.groupby.grouping_columns()
    }

    pub fn uses_our_operator(&self) -> bool {
        self.uses_our_operator
    }
}

impl CustomScanClause<AggregateScan> for TargetList {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        self.groupby.add_to_custom_path(builder)
    }

    fn from_pg(
        args: &Self::Args,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<Self> {
        // Check for DISTINCT - we can't handle DISTINCT queries
        unsafe {
            let parse = args.root().parse;
            if !parse.is_null() && (!(*parse).distinctClause.is_null() || (*parse).hasDistinctOn) {
                return None;
            }
        }

        let schema = index.schema().ok()?;
        let target_list =
            unsafe { PgList::<pg_sys::Expr>::from_pg((*args.output_rel().reltarget).exprs) };
        if target_list.is_empty() {
            return None;
        }

        let heap_rte = unsafe {
            let rt = PgList::<pg_sys::RangeTblEntry>::from_pg((*args.root().parse).rtable);
            rt.get_ptr((heap_rti - 1) as usize)?
        };
        let heap_oid = unsafe { (*heap_rte).relid };

        let groupby_clause = GroupByClause::from_pg(args, heap_rti, index)?;
        let grouping_columns = groupby_clause.grouping_columns();
        let mut entries = Vec::new();
        let mut uses_our_operator = false;

        for expr in target_list.iter_ptr() {
            unsafe {
                let node_tag = (*expr).type_;
                let var_context = VarContext::from_planner(args.root() as *const _ as *mut _);

                if let Some((var, field_name)) =
                    find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node)
                {
                    // This is a Var - it should be a grouping column
                    // Find which grouping column this is
                    let mut found = false;
                    for (i, gc) in grouping_columns.iter().enumerate() {
                        if (*var).varattno == gc.attno
                            && gc.field_name == field_name.clone().into_inner()
                        {
                            entries.push(TargetListEntry::GroupingColumn(i));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return None;
                    }
                } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, expr) {
                    // TODO: Support DISTINCT
                    if !(*aggref).aggdistinct.is_null() {
                        return None;
                    }

                    let mut qual_state = QualExtractState::default();
                    let aggregate = AggregateType::try_from(
                        aggref,
                        heap_oid,
                        index,
                        args.root,
                        heap_rti,
                        &mut qual_state,
                    )?;
                    uses_our_operator = uses_our_operator || qual_state.uses_our_operator;

                    if let Some(field_name) = aggregate.field_name() {
                        if let Some(search_field) = schema.search_field(&field_name) {
                            if !search_field.is_fast() {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }

                    entries.push(TargetListEntry::Aggregate(aggregate));
                } else {
                    return None;
                }
            }
        }

        Some(TargetList {
            entries,
            groupby: groupby_clause,
            uses_our_operator,
        })
    }
}
