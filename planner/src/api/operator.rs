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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

pub use index_read::api::operator::*;

use crate::api::FieldName;
use crate::api::tokenizers::{
    AliasTypmod, try_get_alias, type_can_be_tokenized, type_is_alias, type_is_tokenizer,
};
use crate::nodecast;
use crate::postgres::composite::get_composite_type_fields;
use crate::postgres::customscan::opexpr::{
    UnwrapFromExpr, expr_matches_node, vars_equal_ignoring_varno,
};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::node::NodeExt;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::{VarContext, find_json_path, find_var_relation};
use pgrx::{PgList, pg_sys};

/// Given a [`pg_sys::PlannerInfo`] and a [`pg_sys::Node`] from it, figure out the name of the `Node`.
///
/// Returns the heap relation [`pg_sys::Oid`] that contains the `Node` along with its name.
pub unsafe fn tantivy_field_name_from_node(
    root: *mut pg_sys::PlannerInfo,
    node: *mut pg_sys::Node,
) -> Option<(PgSearchRelation, Option<FieldName>)> {
    let (heaprelid, _, targetlist) = find_node_relation(node, root);
    if heaprelid == pg_sys::Oid::INVALID {
        return None;
    }
    let (heaprel, indexrel) = rel_get_bm25_index(heaprelid).unwrap_or_else(|| {
        panic!(
            "`{}` does not contain a `USING paradedb` index",
            PgSearchRelation::open(heaprelid).name()
        )
    });

    let (field_node, context) = if targetlist.is_some() {
        let node = pg_sys::copyObjectImpl(node.cast()).cast::<pg_sys::Node>();
        for var in node.collect_nodes::<pg_sys::Var>() {
            let (relation, attribute, _) = find_var_relation(var, root);
            if relation != heaprelid {
                return None;
            }
            (*var).varattno = attribute;
            (*var).varattnosyn = attribute;
        }
        (node, VarContext::from_exec(heaprelid))
    } else {
        (node, VarContext::from_planner(root))
    };
    let field_name = field_name_from_node(context, &heaprel, &indexrel, field_node);
    if field_name.is_none() {
        let var = nodecast!(Var, node)?;
        if (*var).varattno != 0 {
            return None;
        }
    }
    Some((indexrel, field_name))
}

unsafe fn var_matches_tokenizer_expr(var: *const pg_sys::Var, expr: *mut pg_sys::Expr) -> bool {
    if !type_is_tokenizer(pg_sys::exprType(expr.cast())) {
        return false;
    }
    if !type_can_be_tokenized((*var).vartype) {
        return false;
    }
    let vars = expr.collect_nodes::<pg_sys::Var>();
    if vars.len() != 1 {
        return false;
    }
    let expr_var = vars[0];
    vars_equal_ignoring_varno(expr_var, var)
}

pub unsafe fn field_name_from_node(
    context: VarContext,
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    node: *mut pg_sys::Node,
) -> Option<FieldName> {
    if let Some(relabel) = nodecast!(RelabelType, node)
        && type_is_alias((*relabel).resulttype)
    {
        let typmod = AliasTypmod::try_from((*relabel).resulttypmod).unwrap_or_else(|e| e.report());
        if let Some(alias) = typmod.alias() {
            return Some(FieldName::from(alias));
        }
    }

    let index_info = unsafe { *indexrel.index_info() };
    if let Some(var) = nodecast!(Var, node) {
        if (*var).varattno == 0 {
            return None;
        }

        let expressions = indexrel.index_expressions();
        let mut candidate_field_names: Vec<FieldName> = vec![];

        let mut expr_no = 0;
        for i in 0..index_info.ii_NumIndexAttrs {
            let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];

            if heap_attno == (*var).varattno {
                return attname_from_var(heaprel, var);
            } else if heap_attno == 0 {
                let Some(expression) = expressions.get_ptr(expr_no) else {
                    panic!("expected expression for index attribute {expr_no}");
                };

                let expr_type = pg_sys::exprType(expression.cast());
                let is_composite = crate::postgres::composite::is_composite_type(expr_type);

                if is_composite {
                    if let Some(row_expr) = <*mut pg_sys::RowExpr>::unwrap_from_coercion(expression)
                    {
                        let composite_oid = pg_sys::exprType(expression.cast());
                        let Ok(fields) = get_composite_type_fields(composite_oid) else {
                            expr_no += 1;
                            continue;
                        };

                        let row_args = PgList::<pg_sys::Node>::from_pg((*row_expr).args);

                        for (position, arg) in row_args.iter_ptr().enumerate() {
                            if position >= fields.len() || fields[position].is_dropped {
                                continue;
                            }

                            if let Some(arg_var) =
                                <*mut pg_sys::Var>::unwrap_from_coercion(arg.cast())
                            {
                                if vars_equal_ignoring_varno(arg_var, var) {
                                    return Some(FieldName::from(
                                        fields[position].field_name.clone(),
                                    ));
                                }
                                continue;
                            }

                            if var_matches_tokenizer_expr(var, arg.cast()) {
                                candidate_field_names
                                    .push(FieldName::from(fields[position].field_name.clone()));
                            }
                        }
                    }
                    expr_no += 1;
                } else if type_is_tokenizer(expr_type) {
                    if type_can_be_tokenized((*var).vartype)
                        && var_matches_tokenizer_expr(var, expression.cast())
                    {
                        let oid = pg_sys::exprType(expression.cast());
                        let typmod = pg_sys::exprTypmod(expression.cast());
                        if let Some(field_name) = try_get_alias(oid, typmod)
                            .map(FieldName::from)
                            .or_else(|| attname_from_var(heaprel, var))
                        {
                            candidate_field_names.push(field_name);
                        }
                    }
                    expr_no += 1;
                }
            }
        }

        match &candidate_field_names[..] {
            [] => {
                return None;
            }
            [field_name] => {
                return Some(field_name.clone());
            }
            _ => {
                let column_field_name = attname_from_var(heaprel, var);
                if let Some(field_name) = column_field_name.as_ref().and_then(|column_field_name| {
                    candidate_field_names
                        .iter()
                        .find(|field_name| *field_name == column_field_name)
                        .cloned()
                }) {
                    return Some(field_name);
                }

                let mut names: Vec<String> = candidate_field_names
                    .into_iter()
                    .map(|f| format!("`{}`", f))
                    .collect();
                names.sort();

                let column_name = column_field_name
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "<unknown>".to_string());

                panic!(
                    "Query is ambiguous: column `{}` matches multiple indexed fields: {}. Use `{}::pdb.alias(...)` to choose one",
                    column_name,
                    names.join(", "),
                    column_name
                );
            }
        }
    }

    let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
    let mut expressions_iter = expressions.iter_ptr();

    for i in 0..index_info.ii_NumIndexAttrs {
        let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];
        if heap_attno == 0 {
            let Some(indexed_expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {i}.");
            };

            let expr_type = unsafe { pg_sys::exprType(indexed_expression.cast()) };
            let is_composite = crate::postgres::composite::is_composite_type(expr_type);

            if is_composite {
                if let Some(row_expr) =
                    <*mut pg_sys::RowExpr>::unwrap_from_coercion(indexed_expression)
                {
                    let composite_oid = unsafe { pg_sys::exprType(indexed_expression.cast()) };
                    let Ok(fields) = get_composite_type_fields(composite_oid) else {
                        continue;
                    };

                    let row_args = unsafe { PgList::<pg_sys::Node>::from_pg((*row_expr).args) };

                    for (position, arg) in row_args.iter_ptr().enumerate() {
                        if position >= fields.len() || fields[position].is_dropped {
                            continue;
                        }

                        if expr_matches_node(node, arg.cast(), type_is_alias) {
                            return Some(FieldName::from(fields[position].field_name.clone()));
                        }
                    }
                }
            } else if expr_matches_node(node, indexed_expression, type_is_alias) {
                let field_name =
                    if type_is_tokenizer(unsafe { pg_sys::exprType(indexed_expression.cast()) }) {
                        let oid = unsafe { pg_sys::exprType(indexed_expression.cast()) };
                        let typmod = unsafe { pg_sys::exprTypmod(indexed_expression.cast()) };
                        try_get_alias(oid, typmod).map(FieldName::from).or_else(|| {
                            indexed_expression
                                .find_single_node::<pg_sys::Var>()
                                .and_then(|var| attname_from_var(heaprel, var.cast()))
                        })
                    } else {
                        let expr_str = deparse_expr(None, heaprel, indexed_expression.cast());
                        panic!(
                            "indexed expression requires a tokenizer cast with an alias: {expr_str}"
                        );
                    };

                return field_name;
            }
        }
    }

    let json_path = find_json_path(&context, node);
    if json_path.len() > 1 {
        return Some(FieldName::from(json_path.join(".")));
    }

    None
}

pub unsafe fn find_node_relation(
    node: *mut pg_sys::Node,
    root: *mut pg_sys::PlannerInfo,
) -> (
    pg_sys::Oid,
    pg_sys::AttrNumber,
    Option<PgList<pg_sys::TargetEntry>>,
) {
    let var = node.collect_nodes::<pg_sys::Var>();
    if var.is_empty() {
        panic!("cannot determine relation: node does not contain a Var");
    }

    find_var_relation(var[0], root)
}
