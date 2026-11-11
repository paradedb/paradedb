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

//! A `pdb.expand` subquery that reads a whole rules table gives the same expansion as the synonym
//! index of that table, so the planner replaces it with the index and no query reads every rule.

use super::pdb_query_typoid;
use crate::api::FieldName;
use crate::nodecast;
use crate::postgres::rel::PgSearchRelation;
use crate::query::pdb_query::synonym_index_of_column;
use pgrx::{IntoDatum, PgList, direct_function_call, pg_sys};

/// Replaces the `pdb.expand` subquery in `rhs` with `pdb.with_synonyms` on a synonym index, for a
/// query on `field` of the text index `index`. `None` leaves the subquery to run as written.
pub(super) unsafe fn read_synonym_index(
    root: *mut pg_sys::PlannerInfo,
    rhs: *mut pg_sys::Node,
    index: &PgSearchRelation,
    field: &FieldName,
) -> Option<*mut pg_sys::Node> {
    let replaced = replace_subquery(rhs, &|subquery| with_synonyms(subquery, index, field))?;
    // `pdb.with_synonyms` is immutable, so a constant text becomes a constant query here.
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

/// `pdb.with_synonyms(text, index)` for a subquery of the form
/// `SELECT pdb.expand(text, rule) FROM rules`.
unsafe fn with_synonyms(
    subquery: *mut pg_sys::Query,
    index: &PgSearchRelation,
    field: &FieldName,
) -> Option<*mut pg_sys::Node> {
    let (table, attno, text) = whole_table_expand(subquery)?;
    let synonym_index = synonym_index_of_column(&table, attno, index, field)?;

    let with_synonyms = direct_function_call::<pg_sys::Oid>(
        pg_sys::to_regprocedure,
        &["pdb.with_synonyms(text, regclass)".into_datum()],
    )?;
    let mut args = PgList::<pg_sys::Node>::new();
    args.push(text);
    args.push(
        pg_sys::makeConst(
            pg_sys::REGCLASSOID,
            -1,
            pg_sys::Oid::INVALID,
            size_of::<pg_sys::Oid>() as _,
            synonym_index.into_datum()?,
            false,
            true,
        )
        .cast(),
    );
    Some(
        pg_sys::makeFuncExpr(
            with_synonyms,
            pdb_query_typoid(),
            args.into_pg(),
            pg_sys::Oid::INVALID,
            pg_sys::Oid::INVALID,
            pg_sys::CoercionForm::COERCE_EXPLICIT_CALL,
        )
        .cast(),
    )
}

/// The rules table, the rule column (counted from zero) and the query text of a subquery that
/// does nothing but `pdb.expand` over every row of one table.
unsafe fn whole_table_expand(
    subquery: *mut pg_sys::Query,
) -> Option<(PgSearchRelation, usize, *mut pg_sys::Node)> {
    let query = &*subquery;
    let only_aggregates = query.commandType == pg_sys::CmdType::CMD_SELECT
        && query.hasAggs
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
        && query.setOperations.is_null();
    if !only_aggregates || query.jointree.is_null() || !(*query.jointree).quals.is_null() {
        return None;
    }

    let from = PgList::<pg_sys::Node>::from_pg((*query.jointree).fromlist);
    if from.len() != 1 {
        return None;
    }
    let rtindex = (*nodecast!(RangeTblRef, from.get_ptr(0)?)?).rtindex;
    let rte =
        PgList::<pg_sys::RangeTblEntry>::from_pg(query.rtable).get_ptr(rtindex as usize - 1)?;
    // Row security adds its quals here, and the index would read rows that they hide.
    if (*rte).rtekind != pg_sys::RTEKind::RTE_RELATION
        || (*rte).relkind as u8 != pg_sys::RELKIND_RELATION
        || !(*rte).tablesample.is_null()
        || !(*rte).securityQuals.is_null()
    {
        return None;
    }

    let targets = PgList::<pg_sys::TargetEntry>::from_pg(query.targetList);
    if targets.len() != 1 {
        return None;
    }
    let aggref = nodecast!(Aggref, (*targets.get_ptr(0)?).expr)?;
    let expand = direct_function_call::<pg_sys::Oid>(
        pg_sys::to_regprocedure,
        &["pdb.expand(text, text)".into_datum()],
    )?;
    if (*aggref).aggfnoid != expand
        || (*aggref).agglevelsup != 0
        || !(*aggref).aggfilter.is_null()
        || !(*aggref).aggorder.is_null()
        || !(*aggref).aggdistinct.is_null()
    {
        return None;
    }
    let args = PgList::<pg_sys::TargetEntry>::from_pg((*aggref).args);
    let text = (*args.get_ptr(0)?).expr.cast::<pg_sys::Node>();
    let rule = nodecast!(Var, without_relabel((*args.get_ptr(1)?).expr.cast()))?;
    if (*rule).varno != rtindex || (*rule).varlevelsup != 0 || (*rule).varattno <= 0 {
        return None;
    }
    // The planner evaluates the text outside the subquery, so it can't read an outer column.
    let is_constant = match without_relabel(text) {
        text if nodecast!(Const, text).is_some() => true,
        text => nodecast!(Param, text)
            .is_some_and(|param| (*param).paramkind == pg_sys::ParamKind::PARAM_EXTERN),
    };
    if !is_constant {
        return None;
    }

    let table = PgSearchRelation::with_lock((*rte).relid, pg_sys::AccessShareLock as _);
    // A child table has rules that the index lacks. `pdb.with_synonyms` needs `SELECT` on the
    // whole table, and a role with a column grant reads the rules through the subquery.
    if (*table.rd_rel).relhassubclass
        || pg_sys::pg_class_aclcheck(table.oid(), pg_sys::GetUserId(), pg_sys::ACL_SELECT as _)
            != pg_sys::AclResult::ACLCHECK_OK
    {
        return None;
    }
    Some((table, (*rule).varattno as usize - 1, text))
}

unsafe fn without_relabel(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    while let Some(relabel) = nodecast!(RelabelType, node) {
        node = (*relabel).arg.cast();
    }
    node
}
