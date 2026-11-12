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
#![allow(unsafe_op_in_unsafe_fn)]

//! A `pdb.expand` subquery that reads the rules of a synonym index gives the same expansion as
//! that index, so the planner replaces it with the index and no query reads every rule.

use super::pdb_query_typoid;
use crate::api::FieldName;
use crate::nodecast;
use crate::postgres::access::IndexOid;
use crate::postgres::node::NodeExt;
use crate::postgres::planner_warnings::add_planner_warning;
use crate::postgres::rel::PgSearchRelation;
use crate::query::pdb_query::{pdb, synonym_index_of_column};
use pgrx::{IntoDatum, PgList, direct_function_call, pg_extern, pg_sys};

/// The planner calls this function in place of a `pdb.expand` subquery that a synonym index
/// holds. The text of a prepared statement can be a parameter, which has a value only at run time.
#[pg_extern(immutable, parallel_safe, strict)]
pub fn expand_with_index(query: String, index: IndexOid) -> pdb::Query {
    pdb::Query::UnclassifiedString {
        string: query,
        fuzzy_data: None,
        slop_data: None,
        synonyms: Some(pdb::SynonymRules::Index(index.oid())),
    }
}

/// Replaces the `pdb.expand` subquery in `rhs` with a query on a synonym index of its table, for
/// a match on `field` of the text index `index`. `None` leaves the subquery to run as written.
pub(super) unsafe fn read_synonym_index(
    root: *mut pg_sys::PlannerInfo,
    rhs: *mut pg_sys::Node,
    index: &PgSearchRelation,
    field: &FieldName,
) -> Option<*mut pg_sys::Node> {
    let replaced = replace_subquery(rhs, &|subquery| {
        let expand = expand_call(subquery)?;
        synonym_index_query(root, subquery, expand, index, field)
            .map_err(|reason| {
                add_planner_warning(
                    "Synonyms",
                    format!(
                        "a `pdb.expand` subquery reads its rules from the table on every query, \
                         because {reason}. To disable this warning: \
                         SET paradedb.planner_warnings = 'off'"
                    ),
                    (),
                )
            })
            .ok()
    })?;
    // The call and a cast around it are immutable, so a constant text gives a constant query.
    Some(pg_sys::eval_const_expressions(root, replaced))
}

/// A cast such as `::pdb.fuzzy(1)` is a function call around the subquery.
unsafe fn replace_subquery(
    node: *mut pg_sys::Node,
    replace: &dyn Fn(*mut pg_sys::Query) -> Option<*mut pg_sys::Node>,
) -> Option<*mut pg_sys::Node> {
    if let Some(sublink) = nodecast!(SubLink, node) {
        if (*sublink).subLinkType != pg_sys::SubLinkType::EXPR_SUBLINK {
            return None;
        }
        return replace(nodecast!(Query, (*sublink).subselect)?);
    }
    let call = nodecast!(FuncExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*call).args);
    let mut replaced_args = PgList::<pg_sys::Node>::new();
    replaced_args.push(replace_subquery(args.get_ptr(0)?, replace)?);
    for arg in args.iter_ptr().skip(1) {
        replaced_args.push(arg);
    }
    let replaced = pg_sys::palloc(size_of::<pg_sys::FuncExpr>()).cast::<pg_sys::FuncExpr>();
    replaced.copy_from_nonoverlapping(call, 1);
    (*replaced).args = replaced_args.into_pg();
    Some(replaced.cast())
}

/// The `pdb.expand` call of a subquery that selects nothing else.
unsafe fn expand_call(subquery: *mut pg_sys::Query) -> Option<*mut pg_sys::Aggref> {
    let targets = PgList::<pg_sys::TargetEntry>::from_pg((*subquery).targetList);
    if targets.len() != 1 {
        return None;
    }
    let aggref = nodecast!(Aggref, (*targets.get_ptr(0)?).expr)?;
    let expand = direct_function_call::<pg_sys::Oid>(
        pg_sys::to_regprocedure,
        &["pdb.expand(text, text)".into_datum()],
    )?;
    ((*aggref).aggfnoid == expand).then_some(aggref)
}

/// The call of `expand_with_index` that reads a synonym index in place of the subquery, or the
/// reason why the subquery reads its rules itself.
unsafe fn synonym_index_query(
    root: *mut pg_sys::PlannerInfo,
    subquery: *mut pg_sys::Query,
    expand: *mut pg_sys::Aggref,
    index: &PgSearchRelation,
    field: &FieldName,
) -> Result<*mut pg_sys::Node, String> {
    let rules = rules_of_one_table(root, subquery, expand)?;
    let synonym_index = synonym_index_of_column(
        &rules.table,
        rules.attno,
        |candidate| holds_rows(candidate, rules.quals),
        index,
        field,
    )
    .ok_or_else(|| {
        let name = rules.table.name();
        // A generic plan has no value for the parameter, so no partial index can be chosen.
        if rules.quals.cast::<pg_sys::Node>().contains_param() {
            format!("the filter on \"{name}\" has a parameter with no value at plan time")
        } else {
            format!(
                "\"{name}\" has no synonym index for these rules with the tokenizer of field \
                 `{field}`"
            )
        }
    })?;

    let expand_with_index = direct_function_call::<pg_sys::Oid>(
        pg_sys::regprocedurein,
        &[c"paradedb.expand_with_index(text, regclass)".into_datum()],
    )
    .ok_or("`paradedb.expand_with_index` does not exist")?;
    let mut args = PgList::<pg_sys::Node>::new();
    args.push(rules.text);
    args.push(
        pg_sys::makeConst(
            pg_sys::REGCLASSOID,
            -1,
            pg_sys::Oid::INVALID,
            size_of::<pg_sys::Oid>() as _,
            synonym_index.into_datum().ok_or("the index has no oid")?,
            false,
            true,
        )
        .cast(),
    );
    Ok(pg_sys::makeFuncExpr(
        expand_with_index,
        pdb_query_typoid(),
        args.into_pg(),
        pg_sys::Oid::INVALID,
        pg_sys::Oid::INVALID,
        pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
    )
    .cast())
}

/// What a subquery of the form `SELECT pdb.expand('text', rule) FROM rules WHERE quals` reads.
struct TableRules {
    table: PgSearchRelation,
    /// The rule column, counted from zero.
    attno: usize,
    /// The `WHERE` as an implicit `AND` list.
    quals: *mut pg_sys::List,
    /// The text to expand, a constant or a parameter of the statement.
    text: *mut pg_sys::Node,
}

unsafe fn rules_of_one_table(
    root: *mut pg_sys::PlannerInfo,
    subquery: *mut pg_sys::Query,
    expand: *mut pg_sys::Aggref,
) -> Result<TableRules, String> {
    const MORE: &str = "it does more than read the rules of one table";

    let query = &*subquery;
    let only_rules = query.commandType == pg_sys::CmdType::CMD_SELECT
        && !query.hasWindowFuncs
        && !query.hasTargetSRFs
        && !query.hasSubLinks
        && !query.hasForUpdate
        && query.cteList.is_null()
        && query.groupClause.is_null()
        && query.groupingSets.is_null()
        && query.havingQual.is_null()
        && query.distinctClause.is_null()
        && query.sortClause.is_null()
        && query.limitOffset.is_null()
        && query.limitCount.is_null()
        && query.setOperations.is_null()
        && !query.jointree.is_null()
        && (*expand).agglevelsup == 0
        && (*expand).aggfilter.is_null()
        && (*expand).aggorder.is_null()
        && (*expand).aggdistinct.is_null();
    if !only_rules {
        return Err(MORE.into());
    }

    let from = PgList::<pg_sys::Node>::from_pg((*query.jointree).fromlist);
    let rtindex = match (from.len(), from.get_ptr(0)) {
        (1, Some(table)) => nodecast!(RangeTblRef, table).map(|table| (*table).rtindex),
        _ => None,
    };
    // The predicate of a partial index names its table as relation 1.
    if rtindex != Some(1) {
        return Err(MORE.into());
    }
    let rte = PgList::<pg_sys::RangeTblEntry>::from_pg(query.rtable)
        .get_ptr(0)
        .ok_or(MORE)?;
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION
        || pg_sys::get_rel_relkind((*rte).relid) as u8 != pg_sys::RELKIND_RELATION
        || !(*rte).tablesample.is_null()
    {
        return Err(MORE.into());
    }

    let args = PgList::<pg_sys::TargetEntry>::from_pg((*expand).args);
    let (Some(text), Some(rule)) = (args.get_ptr(0), args.get_ptr(1)) else {
        return Err(MORE.into());
    };
    let rule = nodecast!(Var, without_relabel((*rule).expr.cast())).ok_or(MORE)?;
    if (*rule).varno != 1 || (*rule).varlevelsup != 0 || (*rule).varattno <= 0 {
        return Err(MORE.into());
    }

    let table = PgSearchRelation::with_lock((*rte).relid, pg_sys::AccessShareLock as _);
    let name = table.name();
    // Row security adds its quals here, and the index would read rows that they hide.
    if !(*rte).securityQuals.is_null() {
        return Err(format!("row security applies to \"{name}\""));
    }
    if (*table.rd_rel).relhassubclass {
        return Err(format!("\"{name}\" has child tables"));
    }
    // The index is read with `SELECT` on the whole table. A role with a column grant reads the
    // rules through the subquery.
    if pg_sys::pg_class_aclcheck(table.oid(), pg_sys::GetUserId(), pg_sys::ACL_SELECT as _)
        != pg_sys::AclResult::ACLCHECK_OK
    {
        return Err(format!("the role has no `SELECT` on all of \"{name}\""));
    }

    // Any other text can read a row of the subquery or of the outer query.
    let text = pg_sys::eval_const_expressions(root, (*text).expr.cast());
    let value = without_relabel(text);
    let known = nodecast!(Const, value).is_some_and(|text| !(*text).constisnull)
        || nodecast!(Param, value)
            .is_some_and(|text| (*text).paramkind == pg_sys::ParamKind::PARAM_EXTERN);
    if !known {
        return Err("the text to expand is not a constant or a parameter".into());
    }

    let quals = (*query.jointree).quals;
    let quals = if quals.is_null() {
        std::ptr::null_mut()
    } else {
        let quals = pg_sys::eval_const_expressions(root, quals);
        pg_sys::make_ands_implicit(pg_sys::canonicalize_qual(quals.cast(), false))
    };
    Ok(TableRules {
        table,
        attno: (*rule).varattno as usize - 1,
        quals,
        text,
    })
}

/// A partial index holds the rows of its predicate, so it holds the rows of `quals` only when
/// each of the two implies the other. An index with no predicate holds them when `quals` is empty.
unsafe fn holds_rows(index: &PgSearchRelation, quals: *mut pg_sys::List) -> bool {
    let predicate = pg_sys::RelationGetIndexPredicate(index.as_ptr());
    pg_sys::predicate_implied_by(predicate, quals, false)
        && pg_sys::predicate_implied_by(quals, predicate, false)
}

unsafe fn without_relabel(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    while let Some(relabel) = nodecast!(RelabelType, node) {
        node = (*relabel).arg.cast();
    }
    node
}
