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

//! The relations a query applies a ParadeDB search operator to, as written.
//!
//! The planner can simplify such a predicate away before any scan sees the quals:
//! `A OR (A AND body @@@ 'q')` is just `A` to `process_duplicate_ors`, and the quals a scan is
//! then offered carry no trace of the operator. Recording the relations from the unsimplified
//! `Query` at planner-hook time lets a scan still tell that the query applied the operator to
//! its table.

use std::cell::RefCell;
use std::ffi::CStr;
use std::sync::OnceLock;

use pgrx::{IntoDatum, PgList, direct_function_call, pg_sys};

use crate::api::operator::{SearchPredicate, is_anyelement_search_opoid};
use crate::nodecast;
use crate::postgres::node::{NodeExt, WalkControl};

/// A relation as the query names it: its OID and the alias of its range table entry. The alias
/// keeps the two sides of a self-join apart.
type Relation = (pg_sys::Oid, String);

/// Nesting deeper than this is not a query anyone writes; it guards the walk against a cycle.
const MAX_QUERY_DEPTH: usize = 64;

thread_local! {
    /// One entry per planner invocation in progress, innermost last. The planner re-enters
    /// itself when it evaluates a SQL function while folding constants.
    static RELATIONS: RefCell<Vec<Vec<Relation>>> = const { RefCell::new(Vec::new()) };
}

/// Pops the entry [`capture`] pushed once the planner invocation ends, by error too.
pub struct CaptureGuard(());

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body only pops a thread
// local, and the entry has to go when the planner raises too.
impl Drop for CaptureGuard {
    fn drop(&mut self) {
        RELATIONS.with(|relations| {
            relations.borrow_mut().pop();
        });
    }
}

/// Records the relations `parse` applies a search operator to, for [`applies_to`] to consult
/// while the planner runs on `parse`. Keep the guard alive for that long.
pub unsafe fn capture(parse: *mut pg_sys::Query) -> CaptureGuard {
    RELATIONS.with(|relations| relations.borrow_mut().push(Vec::new()));
    let guard = CaptureGuard(());
    // Only a score or snippet can need the record, and most queries project neither.
    let Some(funcids) = projection_funcoids() else {
        return guard;
    };
    if !query_projects_score_or_snippet(parse, funcids, 0) {
        return guard;
    }
    let mut found = Vec::new();
    collect_from_query(parse, &mut Vec::new(), &mut found);
    RELATIONS.with(|relations| {
        if let Some(current) = relations.borrow_mut().last_mut() {
            *current = found;
        }
    });
    guard
}

/// Whether the query being planned applies a search operator to the relation `rti` scans, or
/// to a parent `rti` was expanded from.
pub unsafe fn applies_to(root: *mut pg_sys::PlannerInfo, rti: pg_sys::Index) -> bool {
    RELATIONS.with(|relations| {
        let relations = relations.borrow();
        let Some(current) = relations.last() else {
            return false;
        };
        if current.is_empty() {
            return false;
        }
        let mut rti = rti;
        loop {
            if rti == 0 || rti as i32 >= (*root).simple_rel_array_size {
                return false;
            }
            let rte = *(*root).simple_rte_array.add(rti as usize);
            if !rte.is_null()
                && (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION
                && current
                    .iter()
                    .any(|(relid, alias)| *relid == (*rte).relid && *alias == rte_alias(&*rte))
            {
                return true;
            }
            // Partitions and inheritance children scan under their own relid.
            let appinfos = (*root).append_rel_array;
            if appinfos.is_null() {
                return false;
            }
            let appinfo = *appinfos.add(rti as usize);
            if appinfo.is_null() {
                return false;
            }
            rti = (*appinfo).parent_relid;
        }
    })
}

/// The functions a ParadeDB scan computes for its rows: `score`, `snippet`, `snippets` and
/// `snippet_positions`, in both the `pdb` and `paradedb` schemas. `None` until the extension
/// has created all of them: the planner hook also sees the queries `CREATE EXTENSION` runs, so
/// the lookups here must not raise, unlike `score_funcoids()` and the snippet resolvers.
pub fn projection_funcoids() -> Option<&'static [pg_sys::Oid]> {
    const SIGNATURES: [&str; 8] = [
        "pdb.score(anyelement)",
        "paradedb.score(anyelement)",
        "pdb.snippet(anyelement, text, text, int, int, int)",
        "paradedb.snippet(anyelement, text, text, int, int, int)",
        "pdb.snippets(anyelement, text, text, int, int, int, text)",
        "paradedb.snippets(anyelement, text, text, int, int, int, text)",
        "pdb.snippet_positions(anyelement, int, int)",
        "paradedb.snippet_positions(anyelement, int, int)",
    ];
    static CACHE: OnceLock<Vec<pg_sys::Oid>> = OnceLock::new();
    if let Some(cached) = CACHE.get() {
        return Some(cached);
    }
    let resolved = SIGNATURES
        .iter()
        .map(|signature| unsafe {
            direct_function_call::<pg_sys::Oid>(pg_sys::to_regprocedure, &[signature.into_datum()])
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CACHE.get_or_init(|| resolved))
}

/// Whether any level of `query` calls one of `funcids`.
unsafe fn query_projects_score_or_snippet(
    query: *mut pg_sys::Query,
    funcids: &[pg_sys::Oid],
    depth: usize,
) -> bool {
    if query.is_null() || depth >= MAX_QUERY_DEPTH {
        return false;
    }
    let projects = |node: *mut pg_sys::Node| {
        node.any(|node| {
            if let Some(subquery) = nodecast!(Query, T_Query, node) {
                return query_projects_score_or_snippet(subquery, funcids, depth + 1);
            }
            nodecast!(FuncExpr, T_FuncExpr, node)
                .is_some_and(|expr| funcids.contains(&(*expr).funcid))
        })
    };
    projects((*query).targetList.cast())
        || projects((*query).jointree.cast())
        || projects((*query).havingQual)
        || PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable)
            .iter_ptr()
            .any(|rte| {
                (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY
                    && query_projects_score_or_snippet((*rte).subquery, funcids, depth + 1)
            })
        || PgList::<pg_sys::CommonTableExpr>::from_pg((*query).cteList)
            .iter_ptr()
            .any(|cte| query_projects_score_or_snippet((*cte).ctequery.cast(), funcids, depth + 1))
}

/// True if `node` contains a search predicate.
pub unsafe fn contains_search_predicate(node: *mut pg_sys::Node) -> bool {
    node.any(|node| search_predicate_operand(node).is_some())
}

/// The alias a range table entry goes by in the query.
pub fn rte_alias(rte: &pg_sys::RangeTblEntry) -> String {
    if rte.eref.is_null() || unsafe { (*rte.eref).aliasname.is_null() } {
        return String::new();
    }
    unsafe { CStr::from_ptr((*rte.eref).aliasname) }
        .to_string_lossy()
        .into_owned()
}

/// The left operand of `node` when `node` is a search predicate.
unsafe fn search_predicate_operand(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Node> {
    if let Some(predicate) = SearchPredicate::from_node(node) {
        return Some(predicate.lhs());
    }
    let (opno, args) = match (*node).type_ {
        pg_sys::NodeTag::T_OpExpr => {
            let expr = node.cast::<pg_sys::OpExpr>();
            ((*expr).opno, (*expr).args)
        }
        pg_sys::NodeTag::T_ScalarArrayOpExpr => {
            let expr = node.cast::<pg_sys::ScalarArrayOpExpr>();
            ((*expr).opno, (*expr).args)
        }
        _ => return None,
    };
    if !is_anyelement_search_opoid(opno) {
        return None;
    }
    PgList::<pg_sys::Node>::from_pg(args).get_ptr(0)
}

/// `enclosing` lists the queries around `query`, outermost first, for outer references.
unsafe fn collect_from_query(
    query: *mut pg_sys::Query,
    enclosing: &mut Vec<*mut pg_sys::Query>,
    found: &mut Vec<Relation>,
) {
    if query.is_null() || enclosing.len() >= MAX_QUERY_DEPTH {
        return;
    }
    enclosing.push(query);
    // WHERE and JOIN ... ON through the join tree, plus HAVING, which the planner moves to
    // WHERE when it carries no aggregate.
    collect_from_predicates((*query).jointree.cast(), enclosing, found);
    collect_from_predicates((*query).havingQual, enclosing, found);
    // Nested queries are planned in the same invocation, so their predicates count too.
    for rte in PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable).iter_ptr() {
        if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY {
            collect_from_query((*rte).subquery, enclosing, found);
        }
    }
    for cte in PgList::<pg_sys::CommonTableExpr>::from_pg((*query).cteList).iter_ptr() {
        collect_from_query((*cte).ctequery.cast(), enclosing, found);
    }
    collect_from_nested_queries((*query).targetList.cast(), enclosing, found);
    enclosing.pop();
}

unsafe fn collect_from_predicates(
    node: *mut pg_sys::Node,
    enclosing: &mut Vec<*mut pg_sys::Query>,
    found: &mut Vec<Relation>,
) {
    node.walk(|node| {
        if let Some(subquery) = nodecast!(Query, T_Query, node) {
            collect_from_query(subquery, enclosing, found);
            return WalkControl::SkipChildren;
        }
        if let Some(operand) = search_predicate_operand(node) {
            for var in operand.collect_nodes::<pg_sys::Var>() {
                collect_from_var(var, enclosing, found);
            }
        }
        WalkControl::Continue
    });
}

/// Only the queries nested in `node` count, not the operators in `node` itself: a search
/// operator in a SELECT list is a value, not a predicate the scan would run.
unsafe fn collect_from_nested_queries(
    node: *mut pg_sys::Node,
    enclosing: &mut Vec<*mut pg_sys::Query>,
    found: &mut Vec<Relation>,
) {
    node.walk(|node| {
        if let Some(subquery) = nodecast!(Query, T_Query, node) {
            collect_from_query(subquery, enclosing, found);
            return WalkControl::SkipChildren;
        }
        WalkControl::Continue
    });
}

/// Follows `var` through join aliases, subquery and CTE output columns, and outer references
/// to the relations behind it.
unsafe fn collect_from_var(
    var: *mut pg_sys::Var,
    enclosing: &[*mut pg_sys::Query],
    found: &mut Vec<Relation>,
) {
    let Some(level) = enclosing.len().checked_sub(1 + (*var).varlevelsup as usize) else {
        return;
    };
    let query = enclosing[level];
    let Ok(rti) = usize::try_from((*var).varno) else {
        return;
    };
    if rti == 0 {
        return;
    }
    let Some(rte) = PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable).get_ptr(rti - 1)
    else {
        return;
    };
    let attno = (*var).varattno;
    match (*rte).rtekind {
        pg_sys::RTEKind::RTE_RELATION => {
            let relation = ((*rte).relid, rte_alias(&*rte));
            if !found.contains(&relation) {
                found.push(relation);
            }
        }
        pg_sys::RTEKind::RTE_JOIN => {
            // Attribute 0 is the whole row.
            let aliases = PgList::<pg_sys::Node>::from_pg((*rte).joinaliasvars);
            for (i, alias) in aliases.iter_ptr().enumerate() {
                if attno == 0 || i + 1 == attno as usize {
                    for inner in alias.collect_nodes::<pg_sys::Var>() {
                        collect_from_var(inner, &enclosing[..=level], found);
                    }
                }
            }
        }
        pg_sys::RTEKind::RTE_SUBQUERY => {
            collect_from_output_column((*rte).subquery, attno, &enclosing[..=level], found)
        }
        pg_sys::RTEKind::RTE_CTE => {
            if (*rte).self_reference {
                return;
            }
            let Some(cte_level) = level.checked_sub((*rte).ctelevelsup as usize) else {
                return;
            };
            let name = CStr::from_ptr((*rte).ctename);
            let cte = PgList::<pg_sys::CommonTableExpr>::from_pg((*enclosing[cte_level]).cteList)
                .iter_ptr()
                .find(|cte| CStr::from_ptr((**cte).ctename) == name);
            if let Some(cte) = cte {
                collect_from_output_column(
                    (*cte).ctequery.cast(),
                    attno,
                    &enclosing[..=cte_level],
                    found,
                );
            }
        }
        _ => {}
    }
}

/// The relations behind output column `attno` of `subquery`, or behind every column for 0.
unsafe fn collect_from_output_column(
    subquery: *mut pg_sys::Query,
    attno: pg_sys::AttrNumber,
    enclosing: &[*mut pg_sys::Query],
    found: &mut Vec<Relation>,
) {
    if subquery.is_null() || enclosing.len() >= MAX_QUERY_DEPTH {
        return;
    }
    let mut enclosing = enclosing.to_vec();
    enclosing.push(subquery);
    for entry in PgList::<pg_sys::TargetEntry>::from_pg((*subquery).targetList).iter_ptr() {
        if (*entry).resjunk || (attno != 0 && (*entry).resno != attno) {
            continue;
        }
        for var in (*entry)
            .expr
            .cast::<pg_sys::Node>()
            .collect_nodes::<pg_sys::Var>()
        {
            collect_from_var(var, &enclosing, found);
        }
    }
}
