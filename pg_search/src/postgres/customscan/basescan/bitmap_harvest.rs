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

//! Finds a `BitmapHeapPath` over a non-BM25 index (btree, GIN, PostGIS GiST) whose
//! bitmapqual covers quals we would otherwise evaluate as a `heap_filter`. The path is
//! attached as a `custom_paths` child so its `BitmapIndexScan` can be executed at scan
//! time and its TIDBitmap intersected with the Tantivy result set (#5702).
//!
//! Scope: a single `BitmapIndexScan` or `BitmapAnd` tree over top-level AND clauses;
//! `BitmapOr` is rejected because a single OR arm is not a necessary condition of the
//! query, so rejecting rows absent from its bitmap would be unsound.
//!
//! Recheck invariant: a lossy `IndexClause` means the original predicate must always be
//! re-evaluated on bitmap survivors; a non-lossy original only needs re-evaluation when
//! the TIDBitmap page's recheck bit is set.

use crate::postgres::customscan::qual_inspect::Qual;
use crate::postgres::rel::PgSearchRelation;
use pgrx::{PgList, pg_sys};

/// Collect the expr nodes of every `Qual::HeapExpr` reachable through top-level AND
/// structure. Returns `false` if a HeapExpr appears under OR/NOT, where its bitmap
/// could not be used to reject rows.
pub fn collect_heap_expr_nodes(qual: &Qual, out: &mut Vec<*mut pg_sys::Node>) -> bool {
    fn contains_heap_expr(qual: &Qual) -> bool {
        match qual {
            Qual::HeapExpr { .. } => true,
            Qual::And(qs) | Qual::Or(qs) => qs.iter().any(contains_heap_expr),
            Qual::Not(q) => contains_heap_expr(q),
            _ => false,
        }
    }

    match qual {
        Qual::HeapExpr { expr_node, .. } => {
            out.push(*expr_node);
            true
        }
        Qual::And(qs) => qs.iter().all(|q| collect_heap_expr_nodes(q, out)),
        Qual::Or(qs) => !qs.iter().any(contains_heap_expr),
        Qual::Not(q) => !contains_heap_expr(q),
        _ => true,
    }
}

/// Find a `BitmapHeapPath` usable as an intersection source: AND-only bitmapqual, not
/// over the BM25 index itself, covering at least one of our HeapExpr clauses. Prefers a
/// planner-generated path from the rel's pathlists, then falls back to building one.
pub unsafe fn harvest(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    bm25_oid: pg_sys::Oid,
    heap_exprs: &[*mut pg_sys::Node],
) -> Option<*mut pg_sys::Path> {
    unsafe {
        let mut candidates = existing_bitmap_paths(rel);

        // A BitmapHeapPath only survives in the pathlist if it won add_path competition
        // (usually against seqscan), so absence does not mean no index applies.
        // Re-invoking create_index_paths is unreliable for the same reason -- its
        // internal add_path competition against the BM25 index's own path discards what
        // we need -- so build the path directly instead.
        if candidates.is_empty()
            && let Some(bhp) = build_bitmap_path(root, rel, bm25_oid, heap_exprs)
        {
            candidates.push(bhp);
        }

        candidates.into_iter().find_map(|bhp| {
            match classify(bhp, bm25_oid, heap_exprs) {
                Ok(clauses) => {
                    for clause in &clauses {
                        pgrx::notice!(
                            "[bitmap_harvest] index={} lossy={} matched_heap_expr={}",
                            clause.index_name,
                            clause.lossy,
                            clause.matched_heap_expr,
                        );
                    }
                    pgrx::notice!(
                        "[bitmap_harvest] harvested BitmapHeapPath: {} index clause(s), {} matched heap_filter clause(s), est rows={:.0}",
                        clauses.len(),
                        clauses.iter().filter(|c| c.matched_heap_expr).count(),
                        (*bhp).path.rows,
                    );
                    Some(bhp.cast::<pg_sys::Path>())
                }
                Err(rejection) => {
                    pgrx::notice!("[bitmap_harvest] skipping BitmapHeapPath: {rejection}");
                    None
                }
            }
        })
    }
}

/// Run the child `BitmapIndexScan` via `MultiExecProcNode` and report what its
/// TIDBitmap contains.
pub unsafe fn consume_and_report(child: *mut pg_sys::PlanState) {
    unsafe {
        let result = pg_sys::MultiExecProcNode(child);
        if result.is_null() {
            pgrx::notice!("[bitmap_harvest] MultiExecProcNode returned NULL");
            return;
        }
        assert_eq!(
            (*result).type_,
            pg_sys::NodeTag::T_TIDBitmap,
            "MultiExecProcNode must return a TIDBitmap"
        );
        let tbm: *mut pg_sys::TIDBitmap = result.cast();
        let summary = TidBitmapSummary::collect(tbm);
        pgrx::notice!(
            "[bitmap_harvest] TIDBitmap: {} pages ({} exact tuples, {} lossy pages, {} recheck pages)",
            summary.pages,
            summary.exact_tuples,
            summary.lossy_pages,
            summary.recheck_pages,
        );
        pg_sys::tbm_free(tbm);
    }
}

struct HarvestedClause {
    index_name: String,
    lossy: bool,
    matched_heap_expr: bool,
}

enum Rejection {
    ContainsBitmapOr,
    UsesBm25Index,
    NoMatchedClause,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Rejection::ContainsBitmapOr => "bitmapqual contains BitmapOr",
            Rejection::UsesBm25Index => "bitmapqual is over the BM25 index itself",
            Rejection::NoMatchedClause => "covers no heap_filter clause",
        })
    }
}

unsafe fn existing_bitmap_paths(rel: *mut pg_sys::RelOptInfo) -> Vec<*mut pg_sys::BitmapHeapPath> {
    unsafe {
        PgList::<pg_sys::Path>::from_pg((*rel).pathlist)
            .iter_ptr()
            .chain(PgList::<pg_sys::Path>::from_pg((*rel).partial_pathlist).iter_ptr())
            .filter(|path| (**path).type_ == pg_sys::NodeTag::T_BitmapHeapPath)
            .map(|path| path.cast())
            .collect()
    }
}

/// Walk a bitmapqual path tree, collecting leaf IndexPaths. Returns None on a BitmapOr
/// or an unrecognized node.
unsafe fn collect_index_paths(
    path: *mut pg_sys::Path,
    out: &mut Vec<*mut pg_sys::IndexPath>,
) -> Option<()> {
    unsafe {
        match (*path).type_ {
            pg_sys::NodeTag::T_IndexPath => {
                out.push(path.cast());
                Some(())
            }
            pg_sys::NodeTag::T_BitmapAndPath => {
                let and_path: *mut pg_sys::BitmapAndPath = path.cast();
                for sub in PgList::<pg_sys::Path>::from_pg((*and_path).bitmapquals).iter_ptr() {
                    collect_index_paths(sub, out)?;
                }
                Some(())
            }
            _ => None,
        }
    }
}

unsafe fn classify(
    bhp: *mut pg_sys::BitmapHeapPath,
    bm25_oid: pg_sys::Oid,
    heap_exprs: &[*mut pg_sys::Node],
) -> Result<Vec<HarvestedClause>, Rejection> {
    unsafe {
        let mut index_paths = Vec::new();
        collect_index_paths((*bhp).bitmapqual, &mut index_paths)
            .ok_or(Rejection::ContainsBitmapOr)?;

        let mut clauses = Vec::new();
        for ip in index_paths {
            let indexoid = (*(*ip).indexinfo).indexoid;
            if indexoid == bm25_oid {
                return Err(Rejection::UsesBm25Index);
            }
            let index_name = index_name(indexoid);
            for iclause in PgList::<pg_sys::IndexClause>::from_pg((*ip).indexclauses).iter_ptr() {
                let clause = (*(*iclause).rinfo).clause.cast::<pg_sys::Node>();
                clauses.push(HarvestedClause {
                    index_name: index_name.clone(),
                    lossy: (*iclause).lossy,
                    matched_heap_expr: matches_any(clause, heap_exprs),
                });
            }
        }

        if clauses.iter().any(|c| c.matched_heap_expr) {
            Ok(clauses)
        } else {
            Err(Rejection::NoMatchedClause)
        }
    }
}

/// Build a BitmapHeapPath over the first non-BM25 index covering our HeapExpr clauses,
/// using exported planner primitives. Handles direct matches (indexkey OP
/// pseudoconstant, operator in the index's opfamily); clauses needing support-function
/// derivation (LIKE prefix, PostGIS ST_DWithin) are not yet matched.
unsafe fn build_bitmap_path(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    bm25_oid: pg_sys::Oid,
    heap_exprs: &[*mut pg_sys::Node],
) -> Option<*mut pg_sys::BitmapHeapPath> {
    unsafe {
        for ioi in PgList::<pg_sys::IndexOptInfo>::from_pg((*rel).indexlist).iter_ptr() {
            if (*ioi).indexoid == bm25_oid || !(*ioi).amhasgetbitmap {
                continue;
            }
            // Partial indexes need predicate-implication checks.
            if !(*ioi).indpred.is_null() {
                continue;
            }

            let mut iclauses = PgList::<pg_sys::IndexClause>::new();
            for ri in PgList::<pg_sys::RestrictInfo>::from_pg((*ioi).indrestrictinfo).iter_ptr() {
                if !matches_any((*ri).clause.cast(), heap_exprs) {
                    continue;
                }
                if let Some(iclause) = try_build_index_clause(ri, ioi) {
                    iclauses.push(iclause);
                }
            }
            if iclauses.is_empty() {
                continue;
            }

            pgrx::notice!(
                "[bitmap_harvest] directly built IndexPath over {} with {} clause(s)",
                index_name((*ioi).indexoid),
                iclauses.len()
            );
            let ipath = pg_sys::create_index_path(
                root,
                ioi,
                iclauses.into_pg(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                pg_sys::ScanDirection::ForwardScanDirection,
                false,
                std::ptr::null_mut(),
                1.0,
                false,
            );
            let bhp = pg_sys::create_bitmap_heap_path(
                root,
                rel,
                ipath.cast(),
                (*rel).lateral_relids,
                1.0,
                0,
            );
            return Some(bhp);
        }
        None
    }
}

/// Match `indexkey OP pseudoconstant` against the index's key columns, mirroring the
/// direct-match arm of core's `match_opclause_to_indexcol`.
unsafe fn try_build_index_clause(
    ri: *mut pg_sys::RestrictInfo,
    ioi: *mut pg_sys::IndexOptInfo,
) -> Option<*mut pg_sys::IndexClause> {
    unsafe {
        let clause = (*ri).clause.cast::<pg_sys::Node>();
        if (*clause).type_ != pg_sys::NodeTag::T_OpExpr {
            return None;
        }
        let opexpr = clause.cast::<pg_sys::OpExpr>();
        let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
        if args.len() != 2 {
            return None;
        }
        let (mut leftop, rightop) = (args.get_ptr(0)?, args.get_ptr(1)?);
        if (*leftop).type_ == pg_sys::NodeTag::T_RelabelType {
            leftop = (*leftop.cast::<pg_sys::RelabelType>()).arg.cast();
        }
        if pg_sys::contain_var_clause(rightop) || pg_sys::contain_volatile_functions(rightop) {
            return None;
        }

        for indexcol in 0..(*ioi).nkeycolumns {
            if pg_sys::match_index_to_operand(leftop, indexcol, ioi)
                && pg_sys::op_in_opfamily((*opexpr).opno, *(*ioi).opfamily.add(indexcol as usize))
            {
                return Some(make_index_clause(ri, indexcol));
            }
        }
        None
    }
}

unsafe fn make_index_clause(
    ri: *mut pg_sys::RestrictInfo,
    indexcol: i32,
) -> *mut pg_sys::IndexClause {
    unsafe {
        let iclause: *mut pg_sys::IndexClause =
            pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexClause>()).cast();
        (*iclause).type_ = pg_sys::NodeTag::T_IndexClause;
        (*iclause).rinfo = ri;
        let mut indexquals = PgList::<pg_sys::RestrictInfo>::new();
        indexquals.push(ri);
        (*iclause).indexquals = indexquals.into_pg();
        (*iclause).lossy = false;
        (*iclause).indexcol = indexcol as _;
        (*iclause).indexcols = std::ptr::null_mut();
        iclause
    }
}

unsafe fn matches_any(clause: *mut pg_sys::Node, heap_exprs: &[*mut pg_sys::Node]) -> bool {
    unsafe {
        heap_exprs
            .iter()
            .any(|expr| pg_sys::equal(clause.cast(), (*expr).cast()))
    }
}

fn index_name(oid: pg_sys::Oid) -> String {
    PgSearchRelation::open(oid).name().to_string()
}

#[derive(Default)]
struct TidBitmapSummary {
    pages: u64,
    exact_tuples: u64,
    lossy_pages: u64,
    recheck_pages: u64,
}

impl TidBitmapSummary {
    #[cfg(feature = "pg18")]
    unsafe fn collect(tbm: *mut pg_sys::TIDBitmap) -> Self {
        unsafe {
            let mut summary = Self::default();
            let iter = pg_sys::tbm_begin_private_iterate(tbm);
            let mut res = pg_sys::TBMIterateResult::default();
            let mut offsets = [0 as pg_sys::OffsetNumber; 1024];
            while pg_sys::tbm_private_iterate(iter, &mut res) {
                summary.pages += 1;
                if res.lossy {
                    summary.lossy_pages += 1;
                } else {
                    let ntuples =
                        pg_sys::tbm_extract_page_tuple(&mut res, offsets.as_mut_ptr(), 1024);
                    summary.exact_tuples += ntuples as u64;
                }
                if res.recheck {
                    summary.recheck_pages += 1;
                }
            }
            pg_sys::tbm_end_private_iterate(iter);
            summary
        }
    }

    #[cfg(not(feature = "pg18"))]
    unsafe fn collect(tbm: *mut pg_sys::TIDBitmap) -> Self {
        unsafe {
            let mut summary = Self::default();
            let iter = pg_sys::tbm_begin_iterate(tbm);
            loop {
                let res = pg_sys::tbm_iterate(iter);
                if res.is_null() {
                    break;
                }
                summary.pages += 1;
                if (*res).ntuples < 0 {
                    summary.lossy_pages += 1;
                } else {
                    summary.exact_tuples += (*res).ntuples as u64;
                }
                if (*res).recheck {
                    summary.recheck_pages += 1;
                }
            }
            pg_sys::tbm_end_iterate(iter);
            summary
        }
    }
}
