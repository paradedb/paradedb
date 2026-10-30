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

use crate::postgres::customscan::CustomScan;
use crate::postgres::customscan::aggregatescan::{
    AggregateScan, CustomScanBuildError, CustomScanClause, GroupingPushdownDeclineReason,
};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::pullup::{Access, SourceContext};
use crate::postgres::utils::{strip_relabel, strip_unnest_and_relabel};
use crate::postgres::var::{VarContext, find_one_resolved_var_and_fieldname};
use crate::schema::{SearchFieldType, SearchIndexSchema};
use pgrx::pg_sys;
use pgrx::{PgList, pg_guard};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub original_type_oid: pg_sys::Oid,
    /// True when the NULL bucket of this column is keyed by a string sentinel (text, bool and
    /// JSON fields), as opposed to a number such as `i64::MAX`.
    #[serde(default)]
    pub string_null_sentinel: bool,
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

    fn from_pg(args: &Self::Args, context: &SourceContext) -> Result<Self, CustomScanBuildError> {
        let mut grouping_columns = Vec::new();
        let schema = context.schema().expect("could not get index schema");

        // Use PostgreSQL's processed pathkeys, not `parse.groupClause`: redundant
        // GROUP BY columns may be removed, and AggregateScan eligibility validates
        // this same representation before allowing pushdown.
        for pathkey in args.group_by_pathkeys() {
            let pathkey = unsafe { &*pathkey };
            let equivclass = unsafe { &*pathkey.pk_eclass };
            let members =
                unsafe { PgList::<pg_sys::EquivalenceMember>::from_pg(equivclass.ec_members) };

            let mut found_valid_column = false;
            // Track the most recent error reason across equivalence members.
            // If no valid column is found, we report this to the caller.
            let mut last_error: Option<String> = None;

            for member in members.iter_ptr() {
                let member = unsafe { &*member };
                let (expr, is_unnest) =
                    strip_unnest_and_relabel(member.em_expr as *mut pg_sys::Node);
                let access = if is_unnest {
                    Access::List
                } else {
                    Access::Scalar
                };

                let var_context = VarContext::from_planner(args.root);

                let (resolved, field_name) = unsafe {
                    find_one_resolved_var_and_fieldname(var_context, context, expr, access)
                };
                let Some(resolved) = resolved else {
                    last_error = Some(match field_name {
                        Some(name) => unresolved_reason(&schema, &name, is_unnest)?,
                        None => "could not resolve grouping column from expression".to_string(),
                    });
                    continue;
                };

                let field_type = resolved
                    .field
                    .field_type()
                    .expect("All returned fast fields at this point should have a field type");
                if !resolved.field.is_array() && is_unnest {
                    unreachable!(
                        "Postgres should not allow UNNEST() on a non-array column: {}",
                        resolved.field.name()
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
                let original_type_oid = field_type.typeoid().value();
                let string_null_sentinel = matches!(
                    field_type,
                    SearchFieldType::Text(_)
                        | SearchFieldType::Tokenized(..)
                        | SearchFieldType::Bool(_)
                        | SearchFieldType::Json(_)
                );
                grouping_columns.push(GroupingColumn {
                    field_name: resolved.field.name(),
                    original_type_oid,
                    string_null_sentinel,
                });
                found_valid_column = true;
                break; // Found a valid grouping column for this pathkey
            }

            if !found_valid_column {
                if let Some(err) = last_error {
                    return Err(err.into());
                } else {
                    return Err("grouping column could not be found".into());
                }
            }
        }

        // PostgreSQL groups on no key when the WHERE clause pins every key to a
        // constant. The query then has one group or none, which this backend
        // cannot tell apart.
        let parse = args.root().parse;
        if grouping_columns.is_empty() && unsafe { !(*parse).groupClause.is_null() } {
            return Err(GroupingPushdownDeclineReason::MissingPathKeys
                .detail()
                .into());
        }

        Ok(Self { grouping_columns })
    }
}

/// Why `name` did not resolve as a grouping column. `Err` declines outright,
/// `Ok` is a reason to keep trying the other equivalence members.
fn unresolved_reason(
    schema: &SearchIndexSchema,
    name: &str,
    is_unnest: bool,
) -> Result<String, CustomScanBuildError> {
    let Some((search_field, data)) = schema.search_field_with_data(name) else {
        return Ok(format!("grouping column {name} is missing from index"));
    };
    if search_field.field_type().is_numeric() {
        return Err(format!("grouping field {name} is numeric, which is not supported").into());
    }
    if !search_field.is_fast() {
        return Ok(format!(
            "grouping column {name} exists, but is not columnar"
        ));
    }
    if data.is_array && !is_unnest {
        return Err(format!(
            "grouping field {name} is an array, which requires UNNEST() to be used in GROUP BY"
        )
        .into());
    }
    Ok(format!("could not resolve grouping column {name}"))
}

/// Returns true if the grouped output has a column that PostgreSQL does not
/// group on.
///
/// PostgreSQL does not group on a key that the WHERE clause pins to a constant
/// (`pathkey_is_redundant`), or on a key that the primary key or a unique
/// `NOT NULL` index decides (`remove_useless_groupby_columns`). The query can
/// also return a column that the primary key decides, and the planner takes
/// columns out of expressions that a node above computes. The Agg node reads
/// all of these from one row of the group. The Tantivy backend has no row to
/// read, so such a query goes to the DataFusion backend.
pub(super) unsafe fn has_ungrouped_column(args: &<AggregateScan as CustomScan>::Args) -> bool {
    let parse = (*args.root).parse;
    if parse.is_null() || (*parse).groupClause.is_null() {
        return false;
    }
    let keys: Vec<*mut pg_sys::Node> = args
        .group_by_pathkeys()
        .into_iter()
        .filter(|pathkey| !(**pathkey).pk_eclass.is_null())
        .flat_map(|pathkey| {
            PgList::<pg_sys::EquivalenceMember>::from_pg((*(*pathkey).pk_eclass).ec_members)
                .iter_ptr()
                .map(|member| strip_relabel((*member).em_expr.cast()))
                .collect::<Vec<_>>()
        })
        .collect();
    if keys.is_empty() {
        return true;
    }

    let reltarget = args.output_rel().reltarget;
    if reltarget.is_null() {
        return false;
    }
    PgList::<pg_sys::Node>::from_pg((*reltarget).exprs)
        .iter_ptr()
        .any(|expr| has_ungrouped_var(expr, &keys))
}

/// Returns true if `expr` reads a column outside of the GROUP BY keys and the
/// aggregates. A key can be a whole expression, so the walk stops at one.
unsafe fn has_ungrouped_var(expr: *mut pg_sys::Node, keys: &[*mut pg_sys::Node]) -> bool {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        let keys = &*(context as *const &[*mut pg_sys::Node]);
        let stripped = strip_relabel(node);
        if keys
            .iter()
            .any(|key| pg_sys::equal((*key).cast(), stripped.cast()))
        {
            return false;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_Aggref => false,
            pg_sys::NodeTag::T_Var => true,
            _ => pg_sys::expression_tree_walker(node, Some(walker), context),
        }
    }

    walker(
        expr,
        (&keys as *const &[*mut pg_sys::Node]) as *mut core::ffi::c_void,
    )
}
