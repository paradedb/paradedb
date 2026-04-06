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

use crate::postgres::customscan::aggregatescan::{
    AggregateScan, CustomScanBuildError, CustomScanClause,
};
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CustomScan;
use crate::postgres::utils::strip_unnest_and_relabel;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::postgres::PgSearchRelation;
use pgrx::pg_sys;
use pgrx::PgList;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupByClause {
    grouping_columns: Vec<GroupingColumn>,
}

impl GroupByClause {
    pub fn grouping_columns(&self) -> Vec<GroupingColumn> {
        self.grouping_columns.clone()
    }
}

impl CustomScanClause<AggregateScan> for GroupByClause {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        builder
    }

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        if self.grouping_columns.is_empty() {
            return Box::new(std::iter::empty());
        }

        let joined = self
            .grouping_columns
            .iter()
            .map(|column| column.field_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        Box::new(std::iter::once((String::from("Group By"), joined)))
    }

    fn from_pg(
        args: &Self::Args,
        _heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Result<Self, CustomScanBuildError> {
        let mut grouping_columns = Vec::new();
        let schema = index.schema().expect("could not get index schema");
        let index_expressions = index.index_expressions();
        let categorized_fields = schema.categorized_fields();

        let pathkeys = if args.root().group_pathkeys.is_null() {
            PgList::<pg_sys::PathKey>::new()
        } else {
            unsafe { PgList::<pg_sys::PathKey>::from_pg(args.root().group_pathkeys) }
        };

        for pathkey in pathkeys.iter_ptr() {
            unsafe {
                let equivclass = (*pathkey).pk_eclass;
                let members =
                    PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

                let mut found_valid_column = false;
                // Track the most recent error reason across equivalence members.
                // If no valid column is found, we report this to the caller.
                let mut last_error: Option<String> = None;

                for member in members.iter_ptr() {
                    let (expr, is_unnest) =
                        strip_unnest_and_relabel((*member).em_expr as *mut pg_sys::Node);

                    let var_context = VarContext::from_planner(args.root);

                    let (field_name, attno) = if let Some((var, field_name)) =
                        find_one_var_and_fieldname(var_context, expr)
                    {
                        // JSON operator expression or complex field access
                        let (heaprelid, attno, _) = find_var_relation(var, args.root);
                        if heaprelid == pg_sys::InvalidOid {
                            last_error =
                                Some("find_var_relation returned InvalidOid for var".to_string());
                            continue;
                        }
                        (field_name.to_string(), attno)
                    } else if let Some(ff) = find_matching_fast_field(
                        expr,
                        &index_expressions,
                        schema.clone(),
                        _heap_rti,
                    ) {
                        (ff.name(), 0) // Complex expressions don't have a single attno
                    } else {
                        last_error =
                            Some("could not resolve grouping column from expression".to_string());
                        continue;
                    };

                    // Check if this field exists in the index schema as a fast field
                    if let Some(search_field) = schema.search_field(&field_name) {
                        // Reject NUMERIC fields - GROUP BY pushdown not supported
                        // (NUMERIC values are stored scaled and would need descaling)
                        if search_field.field_type().is_numeric() {
                            return Err(format!(
                                "grouping field {} is numeric, which is not supported",
                                field_name
                            )
                            .into());
                        }
                        if search_field.is_fast() {
                            let is_array = categorized_fields
                                .iter()
                                .find(|(sf, _)| sf.field_name().as_ref() == field_name)
                                .map(|(_, data)| data.is_array)
                                .unwrap_or(false);

                            if is_array && !is_unnest {
                                return Err(format!(
                                    "grouping field {} is an array, which requires UNNEST() to be used in GROUP BY",
                                    field_name
                                )
                                .into());
                            } else if !is_array && is_unnest {
                                unreachable!(
                                    "Postgres should not allow UNNEST() on a non-array column: {}",
                                    field_name
                                );
                            }

                            grouping_columns.push(GroupingColumn { field_name, attno });
                            found_valid_column = true;
                            break; // Found a valid grouping column for this pathkey
                        } else {
                            last_error = Some(format!(
                                "grouping column {} exists, but is not a fast field",
                                field_name
                            ));
                            // wait to return error until we check all members
                        }
                    } else {
                        last_error = Some(format!(
                            "grouping column {} is missing from index",
                            field_name
                        ));
                        // wait to return error
                    }
                }

                if !found_valid_column {
                    if let Some(err) = last_error {
                        return Err(err.into());
                    } else {
                        return Err("grouping column could not be found".into());
                    }
                }
            }
        }

        Ok(Self { grouping_columns })
    }
}
