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
use crate::postgres::customscan::aggregatescan::aggregate_type::AggregateType;
use crate::postgres::customscan::aggregatescan::{AggregateScan, CustomScanClause};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::qual_inspect::QualExtractState;
use crate::postgres::customscan::CustomScan;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::postgres::PgSearchRelation;
use pgrx::pg_sys;
use pgrx::PgList;
use std::ptr::addr_of_mut;

/// Find the single Aggref node in an expression tree (handles wrapped aggregates like COALESCE(COUNT(*), 0))
/// Returns the pointer to the Aggref if exactly one is found, None if zero or multiple Aggrefs exist.
/// Expressions like COUNT(*) + SUM(x) will return None since we can't handle multiple aggregates.
unsafe fn find_single_aggref_in_expr(expr: *mut pg_sys::Node) -> Option<*mut pg_sys::Aggref> {
    use pgrx::pg_guard;

    struct WalkerContext {
        aggrefs: Vec<*mut pg_sys::Aggref>,
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn aggref_walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        let ctx = &mut *(context as *mut WalkerContext);

        // Check if this node is an Aggref
        if (*node).type_ == pg_sys::NodeTag::T_Aggref {
            ctx.aggrefs.push(node as *mut pg_sys::Aggref);
            // Continue walking to find any other Aggrefs (don't stop early)
        }

        // Continue walking into child nodes
        pg_sys::expression_tree_walker(node, Some(aggref_walker), context)
    }

    let mut context = WalkerContext {
        aggrefs: Vec::new(),
    };
    aggref_walker(expr, addr_of_mut!(context).cast());

    // Only return an Aggref if exactly one was found
    if context.aggrefs.len() == 1 {
        context.aggrefs.into_iter().next()
    } else {
        None
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TargetListEntry {
    // the grouping columns are not guaranteed to the in the same order in the GROUP BY vs target list,
    // so we store the index of the grouping column in the GROUP BY list
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
    /// Create a new TargetList with a single aggregate
    pub fn new(aggregate: AggregateType) -> Self {
        TargetList {
            entries: vec![TargetListEntry::Aggregate(aggregate)],
            groupby: Default::default(),
            uses_our_operator: false,
        }
    }

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

    /// Get the result type OID for the first aggregate
    pub fn singleton_result_type_oid(&self) -> pg_sys::Oid {
        let agg_count = self.aggregates().count();
        if agg_count > 1 {
            panic!("first_result_type_oid should only be called on a TargetList with a single aggregate");
        }
        self.aggregates()
            .next()
            .map(|agg| agg.result_type_oid())
            .unwrap_or(pg_sys::INT8OID)
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

                // Try to extract field name from the expression (handles both Var and JSON operators)
                if let Some((var, field_name)) =
                    find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node)
                {
                    // This could be a Var or a JSON projection (OpExpr) - check if it's a grouping column
                    // Find which grouping column this is
                    let mut found = false;
                    for (i, gc) in grouping_columns.iter().enumerate() {
                        // For JSON projections, the field_name will be like "metadata_json.value"
                        // and gc.field_name should match
                        if gc.field_name == field_name.clone().into_inner() {
                            entries.push(TargetListEntry::GroupingColumn(i));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return None;
                    }
                } else if let Some(aggref) = find_single_aggref_in_expr(expr as *mut pg_sys::Node) {
                    // Found an Aggref (either top-level or wrapped in COALESCE, NULLIF, etc.)
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
