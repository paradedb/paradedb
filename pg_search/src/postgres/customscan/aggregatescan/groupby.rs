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
    AggregateScan, CustomScanBuildError, CustomScanClause, GroupingPushdownDeclineReason,
    can_decode_group_key_cast,
};
use crate::postgres::customscan::basescan::exec_methods::fast_fields::find_matching_fast_field;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::node::{NodeExt, WalkControl};
use crate::postgres::utils::{strip_relabel, strip_unnest_and_relabel};
use crate::postgres::var::{VarContext, find_one_var_and_fieldname, find_var_relation};
use crate::schema::{SearchFieldType, SearchIndexSchema};
use pgrx::pg_sys;
use pgrx::{PgList, pg_guard};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
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

/// Reject stripped casts that the backend's result decoder cannot evaluate.
pub(super) fn validate_grouping_casts(
    expr: *mut pg_sys::Node,
    stripped: *mut pg_sys::Node,
    supports_io_cast: impl Fn(pg_sys::Oid, pg_sys::Oid) -> bool,
) -> Result<(), String> {
    let expected_type = unsafe { pg_sys::exprType(expr) };
    let needs_evaluation = expr.walk(|node| unsafe {
        if node == stripped {
            return WalkControl::SkipChildren;
        }
        match (*node).type_ {
            pg_sys::NodeTag::T_CoerceToDomain => WalkControl::Break,
            pg_sys::NodeTag::T_CoerceViaIO => {
                let cast = &*node.cast::<pg_sys::CoerceViaIO>();
                if cast.resulttype == expected_type
                    && supports_io_cast(pg_sys::exprType(cast.arg.cast()), expected_type)
                {
                    WalkControl::Continue
                } else {
                    WalkControl::Break
                }
            }
            _ => WalkControl::Continue,
        }
    });
    if needs_evaluation {
        return Err("grouping cast requires PostgreSQL evaluation".into());
    }
    Ok(())
}

/// Validate a Tantivy grouping cast, using an indexed expression when required.
pub(super) fn resolve_grouping_cast(
    expr: *mut pg_sys::Node,
    stripped: *mut pg_sys::Node,
    index_expressions: &PgList<pg_sys::Expr>,
    schema: &SearchIndexSchema,
    heap_rti: pg_sys::Index,
) -> Result<Option<String>, String> {
    let cast_validation = validate_grouping_casts(expr, stripped, can_decode_group_key_cast);
    let indexed_field = if cast_validation.is_err() {
        find_matching_fast_field(expr, index_expressions, schema.clone(), heap_rti)
    } else {
        None
    };
    if let Some(field) = indexed_field {
        return Ok(Some(field.name()));
    }
    cast_validation?;
    Ok(None)
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
                let var_context = VarContext::from_planner(args.root);

                let indexed_field = match resolve_grouping_cast(
                    member.em_expr.cast(),
                    expr,
                    &index_expressions,
                    &schema,
                    _heap_rti,
                ) {
                    Ok(field) => field,
                    Err(error) => {
                        last_error = Some(error);
                        continue;
                    }
                };

                let (field_name, attno) = if let Some(field_name) = indexed_field {
                    (field_name, 0)
                } else if let Some((var, field_name)) =
                    unsafe { find_one_var_and_fieldname(var_context, expr) }
                {
                    // JSON operator expression or complex field access
                    let (heaprelid, attno, _) = unsafe { find_var_relation(var, args.root) };
                    if heaprelid == pg_sys::InvalidOid {
                        last_error =
                            Some("find_var_relation returned InvalidOid for var".to_string());
                        continue;
                    }
                    (field_name.to_string(), attno)
                } else if let Some(ff) =
                    find_matching_fast_field(expr, &index_expressions, schema.clone(), _heap_rti)
                {
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

                        // `group_key_to_datum` needs the indexed type to decode supported casts.
                        let original_type_oid = search_field.field_type().typeoid().value();
                        let string_null_sentinel = matches!(
                            search_field.field_type(),
                            SearchFieldType::Text(_)
                                | SearchFieldType::Tokenized(..)
                                | SearchFieldType::Bool(_)
                                | SearchFieldType::Json(_)
                        );
                        grouping_columns.push(GroupingColumn {
                            field_name,
                            attno,
                            original_type_oid,
                            string_null_sentinel,
                        });
                        found_valid_column = true;
                        break; // Found a valid grouping column for this pathkey
                    } else {
                        last_error = Some(format!(
                            "grouping column {} exists, but is not columnar",
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
