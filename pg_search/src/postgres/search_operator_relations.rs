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

//! The range table entries a query applies a ParadeDB search operator to, as written.
//!
//! The planner can simplify such a predicate away before any scan sees the quals:
//! `A OR (A AND body @@@ 'q')` is just `A` to `process_duplicate_ors`, and the quals a scan is
//! then offered carry no trace of the operator. The record taken from the unsimplified `Query`
//! at planner-hook time is what still tells a scan that the query asked for it.
//!
//! The planner copies range table entries when it pulls a subquery up or plans one on its own,
//! so neither a pointer nor `(relid, alias)` identifies an entry across planning: the same table
//! under its default alias in a sibling subquery would match too. The parser locations of the
//! `Var`s that reference an entry survive every copy, so they are the identity used here.
//!
//! Only a direct reference to a table counts. An operator applied to a subquery's, a CTE's or a
//! join's column is left to the planner, which evaluates it on that reference.

use std::cell::RefCell;
use std::ffi::CStr;

use pgrx::{PgList, pg_sys};

use crate::api::operator::{SearchPredicate, is_paradedb_search_operator};
use crate::nodecast;
use crate::postgres::node::{NodeExt, WalkControl};

/// A range table entry the query applies a search operator to.
struct SearchedRelation {
    relid: pg_sys::Oid,
    /// The alias keeps the two sides of a self-join apart.
    alias: Vec<u8>,
    /// Parser locations of the `Var`s that reference the entry.
    var_locations: Vec<i32>,
}

/// Nesting deeper than this is not a query anyone writes; it guards the walk against a cycle.
const MAX_QUERY_DEPTH: usize = 64;

thread_local! {
    /// One record per planner invocation in progress, innermost last. The planner re-enters
    /// itself when it evaluates a SQL function while folding constants.
    static RECORDS: RefCell<Vec<Vec<SearchedRelation>>> = const { RefCell::new(Vec::new()) };
}

/// Runs `plan` with the record for `parse` in place, for [`applies_to`] to consult.
pub unsafe fn record_during<T>(parse: *mut pg_sys::Query, plan: impl FnOnce() -> T) -> T {
    RECORDS.with(|records| records.borrow_mut().push(searched_relations_in(parse)));
    let _scope = RecordScope(());
    plan()
}

/// Pops the record once planning ends, by error too.
struct RecordScope(());

// NOTE: We intentionally do NOT use `impl_safe_drop!` here because the body only pops a thread
// local, and the record has to go when the planner raises too.
impl Drop for RecordScope {
    fn drop(&mut self) {
        RECORDS.with(|records| {
            records.borrow_mut().pop();
        });
    }
}

/// Whether the query being planned applies a search operator to the relation scanned as `rti`,
/// or to a parent it was expanded from.
pub unsafe fn applies_to(root: *mut pg_sys::PlannerInfo, rti: pg_sys::Index) -> bool {
    RECORDS.with(|records| {
        let records = records.borrow();
        let Some(record) = records.last() else {
            return false;
        };
        if record.is_empty() {
            return false;
        }
        // Partitions and inheritance children scan under their own relid and alias, and the
        // planner rebuilds their `Var`s without a location. The record names the parent, and
        // the parent's own relation still carries the `Var`s as written.
        let Some((rti, rte)) = top_parent(root, rti) else {
            return false;
        };
        let rel = *(*root).simple_rel_array.add(rti as usize);
        if rel.is_null() {
            return false;
        }
        let alias = rte_alias_bytes(&*rte);
        let candidates: Vec<&SearchedRelation> = record
            .iter()
            .filter(|searched| searched.relid == (*rte).relid && searched.alias == alias)
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let references_entry = |node: *mut pg_sys::Node| {
            node.any(|node| {
                nodecast!(Var, T_Var, node).is_some_and(|var| {
                    (*var).varno as pg_sys::Index == rti
                        && (*var).location >= 0
                        && candidates
                            .iter()
                            .any(|searched| searched.var_locations.contains(&(*var).location))
                })
            })
        };
        (!(*rel).reltarget.is_null() && references_entry((*(*rel).reltarget).exprs.cast()))
            || PgList::<pg_sys::RestrictInfo>::from_pg((*rel).baserestrictinfo)
                .iter_ptr()
                .chain(PgList::<pg_sys::RestrictInfo>::from_pg((*rel).joininfo).iter_ptr())
                .any(|ri| references_entry((*ri).clause.cast()))
    })
}

/// True if `node` contains a search predicate. At plan time the operator may already be in its
/// function form, which the support function rewrites it into for heap evaluation.
pub unsafe fn contains_search_predicate(node: *mut pg_sys::Node) -> bool {
    node.any(|node| {
        SearchPredicate::from_node(node).is_some() || search_predicate_operand(node).is_some()
    })
}

/// The alias a range table entry goes by in the query.
pub fn rte_alias(rte: &pg_sys::RangeTblEntry) -> String {
    String::from_utf8_lossy(rte_alias_bytes(rte)).into_owned()
}

fn rte_alias_bytes(rte: &pg_sys::RangeTblEntry) -> &[u8] {
    if rte.eref.is_null() || unsafe { (*rte.eref).aliasname.is_null() } {
        return &[];
    }
    unsafe { CStr::from_ptr((*rte.eref).aliasname) }.to_bytes()
}

/// The entry `rti` scans, or the top parent it was expanded from, when that is a relation.
unsafe fn top_parent(
    root: *mut pg_sys::PlannerInfo,
    mut rti: pg_sys::Index,
) -> Option<(pg_sys::Index, *mut pg_sys::RangeTblEntry)> {
    loop {
        if rti == 0 || rti as i32 >= (*root).simple_rel_array_size {
            return None;
        }
        let appinfos = (*root).append_rel_array;
        let appinfo = if appinfos.is_null() {
            std::ptr::null_mut()
        } else {
            *appinfos.add(rti as usize)
        };
        if appinfo.is_null() {
            let rte = *(*root).simple_rte_array.add(rti as usize);
            return (!rte.is_null() && (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION)
                .then_some((rti, rte));
        }
        rti = (*appinfo).parent_relid;
    }
}

/// The left operand of `node` when `node` is a search predicate as a query writes it.
///
/// The operator is recognized by name through the syscache, which stays quiet when it does not
/// exist yet: the planner hook also sees the queries `CREATE EXTENSION` runs. Built-in operators
/// never qualify, so they skip the lookup. The function forms of the operator are planner
/// output and never appear in a query as written.
unsafe fn search_predicate_operand(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Node> {
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
    if opno.to_u32() < pg_sys::FirstNormalObjectId || !is_paradedb_search_operator(opno) {
        return None;
    }
    PgList::<pg_sys::Node>::from_pg(args).get_ptr(0)
}

/// The first pass is a predicate walk that compares operator OIDs, and it ends things for most
/// queries. Only a query that applies a search operator pays for the second pass, which
/// collects the `Var`s of the searched entries.
unsafe fn searched_relations_in(parse: *mut pg_sys::Query) -> Vec<SearchedRelation> {
    let mut found = Vec::new();
    if query_applies_search_operator(parse, 0) {
        collect_searched_relations(parse, 0, &mut found);
    }
    found
}

/// Whether a predicate of `query`, or of a query nested in it, carries a search operator.
unsafe fn query_applies_search_operator(query: *mut pg_sys::Query, depth: usize) -> bool {
    if query.is_null() || depth >= MAX_QUERY_DEPTH {
        return false;
    }
    // A `SubLink` hands its `subselect`, a `Query`, to the walker. That is the only way a
    // `Query` shows up inside an expression.
    let predicates = |node: *mut pg_sys::Node| {
        node.any(|node| {
            if let Some(subquery) = nodecast!(Query, T_Query, node) {
                return query_applies_search_operator(subquery, depth + 1);
            }
            search_predicate_operand(node).is_some()
        })
    };
    let nested_queries = |node: *mut pg_sys::Node| {
        node.any(|node| {
            nodecast!(Query, T_Query, node)
                .is_some_and(|subquery| query_applies_search_operator(subquery, depth + 1))
        })
    };
    predicates((*query).jointree.cast())
        || predicates((*query).havingQual)
        || nested_queries((*query).targetList.cast())
        || PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable)
            .iter_ptr()
            .any(|rte| {
                (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY
                    && query_applies_search_operator((*rte).subquery, depth + 1)
            })
        || PgList::<pg_sys::CommonTableExpr>::from_pg((*query).cteList)
            .iter_ptr()
            .any(|cte| query_applies_search_operator((*cte).ctequery.cast(), depth + 1))
}

/// What one query level knows about one of its range table entries.
#[derive(Default)]
struct Entry {
    searched: bool,
    var_locations: Vec<i32>,
}

/// Collects the searched entries of `query` and of the queries nested in it. Each level only
/// looks at its own `Var`s: an outer reference from a nested query is a correlation, not a
/// predicate on the outer entry.
unsafe fn collect_searched_relations(
    query: *mut pg_sys::Query,
    depth: usize,
    found: &mut Vec<SearchedRelation>,
) {
    if query.is_null() || depth >= MAX_QUERY_DEPTH {
        return;
    }
    let rtable = PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable);
    let mut entries: Vec<Entry> = (0..=rtable.len()).map(|_| Entry::default()).collect();
    let entry_count = entries.len();
    let own_var = move |var: *mut pg_sys::Var| -> Option<usize> {
        let rti = usize::try_from((*var).varno).ok()?;
        ((*var).varlevelsup == 0 && rti > 0 && rti < entry_count).then_some(rti)
    };
    // The SELECT list only references entries. A search operator there is a value, not a
    // predicate a scan would run. A `SubLink` hands its `subselect`, a `Query`, to the walker,
    // and that nested query gets a level of its own.
    let mut visit = |node: *mut pg_sys::Node, predicates: bool| {
        node.walk(|node| {
            if let Some(subquery) = nodecast!(Query, T_Query, node) {
                collect_searched_relations(subquery, depth + 1, found);
                return WalkControl::SkipChildren;
            }
            if predicates && let Some(operand) = search_predicate_operand(node) {
                for var in operand.collect_nodes::<pg_sys::Var>() {
                    if let Some(rti) = own_var(var) {
                        entries[rti].searched = true;
                    }
                }
            }
            if let Some(var) = nodecast!(Var, T_Var, node)
                && let Some(rti) = own_var(var)
                && (*var).location >= 0
                && !entries[rti].var_locations.contains(&(*var).location)
            {
                entries[rti].var_locations.push((*var).location);
            }
            WalkControl::Continue
        });
    };
    // WHERE and JOIN ... ON through the join tree, plus HAVING, which the planner moves to
    // WHERE when it carries no aggregate.
    visit((*query).jointree.cast(), true);
    visit((*query).havingQual, true);
    visit((*query).targetList.cast(), false);
    for (rti, rte) in rtable.iter_ptr().enumerate().map(|(i, rte)| (i + 1, rte)) {
        match (*rte).rtekind {
            pg_sys::RTEKind::RTE_RELATION if entries[rti].searched => {
                found.push(SearchedRelation {
                    relid: (*rte).relid,
                    alias: rte_alias_bytes(&*rte).to_vec(),
                    var_locations: std::mem::take(&mut entries[rti].var_locations),
                });
            }
            pg_sys::RTEKind::RTE_SUBQUERY => {
                collect_searched_relations((*rte).subquery, depth + 1, found);
            }
            _ => {}
        }
    }
    for cte in PgList::<pg_sys::CommonTableExpr>::from_pg((*query).cteList).iter_ptr() {
        collect_searched_relations((*cte).ctequery.cast(), depth + 1, found);
    }
}
