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

use crate::postgres::PgSearchRelation;
use crate::postgres::customscan::CustomScan;
use crate::postgres::customscan::aggregatescan::{
    AggregateScan, CustomScanBuildError, CustomScanClause,
};
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::utils::strip_unnest_and_relabel;
use crate::postgres::var::{VarContext, find_one_var_and_fieldname, find_var_relation};
use pgrx::PgList;
use pgrx::pg_sys;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
    pub original_type_oid: pg_sys::Oid,
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupByClause {
    grouping_columns: Vec<GroupingColumn>,
}

impl GroupByClause {
    pub fn grouping_columns(&self) -> Vec<GroupingColumn> {
        self.grouping_columns.clone()
    }

    /// The position of the grouping column over `field_name`, if there is one.
    pub fn position(&self, field_name: &str) -> Option<usize> {
        self.grouping_columns
            .iter()
            .position(|column| column.field_name == field_name)
    }

    /// Adds a grouping column after the existing ones and returns its position.
    pub fn push(&mut self, column: GroupingColumn) -> usize {
        self.grouping_columns.push(column);
        self.grouping_columns.len() - 1
    }
}

/// Resolves a grouping expression to the columnar field that AggregateScan
/// groups by, or returns the reason the expression cannot be grouped.
pub(super) fn resolve_grouping_column(
    args: &<AggregateScan as CustomScan>::Args,
    heap_rti: pg_sys::Index,
    index: &PgSearchRelation,
    expr: *mut pg_sys::Node,
) -> Result<GroupingColumn, String> {
    let schema = index.schema().expect("could not get index schema");
    let (expr, is_unnest) = strip_unnest_and_relabel(expr);
    let var_context = VarContext::from_planner(args.root);

    let (field_name, attno) =
        if let Some((var, field_name)) = unsafe { find_one_var_and_fieldname(var_context, expr) } {
            // JSON operator expression or complex field access
            let (heaprelid, attno, _) = unsafe { find_var_relation(var, args.root) };
            if heaprelid == pg_sys::InvalidOid {
                return Err("find_var_relation returned InvalidOid for var".to_string());
            }
            (field_name.to_string(), attno)
        } else if let Some(ff) =
            find_matching_fast_field(expr, &index.index_expressions(), schema.clone(), heap_rti)
        {
            (ff.name(), 0) // Complex expressions don't have a single attno
        } else {
            return Err("could not resolve grouping column from expression".to_string());
        };

    let Some(search_field) = schema.search_field(&field_name) else {
        return Err(format!(
            "grouping column {} is missing from index",
            field_name
        ));
    };
    // Reject NUMERIC fields - GROUP BY pushdown not supported
    // (NUMERIC values are stored scaled and would need descaling)
    if search_field.field_type().is_numeric() {
        return Err(format!(
            "grouping field {} is numeric, which is not supported",
            field_name
        ));
    }
    if !search_field.is_fast() {
        return Err(format!(
            "grouping column {} exists, but is not columnar",
            field_name
        ));
    }

    let is_array = schema
        .categorized_fields()
        .iter()
        .find(|(sf, _)| sf.field_name().as_ref() == field_name)
        .map(|(_, data)| data.is_array)
        .unwrap_or(false);
    if is_array && !is_unnest {
        return Err(format!(
            "grouping field {} is an array, which requires UNNEST() to be used in GROUP BY",
            field_name
        ));
    } else if !is_array && is_unnest {
        unreachable!(
            "Postgres should not allow UNNEST() on a non-array column: {}",
            field_name
        );
    }

    // Because AggregateScan bypasses Postgres's ExecProject for grouping columns
    // and maps them directly to INDEX_VARs pointing at the final scan slot,
    // Postgres expects the slot to contain the Datum of the *cast* type (e.g. TEXTOID),
    // not the base column type.
    // This approach is only valid for known-safe casts (handled in `group_key_to_datum`),
    // where the grouping and comparison semantics of the internal fast field value
    // are strictly equivalent to the semantics of the projected value. If they were
    // different, we would need to evaluate grouping expressions natively via ExecProject.
    let original_type_oid = search_field.field_type().typeoid().value();
    Ok(GroupingColumn {
        field_name,
        attno,
        original_type_oid,
    })
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
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Result<Self, CustomScanBuildError> {
        let mut groupby = Self::default();

        // The keys PostgreSQL sorts the groups by, in order. A GROUP BY key it
        // finds redundant is not among them; `TargetList` adds it from the output.
        let pathkeys = if args.root().group_pathkeys.is_null() {
            PgList::<pg_sys::PathKey>::new()
        } else {
            unsafe { PgList::<pg_sys::PathKey>::from_pg(args.root().group_pathkeys) }
        };

        for pathkey in pathkeys.iter_ptr() {
            let pathkey = unsafe { &*pathkey };
            let equivclass = unsafe { &*pathkey.pk_eclass };
            let members =
                unsafe { PgList::<pg_sys::EquivalenceMember>::from_pg(equivclass.ec_members) };

            // Any member of the equivalence class can stand in for the key.
            let mut resolved = Err("grouping column could not be found".to_string());
            for member in members.iter_ptr() {
                let expr = unsafe { (*member).em_expr } as *mut pg_sys::Node;
                resolved = resolve_grouping_column(args, heap_rti, index, expr);
                if resolved.is_ok() {
                    break;
                }
            }
            groupby.push(resolved?);
        }

        Ok(groupby)
    }
}
