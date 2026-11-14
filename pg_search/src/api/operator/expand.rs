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

use super::{
    SearchOperator, estimate_selectivity, pdb_query_typoid, rewrite_rhs_to_search_query_input,
    searchqueryinput_typoid,
};
use crate::api::FieldName;
use crate::nodecast;
use crate::postgres::access::IndexOid;
use crate::postgres::node::NodeExt;
use crate::postgres::planner_warnings::{add_planner_warning, with_nested_planning};
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::query::pdb_query::{pdb, synonym_index_of_column};
use pgrx::{FromDatum, IntoDatum, PgList, direct_function_call, pg_extern, pg_sys};
use std::ffi::CStr;

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

const MORE: &str = "it does more than read the rules of one table";

/// Replaces the `pdb.expand` subquery in `rhs` with a query on a synonym index of its table, for
/// a match on `field` of the text index `index`. `None` leaves the subquery to run as written.
pub(super) unsafe fn replace_expand_subquery(
    root: *mut pg_sys::PlannerInfo,
    rhs: *mut pg_sys::Node,
    index: &PgSearchRelation,
    field: &FieldName,
) -> Option<*mut pg_sys::Node> {
    // The replaced plan depends on the role, and only a root can record that.
    if root.is_null() {
        return None;
    }
    let replaced = replace_subquery(rhs, &|subquery| {
        if !calls_expand(subquery) {
            return None;
        }
        expand_call(subquery)
            .ok_or_else(|| MORE.to_owned())
            .and_then(|expand| synonym_index_query(root, subquery, expand, index, field))
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

/// The share of rows that the match selects with the rules that the `pdb.expand` subquery in
/// `rhs` returns now. The plan keeps the subquery, so a cached plan reads the rules of each run.
pub(super) unsafe fn selectivity_now(
    root: *mut pg_sys::PlannerInfo,
    operator: SearchOperator,
    lhs: *mut pg_sys::Node,
    rhs: *mut pg_sys::Node,
    index: &PgSearchRelation,
    field: &FieldName,
) -> Option<f64> {
    if root.is_null() {
        return None;
    }
    let rhs = replace_subquery(rhs, &|subquery| {
        expand_call(subquery)?;
        run_now(root, subquery)
    })?;
    let rhs = pg_sys::eval_const_expressions(root, rhs);
    nodecast!(Const, rhs)?;
    let input = rewrite_rhs_to_search_query_input(
        operator,
        searchqueryinput_typoid(),
        lhs,
        rhs,
        Some(field.clone()),
    );
    let input = nodecast!(Const, input)?;
    let input = SearchQueryInput::from_datum((*input).constvalue, (*input).constisnull)?;
    estimate_selectivity(index, input)
}

/// The run for the estimate reads and compiles every rule a second time, so it is only for a
/// rules table of at most this many blocks.
const MAX_BLOCKS_FOR_ESTIMATE: pg_sys::BlockNumber = 16;

/// Whether a run of the subquery alone, by the role, reads what the subquery reads in the plan.
unsafe fn runs_alone(subquery: *mut pg_sys::Query) -> bool {
    let query = &*subquery;
    let tables = PgList::<pg_sys::RangeTblEntry>::from_pg(query.rtable);
    let Some(table) = tables.get_ptr(0) else {
        return false;
    };
    if tables.len() != 1
        || query.hasSubLinks
        || (*table).rtekind != pg_sys::RTEKind::RTE_RELATION
        || pg_sys::get_rel_relkind((*table).relid) as u8 != pg_sys::RELKIND_RELATION
    {
        return false;
    }
    // A subquery in a view has the rights of the view owner, and the run has the rights of the
    // role.
    #[cfg(feature = "pg15")]
    let check_as_user = (*table).checkAsUser;
    #[cfg(not(feature = "pg15"))]
    let check_as_user = PgList::<pg_sys::RTEPermissionInfo>::from_pg(query.rteperminfos)
        .get_ptr(((*table).perminfoindex as usize).wrapping_sub(1))
        .map_or(pg_sys::Oid::INVALID, |rights| (*rights).checkAsUser);
    if check_as_user != pg_sys::Oid::INVALID && check_as_user != pg_sys::GetUserId() {
        return false;
    }
    // A column of the outer query has no value in a run of the subquery alone.
    let reads_outer_row = |node: *mut pg_sys::Node| {
        node.any(|node| nodecast!(Var, node).is_some_and(|column| (*column).varlevelsup > 0))
    };
    if reads_outer_row(query.targetList.cast()) || reads_outer_row(query.jointree.cast()) {
        return false;
    }
    let rules = PgSearchRelation::with_lock((*table).relid, pg_sys::AccessShareLock as _);
    pg_sys::RelationGetNumberOfBlocksInFork(rules.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM)
        <= MAX_BLOCKS_FOR_ESTIMATE
}

/// The value of the subquery under the snapshot of the planner, as a constant.
unsafe fn run_now(
    root: *mut pg_sys::PlannerInfo,
    subquery: *mut pg_sys::Query,
) -> Option<*mut pg_sys::Node> {
    // A volatile subquery can give other rules on its second run.
    if !pg_sys::ActiveSnapshotSet()
        || pg_sys::contain_volatile_functions(subquery.cast())
        || !runs_alone(subquery)
    {
        return None;
    }
    let sql = pg_sys::pg_get_querydef(pg_sys::copyObjectImpl(subquery.cast()).cast(), false);
    let params = (*(*root).glob).boundParams;
    // A generic plan has no value for a parameter.
    if params.is_null() && CStr::from_ptr(sql).to_bytes().contains(&b'$') {
        return None;
    }
    let value = with_nested_planning(|| {
        if pg_sys::SPI_connect() != pg_sys::SPI_OK_CONNECT as i32 {
            return None;
        }
        let mut options: pg_sys::SPIExecuteOptions = std::mem::zeroed();
        options.params = params;
        options.read_only = true;
        options.tcount = 2;
        let mut value = None;
        if pg_sys::SPI_execute_extended(sql, &options) == pg_sys::SPI_OK_SELECT as i32
            && pg_sys::SPI_processed == 1
        {
            let rows = pg_sys::SPI_tuptable;
            let mut is_null = false;
            let datum = pg_sys::SPI_getbinval(*(*rows).vals, (*rows).tupdesc, 1, &mut is_null);
            if !is_null {
                value = Some(pg_sys::SPI_datumTransfer(datum, false, -1));
            }
        }
        pg_sys::SPI_finish();
        value
    })?;
    Some(
        pg_sys::makeConst(
            pdb_query_typoid(),
            -1,
            pg_sys::Oid::INVALID,
            -1,
            value,
            false,
            false,
        )
        .cast(),
    )
}

/// A cast such as `::pdb.fuzzy(1)` is a function call around the subquery. Any other function
/// gets the value that the subquery returns.
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
    if (*call).funcformat == pg_sys::CoercionForm::COERCE_EXPLICIT_CALL {
        return None;
    }
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

/// The schema of `text` is written, because a type of that name can come first on the search
/// path.
unsafe fn is_expand(call: *mut pg_sys::Aggref) -> bool {
    direct_function_call::<pg_sys::Oid>(
        pg_sys::to_regprocedure,
        &["pdb.expand(pg_catalog.text, pg_catalog.text)".into_datum()],
    ) == Some((*call).aggfnoid)
}

/// A subquery that calls `pdb.expand` anywhere in its values is one that the user means to expand
/// with, so it gets the warning when it runs as written.
unsafe fn calls_expand(subquery: *mut pg_sys::Query) -> bool {
    (*subquery)
        .targetList
        .cast::<pg_sys::Node>()
        .any(|node| nodecast!(Aggref, node).is_some_and(|call| is_expand(call)))
}

/// The `pdb.expand` call of a subquery that returns nothing else.
unsafe fn expand_call(subquery: *mut pg_sys::Query) -> Option<*mut pg_sys::Aggref> {
    let targets = PgList::<pg_sys::TargetEntry>::from_pg((*subquery).targetList);
    let mut values = targets.iter_ptr().filter(|target| !(**target).resjunk);
    let (Some(value), None) = (values.next(), values.next()) else {
        return None;
    };
    nodecast!(Aggref, without_relabel((*value).expr.cast())).filter(|call| is_expand(*call))
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
        rules.column_index,
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

    // The scan opens the index with the rights of the role that runs it. That role is not the
    // role of the subquery in a view, and not the role of a cached plan after `SET ROLE`.
    if !IndexOid::from(synonym_index).is_readable() {
        return Err(format!(
            "the role can't read the rules of \"{}\" through an index",
            rules.table.name()
        ));
    }
    (*(*root).glob).dependsOnRole = true;

    let expand_with_index = direct_function_call::<pg_sys::Oid>(
        pg_sys::regprocedurein,
        &[c"paradedb.expand_with_index(pg_catalog.text, pg_catalog.regclass)".into_datum()],
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
    /// The position of the rule column in the table, from zero.
    column_index: usize,
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
    // The subquery is not planned yet. A root of its own makes a column in its expressions a
    // column of its table, and not of a table of the outer query.
    let mut subroot: pg_sys::PlannerInfo = std::mem::zeroed();
    subroot.type_ = pg_sys::NodeTag::T_PlannerInfo;
    subroot.glob = (*root).glob;
    subroot.parse = subquery;
    let root: *mut pg_sys::PlannerInfo = &mut subroot;

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
        && keeps_one_row(root, query.limitCount)
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
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION || !(*rte).tablesample.is_null() {
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
    match pg_sys::get_rel_relkind(table.oid()) as u8 {
        pg_sys::RELKIND_RELATION => {}
        pg_sys::RELKIND_PARTITIONED_TABLE => {
            return Err(format!("\"{name}\" is a partitioned table"));
        }
        _ => return Err(MORE.into()),
    }
    // Row security adds its quals here, and the index would read rows that they hide.
    if !(*rte).securityQuals.is_null() {
        return Err(format!("row security applies to \"{name}\""));
    }
    // `ONLY` reads the rows of the table itself, and those are the rows that the index holds.
    if (*rte).inh && !pg_sys::find_inheritance_children(table.oid(), pg_sys::NoLock as _).is_null()
    {
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
    if nodecast!(Const, value).is_some_and(|text| (*text).constisnull) {
        return Err("the text to expand is NULL".into());
    }
    let known = nodecast!(Const, value).is_some()
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
        column_index: (*rule).varattno as usize - 1,
        quals,
        text,
    })
}

/// An aggregate with no `GROUP BY` returns one row, so a `LIMIT` of one row or more changes
/// nothing. An ORM can add `LIMIT 1` to a scalar subquery.
unsafe fn keeps_one_row(root: *mut pg_sys::PlannerInfo, limit: *mut pg_sys::Node) -> bool {
    if limit.is_null() {
        return true;
    }
    let limit = pg_sys::eval_const_expressions(root, limit);
    nodecast!(Const, limit).is_some_and(|limit| {
        (*limit).constisnull
            || i64::from_datum((*limit).constvalue, false).is_some_and(|rows| rows >= 1)
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
